use crate::{abi::ERROR_INVALID_STATE, api::system::error, host::process::PROCESS};
use std::ffi::c_char;
pub(crate) extern "win64" fn get_command_line_a() -> *mut c_char {
    match PROCESS.get() {
        Some(process) => process.command_line.as_ptr().cast_mut(),
        None => {
            error::set(ERROR_INVALID_STATE);
            std::ptr::null_mut()
        }
    }
}
