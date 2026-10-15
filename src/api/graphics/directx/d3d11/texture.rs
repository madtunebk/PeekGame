use super::super::com::{method, native_of, Com};
use std::ffi::c_void;
pub(crate) unsafe extern "win64" fn get_desc(object: *mut Com, output: *mut c_void) {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void) = unsafe { method(native, 10) };
    unsafe {
        call(native, output);
    }
}
