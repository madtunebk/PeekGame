//! OLEAUT32 BSTRs: a 32-bit byte length, then the UTF-16 text and a NUL; the
//! BSTR points at the text. VARIANTs are initialized and cleared (BSTR members
//! freed).
use std::ffi::c_void;
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}
unsafe fn allocate(bytes: u32) -> *mut u16 {
    let base = unsafe { malloc(bytes as usize + 6) }.cast::<u8>();
    if base.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        base.cast::<u32>().write_unaligned(bytes);
        base.add(4 + bytes as usize)
            .cast::<u16>()
            .write_unaligned(0);
        base.add(4).cast()
    }
}
pub unsafe extern "win64" fn alloc_string_len(text: *const u16, length: u32) -> *mut u16 {
    let s = unsafe { allocate(length * 2) };
    if !s.is_null() {
        unsafe {
            if text.is_null() {
                s.write_bytes(0, length as usize);
            } else {
                std::ptr::copy_nonoverlapping(text, s, length as usize);
            }
        }
    }
    s
}
pub unsafe extern "win64" fn alloc_string(text: *const u16) -> *mut u16 {
    if text.is_null() {
        return std::ptr::null_mut();
    }
    let n = unsafe { crate::api::text::units(text) }.len() as u32;
    unsafe { alloc_string_len(text, n) }
}
pub unsafe extern "win64" fn alloc_string_byte_len(data: *const u8, length: u32) -> *mut u16 {
    let s = unsafe { allocate(length) };
    if !s.is_null() {
        unsafe {
            let d = s.cast::<u8>();
            if data.is_null() {
                d.write_bytes(0, length as usize);
            } else {
                std::ptr::copy_nonoverlapping(data, d, length as usize);
            }
            *d.add(length as usize) = 0;
        }
    }
    s
}
pub unsafe extern "win64" fn free_string(s: *mut u16) {
    if !s.is_null() {
        unsafe { free(s.cast::<u8>().sub(4).cast()) };
    }
}
pub unsafe extern "win64" fn string_byte_len(s: *const u16) -> u32 {
    if s.is_null() {
        0
    } else {
        unsafe { s.cast::<u8>().sub(4).cast::<u32>().read_unaligned() }
    }
}
pub unsafe extern "win64" fn string_len(s: *const u16) -> u32 {
    (unsafe { string_byte_len(s) }) / 2
}
pub unsafe extern "win64" fn realloc_string_len(
    out: *mut *mut u16,
    text: *const u16,
    length: u32,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    let s = unsafe { alloc_string_len(text, length) };
    if s.is_null() {
        return 0;
    }
    unsafe {
        free_string(*out);
        *out = s;
    }
    1
}
pub unsafe extern "win64" fn realloc_string(out: *mut *mut u16, text: *const u16) -> i32 {
    let n = if text.is_null() {
        0
    } else {
        unsafe { crate::api::text::units(text) }.len() as u32
    };
    unsafe { realloc_string_len(out, text, n) }
}
/// VARIANT: vt (u16) and the value at 8.
pub unsafe extern "win64" fn variant_init(v: *mut u8) {
    if !v.is_null() {
        unsafe { v.write_bytes(0, 24) };
    }
}
pub unsafe extern "win64" fn variant_clear(v: *mut u8) -> i32 {
    if v.is_null() {
        return 0x8007_0057u32 as i32;
    }
    unsafe {
        if v.cast::<u16>().read_unaligned() == 8 {
            // VT_BSTR
            free_string(v.add(8).cast::<*mut u16>().read_unaligned());
        }
        v.write_bytes(0, 24);
    }
    0
}
