//! PE32+ reader shared by the analyzer, the relinker and the loaders. Everything a
//! game's image decides (base, sizes, sections, imports, exports, TLS, relocations)
//! is read here, so no other module parses header offsets itself.
mod directories;
mod imports;
mod reader;
#[cfg(test)]
mod tests;
use reader::{Layout, Reader};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Import {
    pub dll: String,
    pub name: Option<String>,
    pub ordinal: Option<u16>,
    pub hint: Option<u16>,
    pub iat_rva: u32,
    pub delayed: bool,
}
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub rva: u32,
    pub virtual_size: u32,
    pub raw_offset: u32,
    pub raw_size: u32,
    pub characteristics: u32,
}
impl Section {
    /// Bytes the section occupies once mapped: the larger of its virtual and raw sizes.
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
    /// Linux `PROT_*` bits for the section.
    pub fn protection(&self) -> i32 {
        self.readable() as i32 | (self.writable() as i32) << 1 | (self.executable() as i32) << 2
    }
}
#[derive(Debug, Clone)]
pub enum ExportTarget {
    Rva(u32),
    /// `DLL.Name` or `DLL.#Ordinal`, resolved by the loader.
    Forward(String),
}
#[derive(Debug, Clone)]
pub struct Export {
    pub ordinal: u32,
    pub name: Option<String>,
    pub target: ExportTarget,
}
#[derive(Debug, Clone)]
pub struct Exports {
    pub dll_name: Option<String>,
    pub ordinal_base: u32,
    pub entries: Vec<Export>,
}
/// The static TLS directory. Addresses are VAs at the preferred image base.
#[derive(Debug, Clone)]
pub struct Tls {
    pub template_start: u64,
    pub template_end: u64,
    pub index: u64,
    pub zero_fill: u32,
    pub callbacks: Vec<u64>,
}
#[derive(Debug)]
pub struct Image {
    pub machine: u16,
    pub image_base: u64,
    pub entry_rva: u32,
    pub image_size: u32,
    pub headers_size: u32,
    /// File header characteristics (`0x2000`: DLL).
    pub characteristics: u16,
    pub sections: Vec<Section>,
    /// Data directories as `(rva, size)`, as many as the image declares.
    pub directories: Vec<(u32, u32)>,
    pub imports: Vec<Import>,
    pub exports: Option<Exports>,
    pub tls: Option<Tls>,
    /// RVAs of the 64-bit fields to adjust when the image is not at its preferred base.
    pub relocations: Vec<u32>,
}
impl Image {
    pub fn is_dll(&self) -> bool {
        self.characteristics & 0x2000 != 0
    }
    pub fn directory(&self, index: usize) -> Option<(u32, u32)> {
        self.directories
            .get(index)
            .copied()
            .filter(|(rva, size)| *rva != 0 || *size != 0)
    }
    pub fn section_at(&self, rva: u32) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| rva >= s.rva && rva - s.rva < s.extent())
    }
    /// The headers as a read-only pseudo-section at RVA 0.
    pub fn headers_section(&self) -> Section {
        Section {
            name: "headers".into(),
            rva: 0,
            virtual_size: self.headers_size,
            raw_offset: 0,
            raw_size: self.headers_size,
            characteristics: 0x4000_0000,
        }
    }
    /// Checks that the image can be mapped page by page: page-aligned base and
    /// sections, sections inside the image and in order, no writable code, and (for
    /// a file of `file_len` bytes) every section's raw data present.
    pub fn check_layout(&self, file_len: Option<usize>) -> Result<()> {
        if !self.image_base.is_multiple_of(4096)
            || self.headers_size == 0
            || self.headers_size > self.image_size
        {
            return Err("invalid image base or header size".into());
        }
        let mut end = self.headers_size;
        let mut sections: Vec<_> = self.sections.iter().collect();
        sections.sort_by_key(|s| s.rva);
        for s in sections {
            if !s.rva.is_multiple_of(4096)
                || s.rva < end
                || s.rva
                    .checked_add(s.extent())
                    .is_none_or(|e| e > self.image_size)
            {
                return Err(format!(
                    "section {} has an invalid extent or alignment",
                    s.name
                ));
            }
            if s.writable() && s.executable() {
                return Err(format!("section {} is writable and executable", s.name));
            }
            if file_len.is_some_and(|n| (s.raw_offset as u64 + s.raw_size as u64) > n as u64) {
                return Err(format!("section {} is truncated", s.name));
            }
            end = s.rva + s.extent();
        }
        Ok(())
    }
}
pub(crate) type Result<T> = std::result::Result<T, String>;

/// Parses a PE file as stored on disk.
pub fn parse(bytes: &[u8]) -> Result<Image> {
    read(bytes, Layout::File)
}
/// Parses an image already laid out in memory (offsets are RVAs). Its IAT may be
/// bound, so imports are not read; everything else is.
pub fn parse_mapped(image: &[u8]) -> Result<Image> {
    read(image, Layout::Mapped)
}
/// `SizeOfImage` of the image mapped at `base`, to know how much memory
/// [`parse_mapped`] may look at.
/// # Safety
/// `base` must point to a mapped PE image whose headers are readable.
pub unsafe fn mapped_size(base: *const u8) -> usize {
    let read = |at: usize| unsafe { base.add(at).cast::<u32>().read_unaligned() } as usize;
    read(read(60) + 24 + 56)
}

/// Records `base` as the ImageBase of a mapped (and relocated) image, as the
/// Windows loader does, so code that reads its own headers sees the real base.
pub fn set_mapped_image_base(image: &mut [u8], base: u64) -> Result<()> {
    let pe = Reader {
        bytes: image,
        layout: Layout::Mapped,
        headers: 0,
        sections: Vec::new(),
    }
    .u32(60)? as usize;
    let field = image
        .get_mut(pe + 24 + 24..pe + 24 + 32)
        .ok_or("truncated PE header")?;
    field.copy_from_slice(&base.to_le_bytes());
    Ok(())
}

fn read(bytes: &[u8], layout: Layout) -> Result<Image> {
    let mut r = Reader {
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
    if machine != 0x8664 {
        return Err(format!(
            "unsupported machine {machine:#x}; prototype supports x86-64 PE only"
        ));
    }
    let count = r.u16(pe + 6)? as usize;
    let opt_len = r.u16(pe + 20)? as usize;
    let opt = pe + 24;
    r.slice(opt, opt_len)?;
    if opt_len < 112 || r.u16(opt)? != 0x20b {
        return Err("invalid PE32+ optional header".into());
    }
    if count == 0 {
        return Err("image has no sections".into());
    }
    r.headers = r.u32(opt + 60)?;
    r.slice(0, r.headers as usize)?;
    let mut sections = Vec::with_capacity(count);
    for i in 0..count {
        let at = opt + opt_len + i * 40;
        let name = r.slice(at, 8)?;
        let name = String::from_utf8_lossy(&name[..name.iter().position(|b| *b == 0).unwrap_or(8)]);
        sections.push(Section {
            name: name.into_owned(),
            virtual_size: r.u32(at + 8)?,
            rva: r.u32(at + 12)?,
            raw_size: r.u32(at + 16)?,
            raw_offset: r.u32(at + 20)?,
            characteristics: r.u32(at + 36)?,
        });
    }
    r.sections = sections.clone();
    let declared = r.u32(opt + 108)? as usize;
    if declared > (opt_len - 112) / 8 {
        return Err("data directories exceed optional header".into());
    }
    let directories = (0..declared)
        .map(|i| Ok((r.u32(opt + 112 + i * 8)?, r.u32(opt + 116 + i * 8)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut image = Image {
        machine,
        image_base: r.u64(opt + 24)?,
        entry_rva: r.u32(opt + 16)?,
        image_size: r.u32(opt + 56)?,
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
        r.imports(&mut image)?;
    }
    r.exports(&mut image)?;
    r.tls(&mut image)?;
    r.relocations(&mut image)?;
    Ok(image)
}
pub fn unique_count(image: &Image) -> usize {
    image
        .imports
        .iter()
        .map(|i| (i.dll.to_ascii_lowercase(), i.name.clone(), i.ordinal))
        .collect::<BTreeSet<_>>()
        .len()
}
