use crate::{
    abi::{Handle, ERROR_INVALID_STATE},
    api::system::{dynamic, error},
    host::process::PROCESS,
};
use std::ffi::{c_char, CStr};
fn main_image() -> Handle {
    match PROCESS.get() {
        Some(p) => p.image_base,
        None => {
            error::set(ERROR_INVALID_STATE);
            0
        }
    }
}
pub unsafe extern "win64" fn get_module_handle_a(name: *const c_char) -> Handle {
    if name.is_null() {
        return main_image();
    }
    unsafe { dynamic::load_library_a(name) }
}
pub unsafe extern "win64" fn get_module_handle_w(name: *const u16) -> Handle {
    if name.is_null() {
        return main_image();
    }
    unsafe { dynamic::load_library_w(name) }
}
// The static runner maps only the main PE plus registered compatibility modules.
pub unsafe extern "win64" fn get_module_handle_ex_w(
    flags: u32,
    name: *const u16,
    out: *mut Handle,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    unsafe {
        *out = 0;
    }
    if flags & !7 != 0 || flags & 3 == 3 {
        error::set(87);
        return 0;
    }
    if flags & 4 != 0 {
        let base = main_image();
        if base == 0 {
            return 0;
        }
        let p = name as usize;
        let size = unsafe {
            let offset = *(base.wrapping_add(60) as *const u32) as usize;
            *(base.wrapping_add(offset + 80) as *const u32)
        } as usize;
        if p < base || p >= base + size {
            error::set(126);
            return 0;
        }
        unsafe {
            *out = base;
        }
        return 1;
    }
    let handle = unsafe { get_module_handle_w(name) };
    if handle == 0 {
        return 0;
    }
    unsafe {
        *out = handle;
    }
    1
}
pub unsafe extern "win64" fn get_module_handle_ex_a(
    flags: u32,
    name: *const c_char,
    out: *mut Handle,
) -> i32 {
    if flags & 4 != 0 {
        return unsafe { get_module_handle_ex_w(flags, name.cast(), out) };
    }
    if name.is_null() {
        return unsafe { get_module_handle_ex_w(flags, std::ptr::null(), out) };
    }
    let Ok(text) = unsafe { CStr::from_ptr(name) }.to_str() else {
        error::set(87);
        return 0;
    };
    let text = text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe { get_module_handle_ex_w(flags, text.as_ptr(), out) }
}
