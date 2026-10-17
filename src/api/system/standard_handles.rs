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
        Some(process) => process.standard_handles[index].load(std::sync::atomic::Ordering::SeqCst),
        None => {
            error::set(ERROR_INVALID_STATE);
            INVALID_HANDLE
        }
    }
}

/// Windows accepts arbitrary handle values here; subsequent I/O validates them.
pub(crate) extern "win64" fn set_std_handle(which: u32, handle: Handle) -> i32 {
    let index = match which as i32 {
        -10 => 0,
        -11 => 1,
        -12 => 2,
        _ => {
            error::set(ERROR_INVALID_PARAMETER);
            return 0;
        }
    };
    let Some(process) = PROCESS.get() else {
        error::set(ERROR_INVALID_STATE);
        return 0;
    };
    process.standard_handles[index].store(handle, std::sync::atomic::Ordering::SeqCst);
    1
}
