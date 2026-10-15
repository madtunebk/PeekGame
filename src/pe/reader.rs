use super::Result;
pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) headers: u32,
    pub(super) sections: Vec<(u32, u32, u32, u32)>,
}
impl Reader<'_> {
    pub(super) fn slice(&self, at: usize, n: usize) -> Result<&[u8]> {
        let end = at.checked_add(n).ok_or("offset overflow")?;
        self.bytes
            .get(at..end)
            .ok_or_else(|| format!("truncated PE at file offset {at:#x}"))
    }
    pub(super) fn u16(&self, at: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.slice(at, 2)?.try_into().unwrap()))
    }
    pub(super) fn u32(&self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.slice(at, 4)?.try_into().unwrap()))
    }
    pub(super) fn u64(&self, at: usize) -> Result<u64> {
        Ok(u64::from_le_bytes(self.slice(at, 8)?.try_into().unwrap()))
    }
    pub(super) fn offset(&self, rva: u32, size: usize) -> Result<usize> {
        if rva < self.headers && (rva as u64 + size as u64) <= self.headers as u64 {
            self.slice(rva as usize, size)?;
            return Ok(rva as usize);
        }
        for &(va, virtual_size, raw, raw_size) in &self.sections {
            if let Some(delta) = rva.checked_sub(va) {
                if delta as u64 + size as u64 <= virtual_size.max(raw_size) as u64
                    && delta as u64 + size as u64 <= raw_size as u64
                {
                    let at = (raw as usize)
                        .checked_add(delta as usize)
                        .ok_or("offset overflow")?;
                    self.slice(at, size)?;
                    return Ok(at);
                }
            }
        }
        Err(format!(
            "RVA {rva:#x} ({size} bytes) has no file-backed section"
        ))
    }
    pub(super) fn string(&self, rva: u32) -> Result<String> {
        let mut result = Vec::new();
        for i in 0..4096u32 {
            let at = self.offset(rva.checked_add(i).ok_or("string RVA overflow")?, 1)?;
            let ch = self.bytes[at];
            if ch == 0 {
                if result.is_empty() {
                    return Err("empty import name".into());
                }
                return String::from_utf8(result).map_err(|_| "import name is not UTF-8".into());
            }
            result.push(ch);
        }
        Err("unterminated import name".into())
    }
}
