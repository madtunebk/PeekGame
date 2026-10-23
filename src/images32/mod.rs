//! 32-bit PE module manager (for 32-bit Blizzard services / PE32 executables).
//! Completely separated from the 64-bit module loader (`src/images/`).

use crate::pe32::Image32;
use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex,
    },
};

unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
    fn mprotect(a: *mut c_void, n: usize, p: i32) -> i32;
}

const MAP_PRIVATE_ANONYMOUS: i32 = 0x22;
const MAP_32BIT: i32 = 0x40; // On Linux x86_64, allocates in first 2GB of address space
const PAGE: usize = 4096;

pub struct Module32 {
    pub base: u32,
    pub size: u32,
    pub name: String,
    pub path: PathBuf,
    pub image: Image32,
    _refs: AtomicU32,
    _attached: AtomicBool,
}

impl Module32 {
    pub fn contains(&self, address: u32) -> bool {
        address >= self.base && address - self.base < self.size
    }
}

static MODULES32: Mutex<Vec<Arc<Module32>>> = Mutex::new(Vec::new());

/// Register a 32-bit executable image.
pub fn register_exe(base: u32, path: &Path, image: Image32) -> Arc<Module32> {
    let module = Arc::new(Module32 {
        base,
        size: image.image_size,
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.exe")
            .to_string(),
        path: path.to_path_buf(),
        image,
        _refs: AtomicU32::new(1),
        _attached: AtomicBool::new(true),
    });
    let mut mods = MODULES32.lock().unwrap();
    if mods.is_empty() {
        mods.push(module.clone());
    } else {
        mods[0] = module.clone();
    }
    module
}

/// Retrieve the main 32-bit executable module.
pub fn exe() -> Option<Arc<Module32>> {
    MODULES32.lock().unwrap().first().cloned()
}

/// Find a 32-bit module by handle/base.
pub fn by_handle(handle: u32) -> Option<Arc<Module32>> {
    MODULES32
        .lock()
        .unwrap()
        .iter()
        .find(|m| m.base == handle)
        .cloned()
}

/// Find a 32-bit module by address.
pub fn by_address(address: u32) -> Option<Arc<Module32>> {
    MODULES32
        .lock()
        .unwrap()
        .iter()
        .find(|m| m.contains(address))
        .cloned()
}

/// Find a 32-bit module by name.
pub fn by_name(name: &str) -> Option<Arc<Module32>> {
    let lower = name.to_ascii_lowercase();
    MODULES32
        .lock()
        .unwrap()
        .iter()
        .find(|m| m.name.to_ascii_lowercase() == lower)
        .cloned()
}

/// Get all registered 32-bit modules.
pub fn all() -> Vec<Arc<Module32>> {
    MODULES32.lock().unwrap().clone()
}

/// Map a 32-bit PE image into 32-bit address space (< 4GB).
pub struct Mapping32 {
    address: *mut u8,
    size: usize,
}

impl Mapping32 {
    pub fn new(image: &Image32, bytes: &[u8]) -> Result<Self, String> {
        let size = (image.image_size as usize).next_multiple_of(PAGE);
        // Request mapping below 2GB (MAP_32BIT)
        let raw = unsafe {
            mmap(
                image.image_base as *mut c_void,
                size,
                3, // PROT_READ | PROT_WRITE
                MAP_PRIVATE_ANONYMOUS | MAP_32BIT,
                -1,
                0,
            )
        };
        if raw as isize == -1 {
            return Err("failed to allocate 32-bit address space for PE32 image".into());
        }
        let address = raw as *mut u8;

        // Copy headers
        let headers_len = (image.headers_size as usize).min(bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), address, headers_len);
        }

        // Copy sections
        for s in &image.sections {
            if s.raw_size == 0 || s.raw_offset == 0 {
                continue;
            }
            let src_start = s.raw_offset as usize;
            let src_len = (s.raw_size as usize).min(bytes.len().saturating_sub(src_start));
            if src_len > 0 {
                unsafe {
                    let dest = address.add(s.rva as usize);
                    std::ptr::copy_nonoverlapping(bytes.as_ptr().add(src_start), dest, src_len);
                }
            }
        }

        // Apply relocations if mapped base differs from image_base
        let mapped_base = address as usize as u32;
        let delta = mapped_base.wrapping_sub(image.image_base);
        if delta != 0 {
            for &rva in &image.relocations {
                unsafe {
                    let target_ptr = address.add(rva as usize) as *mut u32;
                    let old_val = target_ptr.read_unaligned();
                    target_ptr.write_unaligned(old_val.wrapping_add(delta));
                }
            }
        }

        // Apply section page protections
        for s in &image.sections {
            let sec_size = (s.extent() as usize).next_multiple_of(PAGE);
            let prot = s.protection();
            unsafe {
                mprotect(address.add(s.rva as usize) as *mut c_void, sec_size, prot);
            }
        }

        Ok(Self { address, size })
    }

    pub fn base(&self) -> u32 {
        self.address as usize as u32
    }

    pub fn size(&self) -> u32 {
        self.size as u32
    }

    pub fn bind_iat(&self, iat_rva: u32, target_address: u32) {
        unsafe {
            let slot = self.address.add(iat_rva as usize) as *mut u32;
            slot.write(target_address);
        }
    }
}

impl Drop for Mapping32 {
    fn drop(&mut self) {
        unsafe {
            munmap(self.address as *mut c_void, self.size);
        }
    }
}
