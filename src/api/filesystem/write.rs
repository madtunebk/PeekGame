use crate::{
    abi::{Handle, WinBool, ERROR_INVALID_HANDLE, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED},
    api::system::error,
    host::stdio,
};
use std::ffi::c_void;
// Only inherited standard streams are registered in the first backend.
pub(crate) unsafe extern "win64" fn write_file(
    handle: Handle,
    data: *const c_void,
    count: u32,
    written: *mut u32,
    overlapped: *mut c_void,
) -> WinBool {
    if !written.is_null() {
        unsafe {
            written.write(0);
        }
    }
    if let Some(result) =
        unsafe { super::files::write(handle, data.cast(), count, written, overlapped.cast()) }
    {
        return result;
    }
    if !overlapped.is_null() {
        error::set(ERROR_NOT_SUPPORTED);
        return 0;
    }
    if written.is_null() || (data.is_null() && count != 0) {
        error::set(ERROR_INVALID_PARAMETER);
        return 0;
    }
    let Some(fd) = stdio::fd_for(handle) else {
        error::set(ERROR_INVALID_HANDLE);
        return 0;
    };
    // Even a zero-byte write validates the handle through the host syscall.
    let data = if data.is_null() {
        std::ptr::NonNull::<u8>::dangling().as_ptr().cast()
    } else {
        data
    };
    match unsafe { stdio::write(fd, data, count as usize) } {
        Ok(n) => {
            unsafe {
                written.write(n as u32);
            }
            1
        }
        Err(code) => {
            error::set(code);
            0
        }
    }
}
