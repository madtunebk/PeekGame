use crate::pe;
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
const MAP_PRIVATE_ANONYMOUS: i32 = 0x22;
const MAP_FIXED_NOREPLACE: i32 = 0x100000;
/// Windows never maps images in the first 64 KiB (its allocation granularity), and
/// address 0 is NULL; a preferred base there is treated as unavailable.
const LOWEST_IMAGE: usize = 0x10000;
/// IMAGE_FILE_RELOCS_STRIPPED: the image can only run at its preferred base.
const RELOCS_STRIPPED: u16 = 0x0001;
pub struct Mapping {
    address: *mut u8,
    size: usize,
}
impl Mapping {
    /// Maps `image` (parsed from `bytes`) at its preferred base, or anywhere and
    /// relocated when that range is taken and the image carries base relocations.
    pub fn new(bytes: &[u8], image: &pe::Image) -> Result<Self, String> {
        image.check_layout(Some(bytes.len()))?;
        let size = (image.image_size as usize).div_ceil(4096) * 4096;
        let preferred = image.image_base as usize;
        // MAP_FIXED_NOREPLACE never overwrites existing Linux mappings.
        let mut raw = if preferred < LOWEST_IMAGE {
            -1isize as *mut c_void
        } else {
            unsafe {
                mmap(
                    preferred as *mut c_void,
                    size,
                    3,
                    MAP_PRIVATE_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    -1,
                    0,
                )
            }
        };
        if raw as isize == -1 || raw as usize != preferred {
            if raw as isize != -1 {
                unsafe { munmap(raw, size) };
            }
            if image.characteristics & RELOCS_STRIPPED != 0 {
                return Err(format!(
                    "preferred PE base {preferred:#x} is unavailable and the image has no relocations"
                ));
            }
            raw = unsafe { mmap(std::ptr::null_mut(), size, 3, MAP_PRIVATE_ANONYMOUS, -1, 0) };
            if raw as isize == -1 {
                return Err(format!(
                    "cannot map PE image: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        let mut result = Self {
            address: raw.cast(),
            size,
        };
        let headers = image.headers_section();
        for section in std::iter::once(&headers).chain(&image.sections) {
            let data = &bytes[section.raw_offset as usize..][..section.raw_size as usize];
            let n = data.len().min(section.extent() as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    result.address.add(section.rva as usize),
                    n,
                );
            }
        }
        result.relocate(image)?;
        Ok(result)
    }
    /// Adds the load delta to every DIR64 field the image lists.
    fn relocate(&mut self, image: &pe::Image) -> Result<(), String> {
        let delta = (self.base() as u64).wrapping_sub(image.image_base);
        if delta == 0 {
            return Ok(());
        }
        for &rva in &image.relocations {
            let field = self.slot(rva, 8)?.cast::<u64>();
            unsafe { field.write_unaligned(field.read_unaligned().wrapping_add(delta)) };
        }
        let base = self.base() as u64;
        pe::set_mapped_image_base(self.memory_mut(), base)
    }
    fn memory_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.address, self.size) }
    }
    fn slot(&self, rva: u32, n: usize) -> Result<*mut u8, String> {
        if (rva as usize)
            .checked_add(n)
            .is_none_or(|end| end > self.size)
        {
            return Err(format!("RVA {rva:#x} outside image"));
        }
        Ok(unsafe { self.address.add(rva as usize) })
    }
    pub fn base(&self) -> usize {
        self.address as usize
    }
    pub fn bind(&self, rva: u32, value: usize) -> Result<(), String> {
        unsafe { self.slot(rva, 8)?.cast::<usize>().write_unaligned(value) };
        Ok(())
    }
    /// Gaps become inaccessible; headers and sections get their PE protection.
    pub fn protect(&self, image: &pe::Image) -> Result<(), String> {
        if unsafe { mprotect(self.address.cast(), self.size, 0) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe { game_imports::mapped::protect(image, self.base()) }
    }
    pub unsafe fn enter(&self, image: &pe::Image) -> Result<(), String> {
        if !image
            .section_at(image.entry_rva)
            .is_some_and(|s| s.executable())
        {
            return Err("entry is not in an executable section".into());
        }
        let entry: unsafe extern "win64" fn() =
            unsafe { std::mem::transmute(self.address.add(image.entry_rva as usize)) };
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
