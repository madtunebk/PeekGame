//! Dedicated 32-bit PE (PE32 / i386) parser.
//! Completely separated from the 64-bit PE32+ reader.

#[derive(Debug, Clone)]
pub struct Import32 {
    pub dll: String,
    pub name: Option<String>,
    pub ordinal: Option<u16>,
    pub hint: Option<u16>,
    pub iat_rva: u32,
    pub delayed: bool,
}

#[derive(Debug, Clone)]
pub struct Section32 {
    pub name: String,
    pub rva: u32,
    pub virtual_size: u32,
    pub raw_offset: u32,
    pub raw_size: u32,
    pub characteristics: u32,
}

impl Section32 {
    pub fn extent(&self) -> u32 {
        self.virtual_size.max(self.raw_size)
    }
    pub fn readable(&self) -> bool {
        self.characteristics & 0x4000_0000 != 0
    }
    pub fn writable(&self) -> bool {
        self.characteristics & 0x8000_0000 != 0
    }
    pub fn executable(&self) -> bool {
        self.characteristics & 0x2000_0000 != 0
    }
    pub fn protection(&self) -> i32 {
        self.readable() as i32 | (self.writable() as i32) << 1 | (self.executable() as i32) << 2
    }
}

#[derive(Debug, Clone)]
pub enum ExportTarget32 {
    Rva(u32),
    Forward(String),
}

#[derive(Debug, Clone)]
pub struct Export32 {
    pub ordinal: u32,
    pub name: Option<String>,
    pub target: ExportTarget32,
}

#[derive(Debug, Clone)]
pub struct Exports32 {
    pub dll_name: Option<String>,
    pub ordinal_base: u32,
    pub entries: Vec<Export32>,
}

#[derive(Debug, Clone)]
pub struct Tls32 {
    pub template_start: u32,
    pub template_end: u32,
    pub index: u32,
    pub zero_fill: u32,
    pub callbacks: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Image32 {
    pub machine: u16, // 0x14c (IMAGE_FILE_MACHINE_I386)
    pub image_base: u32,
    pub entry_rva: u32,
    pub image_size: u32,
    pub headers_size: u32,
    pub characteristics: u16,
    pub sections: Vec<Section32>,
    pub directories: Vec<(u32, u32)>,
    pub imports: Vec<Import32>,
    pub exports: Option<Exports32>,
    pub tls: Option<Tls32>,
    pub relocations: Vec<u32>,
}

impl Image32 {
    pub fn is_dll(&self) -> bool {
        self.characteristics & 0x2000 != 0
    }
    pub fn directory(&self, index: usize) -> Option<(u32, u32)> {
        self.directories
            .get(index)
            .copied()
            .filter(|(rva, size)| *rva != 0 || *size != 0)
    }
    pub fn section_at(&self, rva: u32) -> Option<&Section32> {
        self.sections
            .iter()
            .find(|s| rva >= s.rva && rva - s.rva < s.extent())
    }
    pub fn headers_section(&self) -> Section32 {
        Section32 {
            name: "headers".into(),
            rva: 0,
            virtual_size: self.headers_size,
            raw_offset: 0,
            raw_size: self.headers_size,
            characteristics: 0x4000_0000,
        }
    }
}

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    File,
    Mapped,
}

struct Reader32<'a> {
    bytes: &'a [u8],
    layout: Layout,
    headers: u32,
    sections: Vec<Section32>,
}

impl Reader32<'_> {
    fn slice(&self, at: usize, n: usize) -> Result<&[u8]> {
        let end = at.checked_add(n).ok_or("offset overflow")?;
        self.bytes
            .get(at..end)
            .ok_or_else(|| format!("truncated 32-bit PE at offset {at:#x}"))
    }
    fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.slice(at, 2)?.try_into().unwrap()))
    }
    fn u32(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.slice(at, 4)?.try_into().unwrap()))
    }
    fn offset(&self, rva: u32, size: usize) -> Result<usize> {
        if self.layout == Layout::Mapped {
            self.slice(rva as usize, size)?;
            return Ok(rva as usize);
        }
        if rva < self.headers && (rva as u64 + size as u64) <= self.headers as u64 {
            self.slice(rva as usize, size)?;
            return Ok(rva as usize);
        }
        for s in &self.sections {
            if let Some(delta) = rva.checked_sub(s.rva) {
                if delta as u64 + size as u64 <= s.extent() as u64
                    && delta as u64 + size as u64 <= s.raw_size as u64
                {
                    let at = (s.raw_offset as usize)
                        .checked_add(delta as usize)
                        .ok_or("offset overflow")?;
                    self.slice(at, size)?;
                    return Ok(at);
                }
            }
        }
        Err(format!("RVA {rva:#x} ({size} bytes) has no file-backed section in 32-bit PE"))
    }
    fn string(&self, rva: u32) -> Result<String> {
        let mut result = Vec::new();
        for i in 0u32.. {
            let b = self.slice(self.offset(rva.checked_add(i).ok_or("overflow")?, 1)?, 1)?[0];
            if b == 0 {
                return String::from_utf8(result)
                    .map_err(|_| "invalid UTF-8 in 32-bit PE string".into());
            }
            result.push(b);
        }
        unreachable!()
    }
}

pub fn parse(bytes: &[u8]) -> Result<Image32> {
    read(bytes, Layout::File)
}

pub fn parse_mapped(image: &[u8]) -> Result<Image32> {
    read(image, Layout::Mapped)
}

fn read(bytes: &[u8], layout: Layout) -> Result<Image32> {
    let mut r = Reader32 {
        bytes,
        layout,
        headers: 0,
        sections: Vec::new(),
    };
    if r.slice(0, 2)? != b"MZ" {
        return Err("not an MZ executable".into());
    }
    let pe = r.u32(60)? as usize;
    if r.slice(pe, 4)? != b"PE\0\0" {
        return Err("missing PE signature".into());
    }
    let machine = r.u16(pe + 4)?;
    if machine != 0x14c {
        return Err(format!("expected 32-bit i386 machine (0x14c), found {machine:#x}"));
    }
    let count = r.u16(pe + 6)? as usize;
    let opt_len = r.u16(pe + 20)? as usize;
    let opt = pe + 24;
    r.slice(opt, opt_len)?;
    let opt_magic = r.u16(opt)?;
    if opt_magic != 0x10b {
        return Err(format!("expected PE32 optional header (0x10b), found {opt_magic:#x}"));
    }
    if opt_len < 96 {
        return Err("invalid PE32 optional header length".into());
    }
    if count == 0 {
        return Err("32-bit PE image has no sections".into());
    }
    let image_base = r.u32(opt + 28)?;
    let entry_rva = r.u32(opt + 16)?;
    let image_size = r.u32(opt + 56)?;
    r.headers = r.u32(opt + 60)?;
    r.slice(0, r.headers as usize)?;
    let declared = r.u32(opt + 92)? as usize;

    let mut sections = Vec::with_capacity(count);
    for i in 0..count {
        let at = opt + opt_len + i * 40;
        let name = r.slice(at, 8)?;
        let name = String::from_utf8_lossy(&name[..name.iter().position(|b| *b == 0).unwrap_or(8)]);
        sections.push(Section32 {
            name: name.into_owned(),
            virtual_size: r.u32(at + 8)?,
            rva: r.u32(at + 12)?,
            raw_size: r.u32(at + 16)?,
            raw_offset: r.u32(at + 20)?,
            characteristics: r.u32(at + 36)?,
        });
    }
    r.sections = sections.clone();

    if declared > (opt_len - 96) / 8 {
        return Err("data directories exceed PE32 optional header".into());
    }
    let directories = (0..declared)
        .map(|i| Ok((r.u32(opt + 96 + i * 8)?, r.u32(opt + 96 + i * 8 + 4)?)))
        .collect::<Result<Vec<_>>>()?;

    let mut image = Image32 {
        machine,
        image_base,
        entry_rva,
        image_size,
        headers_size: r.headers,
        characteristics: r.u16(pe + 22)?,
        sections,
        directories,
        imports: Vec::new(),
        exports: None,
        tls: None,
        relocations: Vec::new(),
    };

    if layout == Layout::File {
        read_imports(&r, &mut image)?;
    }
    read_exports(&r, &mut image)?;
    read_tls(&r, &mut image)?;
    read_relocations(&r, &mut image)?;

    Ok(image)
}

fn read_imports(r: &Reader32, image: &mut Image32) -> Result<()> {
    for (directory, stride, delayed) in [(1, 20, false), (13, 32, true)] {
        let Some((start, size)) = image.directory(directory) else {
            continue;
        };
        if start == 0 || size < stride {
            return Err("invalid import directory extent in 32-bit PE".into());
        }
        let mut terminated = false;
        for offset in (0..=size - stride).step_by(stride as usize) {
            let at = r.offset(start.checked_add(offset).ok_or("descriptor overflow")?, stride as usize)?;
            if r.slice(at, stride as usize)?.iter().all(|b| *b == 0) {
                terminated = true;
                break;
            }
            let (name, lookup, iat) = if delayed {
                if r.u32(at)? != 1 {
                    return Err("unsupported legacy delay descriptor in 32-bit PE".into());
                }
                (r.u32(at + 4)?, r.u32(at + 16)?, r.u32(at + 12)?)
            } else {
                let iat = r.u32(at + 16)?;
                let lookup = r.u32(at)?;
                (r.u32(at + 12)?, if lookup == 0 { iat } else { lookup }, iat)
            };
            let dll = r.string(name)?;
            if lookup == 0 || iat == 0 {
                return Err("null import thunk table in 32-bit PE".into());
            }
            read_thunks(r, &dll, lookup, iat, delayed, &mut image.imports)?;
        }
        if !terminated {
            return Err("unterminated import descriptor table in 32-bit PE".into());
        }
    }
    Ok(())
}

fn read_thunks(
    r: &Reader32,
    dll: &str,
    lookup: u32,
    iat: u32,
    delayed: bool,
    out: &mut Vec<Import32>,
) -> Result<()> {
    for i in 0u32.. {
        let delta = i.checked_mul(4).ok_or("thunk overflow")?;
        let at = r.offset(lookup.checked_add(delta).ok_or("lookup overflow")?, 4)?;
        let thunk = r.u32(at)?;
        if thunk == 0 {
            return Ok(());
        }
        let mut import = Import32 {
            dll: dll.into(),
            name: None,
            ordinal: None,
            hint: None,
            iat_rva: iat.checked_add(delta).ok_or("IAT overflow")?,
            delayed,
        };
        r.offset(import.iat_rva, 4)?;
        if thunk & (1u32 << 31) != 0 {
            import.ordinal = Some((thunk & 0xffff) as u16);
        } else {
            let name_rva = thunk & 0x7fff_ffff;
            let at = r.offset(name_rva, 2)?;
            import.hint = Some(r.u16(at)?);
            import.name = Some(r.string(name_rva.checked_add(2).ok_or("name overflow")?)?);
        }
        out.push(import);
    }
    unreachable!()
}

fn read_exports(r: &Reader32, image: &mut Image32) -> Result<()> {
    let Some((start, size)) = image.directory(0) else {
        return Ok(());
    };
    let at = r.offset(start, 40)?;
    let name = r.u32(at + 12)?;
    let base = r.u32(at + 16)?;
    let functions = r.u32(at + 20)?;
    let names = r.u32(at + 24)?;
    let (function_table, name_table, ordinal_table) =
        (r.u32(at + 28)?, r.u32(at + 32)?, r.u32(at + 36)?);
    let mut entries = Vec::new();
    for index in 0..functions {
        let rva = r.u32(r.offset(function_table.checked_add(index.checked_mul(4).ok_or("overflow")?).ok_or("overflow")?, 4)?)?;
        if rva == 0 {
            continue;
        }
        let target = if rva >= start && rva - start < size {
            ExportTarget32::Forward(r.string(rva)?)
        } else {
            ExportTarget32::Rva(rva)
        };
        entries.push(Export32 {
            ordinal: base.checked_add(index).ok_or("export ordinal overflow")?,
            name: None,
            target,
        });
    }
    for index in 0..names {
        let slot = r.u16(r.offset(ordinal_table.checked_add(index.checked_mul(2).ok_or("overflow")?).ok_or("overflow")?, 2)?)? as u32;
        let name_str = r.string(r.u32(r.offset(name_table.checked_add(index.checked_mul(4).ok_or("overflow")?).ok_or("overflow")?, 4)?)?)?;
        let ordinal = base.checked_add(slot).ok_or("export ordinal overflow")?;
        if let Some(entry) = entries.iter_mut().find(|e| e.ordinal == ordinal) {
            entry.name = Some(name_str);
        }
    }
    image.exports = Some(Exports32 {
        dll_name: if name == 0 { None } else { Some(r.string(name)?) },
        ordinal_base: base,
        entries,
    });
    Ok(())
}

fn read_tls(r: &Reader32, image: &mut Image32) -> Result<()> {
    let Some((start, _)) = image.directory(9) else {
        return Ok(());
    };
    let at = r.offset(start, 24)?;
    let template_start = r.u32(at)?;
    let template_end = r.u32(at + 4)?;
    let index = r.u32(at + 8)?;
    let array = r.u32(at + 12)?;
    let zero_fill = r.u32(at + 16)?;
    let mut tls = Tls32 {
        template_start,
        template_end,
        index,
        zero_fill,
        callbacks: Vec::new(),
    };
    if tls.template_end < tls.template_start {
        return Err("invalid 32-bit TLS template range".into());
    }
    if array != 0 {
        let mut rva = array.checked_sub(image.image_base).ok_or("TLS array below image base")?;
        loop {
            let callback = r.u32(r.offset(rva, 4)?)?;
            if callback == 0 {
                break;
            }
            tls.callbacks.push(callback);
            rva = rva.checked_add(4).ok_or("TLS callback array overflow")?;
        }
    }
    image.tls = Some(tls);
    Ok(())
}

fn read_relocations(r: &Reader32, image: &mut Image32) -> Result<()> {
    let Some((start, size)) = image.directory(5) else {
        return Ok(());
    };
    let mut block = 0u32;
    while block + 8 <= size {
        let at = r.offset(start + block, 8)?;
        let (page, length) = (r.u32(at)?, r.u32(at + 4)?);
        if length < 8 || length % 2 != 0 || block.checked_add(length).is_none_or(|end| end > size) {
            return Err("invalid 32-bit base relocation block".into());
        }
        for entry in (8..length).step_by(2) {
            let value = r.u16(r.offset(start + block + entry, 2)?)?;
            match value >> 12 {
                0 => {} // ABSOLUTE
                3 => image.relocations.push(
                    page.checked_add((value & 0xfff) as u32).ok_or("relocation overflow")?,
                ),
                kind => return Err(format!("unsupported 32-bit base relocation type {kind}")),
            }
        }
        block += length;
    }
    Ok(())
}
