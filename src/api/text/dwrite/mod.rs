//! DirectWrite system-font discovery backed by the local fontconfig adapter.
//! All COM vtables in the adapter contain Windows ABI functions.
use std::ffi::{c_char, c_void};
unsafe extern "win64" {
    fn peekgame_dwrite_create_factory(kind: u32, iid: *const c_void, out: *mut *mut c_void) -> i32;
}
pub unsafe extern "win64" fn create_factory(
    kind: u32,
    iid: *const c_void,
    out: *mut *mut c_void,
) -> i32 {
    unsafe { peekgame_dwrite_create_factory(kind, iid, out) }
}
#[no_mangle]
unsafe extern "win64" fn peekgame_dwrite_multi(
    cp: u32,
    flags: u32,
    input: *const c_char,
    len: i32,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    unsafe { crate::api::system::encoding::multi_to_wide(cp, flags, input, len, out, capacity) }
}
#[no_mangle]
unsafe extern "win64" fn peekgame_dwrite_wide(
    cp: u32,
    flags: u32,
    input: *const u16,
    len: i32,
    out: *mut u8,
    capacity: i32,
    default: *const u8,
    used: *mut i32,
) -> i32 {
    unsafe {
        crate::api::system::encoding::wide_to_multi(
            cp, flags, input, len, out, capacity, default, used,
        )
    }
}
