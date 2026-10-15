use super::{reader::Reader, Image, Import, Result};
impl Reader<'_> {
    /// Normal (directory 1) and delay-load (directory 13) import tables.
    pub(super) fn imports(&self, image: &mut Image) -> Result<()> {
        for (directory, stride, delayed) in [(1, 20, false), (13, 32, true)] {
            let Some((start, size)) = image.directory(directory) else {
                continue;
            };
            if start == 0 || size < stride {
                return Err("invalid import directory extent".into());
            }
            let mut terminated = false;
            for offset in (0..=size - stride).step_by(stride as usize) {
                let at = self.offset(
                    start.checked_add(offset).ok_or("descriptor overflow")?,
                    stride as usize,
                )?;
                if self.slice(at, stride as usize)?.iter().all(|b| *b == 0) {
                    terminated = true;
                    break;
                }
                let (name, lookup, iat) = if delayed {
                    if self.u32(at)? != 1 {
                        return Err(
                            "unsupported legacy delay descriptor: expected RVA-based fields".into(),
                        );
                    }
                    (self.u32(at + 4)?, self.u32(at + 16)?, self.u32(at + 12)?)
                } else {
                    let iat = self.u32(at + 16)?;
                    let lookup = self.u32(at)?;
                    (
                        self.u32(at + 12)?,
                        if lookup == 0 { iat } else { lookup },
                        iat,
                    )
                };
                let dll = self.string(name)?;
                if lookup == 0 || iat == 0 {
                    return Err("null import thunk table".into());
                }
                self.thunks(&dll, lookup, iat, delayed, &mut image.imports)?;
            }
            if !terminated {
                return Err("unterminated import descriptor table".into());
            }
        }
        Ok(())
    }
    /// One DLL's thunks, up to the zero terminator (the table cannot outrun its section).
    fn thunks(
        &self,
        dll: &str,
        lookup: u32,
        iat: u32,
        delayed: bool,
        out: &mut Vec<Import>,
    ) -> Result<()> {
        for i in 0u32.. {
            let delta = i.checked_mul(8).ok_or("thunk overflow")?;
            let at = self.offset(lookup.checked_add(delta).ok_or("lookup overflow")?, 8)?;
            let thunk = self.u64(at)?;
            if thunk == 0 {
                return Ok(());
            }
            let mut import = Import {
                dll: dll.into(),
                name: None,
                ordinal: None,
                hint: None,
                iat_rva: iat.checked_add(delta).ok_or("IAT overflow")?,
                delayed,
            };
            self.offset(import.iat_rva, 8)?;
            if thunk & (1 << 63) != 0 {
                import.ordinal = Some(thunk as u16);
            } else {
                let rva = u32::try_from(thunk).map_err(|_| {
                    "invalid import-name RVA (possibly a bound IAT without lookup table)"
                })?;
                import.hint = Some(self.u16(self.offset(rva, 2)?)?);
                import.name = Some(self.string(rva.checked_add(2).ok_or("name overflow")?)?);
            }
            out.push(import);
        }
        unreachable!()
    }
}
