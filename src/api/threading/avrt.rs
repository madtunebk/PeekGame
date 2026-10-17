//! AVRT/MMCSS scheduling has no backend here; return real failure, no fake task handles.
use crate::api::system::error;
use std::ffi::c_void;
pub extern "win64" fn characteristics(task: *const c_void, index: *mut u32) -> usize {
    error::set(if task.is_null() || index.is_null() {
        87
    } else {
        50
    });
    0
}
pub extern "win64" fn revert(_handle: usize) -> i32 {
    error::set(6);
    0
}
pub extern "win64" fn priority(_handle: usize, _priority: i32) -> i32 {
    error::set(6);
    0
}
