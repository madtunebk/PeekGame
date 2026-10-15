use crate::{
    abi::{ERROR_ALREADY_EXISTS, ERROR_INVALID_PARAMETER},
    host::{
        process::{Process, PROCESS},
        stdio,
    },
    registry,
};
use std::ffi::{c_char, c_void, CStr};
#[no_mangle]
pub extern "C" fn peekgame_abi_version() -> u32 {
    1
}
/// Configure once, before any Windows code runs. Strings are copied.
/// # Safety
/// `command_line` must point to a readable NUL-terminated string. `image_base`
/// must be the mapped main PE base. This API does not map the image itself.
#[no_mangle]
pub unsafe extern "C" fn peekgame_configure_process(
    image_base: usize,
    command_line: *const c_char,
) -> u32 {
    if image_base == 0 || command_line.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    let process = Process {
        image_base,
        command_line: unsafe { CStr::from_ptr(command_line) }.to_owned(),
        standard_handles: stdio::inherited_handles(),
    };
    match PROCESS.set(process) {
        Ok(()) => {
            // Faults in program code become Windows exceptions from here on.
            crate::seh::install_fault_handlers();
            0
        }
        Err(_) => ERROR_ALREADY_EXISTS,
    }
}
/// Resolve an implemented named import. Missing or unknown imports return NULL.
/// # Safety
/// Both arguments must be readable NUL-terminated UTF-8 strings.
/// The returned pointer MUST be called with its declared Windows x64 signature.
#[no_mangle]
pub unsafe extern "C" fn peekgame_resolve(
    dll: *const c_char,
    name: *const c_char,
) -> *const c_void {
    if dll.is_null() || name.is_null() {
        return std::ptr::null();
    }
    let (Ok(dll), Ok(name)) = (
        unsafe { CStr::from_ptr(dll) }.to_str(),
        unsafe { CStr::from_ptr(name) }.to_str(),
    ) else {
        return std::ptr::null();
    };
    registry::find(dll, name)
        .and_then(|entry| entry.address)
        .map(|get| get())
        .unwrap_or(std::ptr::null())
}

/// Start a flushed dynamic import trace through the Linux loader ABI.
/// # Safety
/// `path` must be a readable NUL-terminated filesystem path.
#[no_mangle]
pub unsafe extern "C" fn peekgame_start_import_trace(path: *const c_char) -> u32 {
    if path.is_null() {
        return ERROR_INVALID_PARAMETER;
    }
    use std::os::unix::ffi::OsStrExt;
    let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
    match crate::api::system::dynamic::start(std::path::Path::new(std::ffi::OsStr::from_bytes(
        bytes,
    ))) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("cannot start import trace: {e}");
            50
        }
    }
}
