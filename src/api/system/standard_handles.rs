use crate::{
    abi::{Handle, ERROR_INVALID_PARAMETER, ERROR_INVALID_STATE, INVALID_HANDLE},
    api::system::error,
    host::process::PROCESS,
};
pub(crate) extern "win64" fn get_std_handle(which: u32) -> Handle {
    let index = match which as i32 {
        -10 => 0,
        -11 => 1,
        -12 => 2,
        _ => {
            error::set(ERROR_INVALID_PARAMETER);
            return INVALID_HANDLE;
        }
    };
    match PROCESS.get() {
        Some(process) => process.standard_handles[index],
        None => {
            error::set(ERROR_INVALID_STATE);
            INVALID_HANDLE
        }
    }
}
