use std::ffi::c_void;
unsafe extern "C" {
    fn mmap(
        address: *mut c_void,
        size: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn mprotect(address: *mut c_void, size: usize, prot: i32) -> i32;
    fn munmap(address: *mut c_void, size: usize) -> i32;
}
struct Section {
    start: usize,
    size: usize,
    protection: i32,
}
pub struct Mapping {
    address: *mut u8,
    size: usize,
    headers: usize,
    sections: Vec<Section>,
}
fn u32_at(b: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        b.get(at..at + 4)
            .ok_or("truncated PE header")?
            .try_into()
            .unwrap(),
    ))
}
impl Mapping {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        let pe = u32_at(bytes, 60)? as usize;
        let opt = pe + 24;
        let count = u16::from_le_bytes(bytes[pe + 6..pe + 8].try_into().unwrap()) as usize;
        let opt_size = u16::from_le_bytes(bytes[pe + 20..pe + 22].try_into().unwrap()) as usize;
        let preferred = u64::from_le_bytes(bytes[opt + 24..opt + 32].try_into().unwrap()) as usize;
        let size = u32_at(bytes, opt + 56)? as usize;
        let headers = u32_at(bytes, opt + 60)? as usize;
        if size == 0
            || size > 1024 * 1024 * 1024
            || !preferred.is_multiple_of(4096)
            || headers > size
        {
            return Err("invalid image extent/base".into());
        }
        // MAP_FIXED_NOREPLACE never overwrites existing Linux mappings.
        let raw = unsafe { mmap(preferred as *mut c_void, size, 3, 0x100022, -1, 0) };
        if raw as isize == -1 {
            return Err(format!(
                "cannot reserve preferred PE base: {}",
                std::io::Error::last_os_error()
            ));
        }
        if raw as usize != preferred {
            unsafe {
                munmap(raw, size);
            }
            return Err("kernel did not honor preferred PE base".into());
        }
        let mut result = Self {
            address: raw.cast(),
            size,
            headers,
            sections: Vec::new(),
        };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), result.address, headers);
        }
        for index in 0..count {
            let at = opt + opt_size + index * 40;
            let start = u32_at(bytes, at + 12)? as usize;
            let raw_size = u32_at(bytes, at + 16)? as usize;
            let file = u32_at(bytes, at + 20)? as usize;
            let extent = (u32_at(bytes, at + 8)? as usize).max(raw_size);
            if !start.is_multiple_of(4096) || start.checked_add(extent).is_none_or(|end| end > size)
            {
                return Err("invalid section extent/alignment".into());
            }
            let data = bytes
                .get(file..file.checked_add(raw_size).ok_or("section overflow")?)
                .ok_or("truncated section")?;
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), result.address.add(start), raw_size);
            }
            let flags = u32_at(bytes, at + 36)?;
            let protection = ((flags & 0x40000000 != 0) as i32)
                | (((flags & 0x80000000 != 0) as i32) << 1)
                | (((flags & 0x20000000 != 0) as i32) << 2);
            if protection & 6 == 6 {
                return Err("writable executable PE sections are unsupported".into());
            }
            result.sections.push(Section {
                start,
                size: extent,
                protection,
            });
        }
        Ok(result)
    }
    pub fn read(&self, address: usize, n: usize) -> Result<&[u8], String> {
        let offset = address.checked_sub(self.base()).ok_or("VA below image")?;
        if offset.checked_add(n).is_none_or(|end| end > self.size) {
            return Err("VA outside image".into());
        }
        Ok(unsafe { std::slice::from_raw_parts(self.address.add(offset), n) })
    }
    pub fn write_u32(&self, address: usize, value: u32) -> Result<(), String> {
        self.read(address, 4)?;
        unsafe {
            (address as *mut u32).write_unaligned(value);
        }
        Ok(())
    }
    pub fn executable(&self, address: usize) -> bool {
        address.checked_sub(self.base()).is_some_and(|offset| {
            self.sections
                .iter()
                .any(|s| s.protection & 4 != 0 && offset >= s.start && offset < s.start + s.size)
        })
    }
    pub fn base(&self) -> usize {
        self.address as usize
    }
    pub fn bind(&self, rva: u32, value: usize) -> Result<(), String> {
        let at = rva as usize;
        if at.checked_add(8).is_none_or(|end| end > self.size) {
            return Err("IAT slot outside image".into());
        }
        unsafe {
            self.address.add(at).cast::<usize>().write_unaligned(value);
        }
        Ok(())
    }
    pub fn protect(&self) -> Result<(), String> {
        let apply = |start, size, prot| {
            if size > 0
                && unsafe { mprotect(self.address.wrapping_add(start).cast(), size, prot) } != 0
            {
                Err(std::io::Error::last_os_error().to_string())
            } else {
                Ok(())
            }
        };
        apply(0, self.size, 0)?;
        apply(0, self.headers, 1)?;
        for section in &self.sections {
            apply(section.start, section.size, section.protection)?;
        }
        Ok(())
    }
    pub unsafe fn enter(&self, rva: u32) -> Result<(), String> {
        if !self.sections.iter().any(|s| {
            s.protection & 4 != 0 && rva as usize >= s.start && (rva as usize) < s.start + s.size
        }) {
            return Err("entry is not in an executable section".into());
        }
        let entry: unsafe extern "win64" fn() =
            unsafe { std::mem::transmute(self.address.add(rva as usize)) };
        unsafe {
            entry();
        }
        Ok(())
    }
}
impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            munmap(self.address.cast(), self.size);
        }
    }
}
