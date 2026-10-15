use super::{reader::Reader, Import, Result};
impl Reader<'_> {
    pub(super) fn thunks(
        &self,
        dll: &str,
        lookup: u32,
        iat: u32,
        delayed: bool,
        out: &mut Vec<Import>,
    ) -> Result<()> {
        for i in 0..1_000_000u32 {
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
        Err("too many import thunks".into())
    }
}
