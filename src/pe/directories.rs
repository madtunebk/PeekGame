use super::{reader::Reader, Export, ExportTarget, Exports, Image, Result, Tls};
const EXPORT: usize = 0;
const RELOCATION: usize = 5;
const TLS: usize = 9;
impl Reader<'_> {
    pub(super) fn exports(&self, image: &mut Image) -> Result<()> {
        let Some((start, size)) = image.directory(EXPORT) else {
            return Ok(());
        };
        let at = self.offset(start, 40)?;
        let name = self.u32(at + 12)?;
        let base = self.u32(at + 16)?;
        let functions = self.u32(at + 20)?;
        let names = self.u32(at + 24)?;
        let (function_table, name_table, ordinal_table) =
            (self.u32(at + 28)?, self.u32(at + 32)?, self.u32(at + 36)?);
        let mut entries = Vec::new();
        for index in 0..functions {
            let rva = self.u32(self.offset(table(function_table, index, 4)?, 4)?)?;
            if rva == 0 {
                continue;
            }
            // An RVA inside the export directory names another DLL's export instead.
            let target = if rva >= start && rva - start < size {
                ExportTarget::Forward(self.string(rva)?)
            } else {
                ExportTarget::Rva(rva)
            };
            entries.push(Export {
                ordinal: base.checked_add(index).ok_or("export ordinal overflow")?,
                name: None,
                target,
            });
        }
        for index in 0..names {
            let slot = self.u16(self.offset(table(ordinal_table, index, 2)?, 2)?)? as u32;
            let name = self.string(self.u32(self.offset(table(name_table, index, 4)?, 4)?)?)?;
            let ordinal = base.checked_add(slot).ok_or("export ordinal overflow")?;
            let entry = entries
                .iter_mut()
                .find(|e| e.ordinal == ordinal)
                .ok_or_else(|| format!("export {name} names an empty function slot"))?;
            entry.name = Some(name);
        }
        image.exports = Some(Exports {
            dll_name: if name == 0 {
                None
            } else {
                Some(self.string(name)?)
            },
            ordinal_base: base,
            entries,
        });
        Ok(())
    }
    pub(super) fn tls(&self, image: &mut Image) -> Result<()> {
        let Some((start, _)) = image.directory(TLS) else {
            return Ok(());
        };
        let at = self.offset(start, 40)?;
        let mut tls = Tls {
            template_start: self.u64(at)?,
            template_end: self.u64(at + 8)?,
            index: self.u64(at + 16)?,
            zero_fill: self.u32(at + 32)?,
            callbacks: Vec::new(),
        };
        if tls.template_end < tls.template_start {
            return Err("invalid TLS template range".into());
        }
        let array = self.u64(at + 24)?;
        if array != 0 {
            // A zero-terminated array of VAs; it cannot run past its section.
            let mut rva = rva_of(image, array)?;
            loop {
                let callback = self.u64(self.offset(rva, 8)?)?;
                if callback == 0 {
                    break;
                }
                rva_of(image, callback)?;
                tls.callbacks.push(callback);
                rva = rva.checked_add(8).ok_or("TLS callback array overflow")?;
            }
        }
        image.tls = Some(tls);
        Ok(())
    }
    pub(super) fn relocations(&self, image: &mut Image) -> Result<()> {
        let Some((start, size)) = image.directory(RELOCATION) else {
            return Ok(());
        };
        let mut block = 0u32;
        while block + 8 <= size {
            let at = self.offset(start + block, 8)?;
            let (page, length) = (self.u32(at)?, self.u32(at + 4)?);
            if length < 8
                || length % 2 != 0
                || block.checked_add(length).is_none_or(|end| end > size)
            {
                return Err("invalid base relocation block".into());
            }
            for entry in (8..length).step_by(2) {
                let value = self.u16(self.offset(start + block + entry, 2)?)?;
                match value >> 12 {
                    0 => {}
                    10 => image.relocations.push(
                        page.checked_add((value & 0xfff) as u32)
                            .ok_or("relocation overflow")?,
                    ),
                    kind => return Err(format!("unsupported base relocation type {kind}")),
                }
            }
            block += length;
        }
        Ok(())
    }
}
fn table(start: u32, index: u32, width: u32) -> Result<u32> {
    index
        .checked_mul(width)
        .and_then(|delta| start.checked_add(delta))
        .ok_or_else(|| "export table overflow".into())
}
/// RVA of a VA the image stores for its preferred base.
fn rva_of(image: &Image, va: u64) -> Result<u32> {
    va.checked_sub(image.image_base)
        .filter(|rva| *rva < image.image_size as u64)
        .map(|rva| rva as u32)
        .ok_or_else(|| format!("address {va:#x} outside image"))
}
