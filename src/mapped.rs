//! Work on a PE image already mapped at `base`, shared by the PE runner and relinked
//! executables. Every size and address comes from the parsed image; VAs the image
//! stores for its preferred base are moved by `base - image_base`.
use crate::pe;

/// Initial TLS block (template plus zero fill) and TLS callback addresses.
pub struct Tls {
    pub data: Vec<u8>,
    pub callbacks: Vec<usize>,
}

fn rebase(image: &pe::Image, base: usize, va: u64) -> Result<usize, String> {
    let rva = va
        .checked_sub(image.image_base)
        .filter(|rva| *rva < image.image_size as u64)
        .ok_or_else(|| format!("address {va:#x} outside image"))?;
    Ok(base + rva as usize)
}

/// Reads the TLS template and callbacks and stores 0, the main image's TLS index,
/// in the image's index variable.
/// # Safety
/// `image` must describe the image mapped at `base`, with its sections readable
/// and the index variable writable.
pub unsafe fn tls(image: &pe::Image, base: usize) -> Result<Tls, String> {
    let Some(tls) = &image.tls else {
        return Ok(Tls {
            data: Vec::new(),
            callbacks: Vec::new(),
        });
    };
    let raw = (tls.template_end - tls.template_start) as usize;
    let mut data = vec![0; (raw + tls.zero_fill as usize).max(1)];
    if raw > 0 {
        let start = rebase(image, base, tls.template_start)?;
        rebase(image, base, tls.template_end - 1)?;
        data[..raw].copy_from_slice(unsafe { std::slice::from_raw_parts(start as *const u8, raw) });
    }
    let index = rebase(image, base, tls.index)?;
    rebase(image, base, tls.index + 3)?;
    unsafe { (index as *mut u32).write_unaligned(0) };
    let callbacks = tls
        .callbacks
        .iter()
        .map(|&va| rebase(image, base, va))
        .collect::<Result<_, _>>()?;
    Ok(Tls { data, callbacks })
}

/// Calls each TLS callback with `reason` (1: process attach), in image order.
pub fn notify(base: usize, callbacks: &[usize], reason: u32) {
    for &address in callbacks {
        let callback: extern "win64" fn(usize, u32, usize) =
            unsafe { std::mem::transmute(address) };
        callback(base, reason, 0);
    }
}

/// Gives the headers and every section the protection the PE declares.
/// # Safety
/// `image` must describe the image mapped at `base`, whose pages this call owns.
pub unsafe fn protect(image: &pe::Image, base: usize) -> Result<(), String> {
    unsafe extern "C" {
        fn mprotect(address: *mut std::ffi::c_void, size: usize, prot: i32) -> i32;
    }
    for section in std::iter::once(image.headers_section()).chain(image.sections.iter().cloned()) {
        let size = (section.extent() as usize).div_ceil(4096) * 4096;
        let start = base + section.rva as usize;
        if size > 0 && unsafe { mprotect(start as *mut _, size, section.protection()) } != 0 {
            return Err(format!(
                "mprotect {}: {}",
                section.name,
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}
