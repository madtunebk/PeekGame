//! Registry/filesystem transactions have no implementation. Report failure to
//! callers (including Discord's launch-command registration), never a fake handle.
use super::error;
use std::ffi::c_void;
pub extern "win64" fn create(
    _security: *const c_void,
    _unit_of_work: *const u8,
    options: u32,
    isolation: u32,
    isolation_flags: u32,
    _timeout: u32,
    _description: *const u16,
) -> usize {
    error::set(
        if options & !1 != 0 || isolation != 0 || isolation_flags != 0 {
            87
        } else {
            50
        },
    );
    usize::MAX
}
pub extern "win64" fn commit(_transaction: usize) -> i32 {
    error::set(6); // No valid transaction handles can be created by this backend.
    0
}
pub unsafe extern "win64" fn create_key(
    _key: usize,
    _name: *const u16,
    _reserved: u32,
    _class: *mut u16,
    _options: u32,
    _access: u32,
    _security: *const c_void,
    out: *mut usize,
    _disposition: *mut u32,
    _transaction: usize,
    _extended: *const c_void,
) -> i32 {
    if out.is_null() {
        return 87;
    }
    unsafe {
        *out = 0;
    }
    50
}
