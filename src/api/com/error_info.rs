//! OLEAUT32 error info management (SetErrorInfo, GetErrorInfo).
use std::cell::Cell;
use std::ffi::c_void;

thread_local! {
    static ERROR_INFO: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
}

pub extern "win64" fn set_error_info(_reserved: u32, err: *mut c_void) -> i32 {
    ERROR_INFO.with(|c| c.set(err));
    0
}

pub unsafe extern "win64" fn get_error_info(_reserved: u32, out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return 0x80070057u32 as i32;
    }
    ERROR_INFO.with(|c| {
        let prev = c.replace(std::ptr::null_mut());
        unsafe { *out = prev };
        if prev.is_null() {
            1 // S_FALSE
        } else {
            0 // S_OK
        }
    })
}
