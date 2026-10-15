mod imports;
mod reader;
#[cfg(test)]
mod tests;
use reader::Reader;
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
#[derive(Debug)]
pub struct Image {
    pub machine: u16,
    pub image_base: u64,
    pub entry_rva: u32,
    pub imports: Vec<Import>,
}
type Result<T> = std::result::Result<T, String>;
pub fn parse(bytes: &[u8]) -> Result<Image> {
    let mut r = Reader {
        bytes,
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
    if count == 0 || count > 96 {
        return Err("invalid section count".into());
    }
    r.headers = r.u32(opt + 60)?;
    r.slice(0, r.headers as usize)?;
    for i in 0..count {
        let section = opt + opt_len + i * 40;
        r.slice(section, 40)?;
        r.sections.push((
            r.u32(section + 12)?,
            r.u32(section + 8)?,
            r.u32(section + 20)?,
            r.u32(section + 16)?,
        ));
    }
    let directories = r.u32(opt + 108)? as usize;
    if directories > (opt_len - 112) / 8 {
        return Err("data directories exceed optional header".into());
    }
    let mut image = Image {
        machine,
        image_base: r.u64(opt + 24)?,
        entry_rva: r.u32(opt + 16)?,
        imports: Vec::new(),
    };
    for (directory, stride, delayed) in [(1, 20, false), (13, 32, true)] {
        if directory >= directories {
            continue;
        }
        let start = r.u32(opt + 112 + directory * 8)?;
        let size = r.u32(opt + 116 + directory * 8)?;
        if start == 0 && size == 0 {
            continue;
        }
        if start == 0 || size < stride {
            return Err("invalid import directory extent".into());
        }
        let mut terminated = false;
        for offset in (0..=size - stride).step_by(stride as usize) {
            let at = r.offset(
                start.checked_add(offset).ok_or("descriptor overflow")?,
                stride as usize,
            )?;
            if r.slice(at, stride as usize)?.iter().all(|b| *b == 0) {
                terminated = true;
                break;
            }
            let (name, lookup, iat) = if delayed {
                if r.u32(at)? != 1 {
                    return Err(
                        "unsupported legacy delay descriptor: expected RVA-based fields".into(),
                    );
                }
                (r.u32(at + 4)?, r.u32(at + 16)?, r.u32(at + 12)?)
            } else {
                let iat = r.u32(at + 16)?;
                let lookup = r.u32(at)?;
                (r.u32(at + 12)?, if lookup == 0 { iat } else { lookup }, iat)
            };
            let dll = r.string(name)?;
            if lookup == 0 || iat == 0 {
                return Err("null import thunk table".into());
            }
            r.thunks(&dll, lookup, iat, delayed, &mut image.imports)?;
        }
        if !terminated {
            return Err("unterminated import descriptor table".into());
        }
    }
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
