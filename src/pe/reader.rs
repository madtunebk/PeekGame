use super::{Result, Section};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Layout {
    /// Bytes of the file on disk: RVAs go through the section table.
    File,
    /// A mapped image: an RVA is the offset.
    Mapped,
}
pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) layout: Layout,
    pub(super) headers: u32,
    pub(super) sections: Vec<Section>,
}
impl Reader<'_> {
    pub(super) fn slice(&self, at: usize, n: usize) -> Result<&[u8]> {
        let end = at.checked_add(n).ok_or("offset overflow")?;
        self.bytes
            .get(at..end)
            .ok_or_else(|| format!("truncated PE at offset {at:#x}"))
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
    /// Offset in `bytes` of `size` bytes at `rva`.
    pub(super) fn offset(&self, rva: u32, size: usize) -> Result<usize> {
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
        Err(format!(
            "RVA {rva:#x} ({size} bytes) has no file-backed section"
        ))
    }
    /// The NUL-terminated string at `rva`; it may run to the end of its section.
    pub(super) fn string(&self, rva: u32) -> Result<String> {
        let mut result = Vec::new();
        let mut at = rva;
        loop {
            let ch = self.bytes[self.offset(at, 1)?];
            if ch == 0 {
                if result.is_empty() {
                    return Err(format!("empty name at RVA {rva:#x}"));
                }
                return String::from_utf8(result).map_err(|_| "name is not UTF-8".into());
            }
            result.push(ch);
            at = at.checked_add(1).ok_or("string RVA overflow")?;
        }
    }
}
