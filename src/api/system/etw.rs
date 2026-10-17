//! Event Tracing for Windows has no Linux counterpart (ported from peek's etw.inc):
//! EventRegister fails with ERROR_NOT_SUPPORTED and a null REGHANDLE, and calls with
//! such a handle answer as Windows does for a provider that is not registered.
pub unsafe extern "win64" fn register(
    _provider: usize,
    _callback: usize,
    _context: usize,
    handle: *mut u64,
) -> u32 {
    if !handle.is_null() {
        unsafe { *handle = 0 };
    }
    50
}
pub extern "win64" fn unregister(handle: u64) -> u32 {
    if handle != 0 {
        6
    } else {
        0
    }
}
pub extern "win64" fn write(_handle: u64, _descriptor: usize, _count: u32, _data: usize) -> u32 {
    6
}
pub extern "win64" fn write_transfer(
    _handle: u64,
    _descriptor: usize,
    _activity: usize,
    _related: usize,
    _count: u32,
    _data: usize,
) -> u32 {
    6
}
pub extern "win64" fn set_information(_handle: u64, _kind: i32, _data: usize, _size: u32) -> u32 {
    6
}
pub extern "win64" fn enabled(_handle: u64, _descriptor: usize) -> i32 {
    0
}
pub extern "win64" fn provider_enabled(_handle: u64, _level: u8, _keyword: u64) -> i32 {
    0
}
/// Classic ETW provider registration. No tracing session is available, so
/// report the failure and never hand Windows code a fabricated trace handle.
pub unsafe extern "win64" fn register_trace_guids_w(
    _request: usize, _context: usize, _control_guid: usize, _guid_count: u32,
    _registrations: usize, _image_path: usize, _resource_name: usize,
    registration_handle: *mut u64,
) -> u32 {
    if registration_handle.is_null() { return 87; }
    unsafe { *registration_handle = 0 };
    50 // ERROR_NOT_SUPPORTED
}
pub extern "win64" fn unregister_trace_guids(_handle: u64) -> u32 { 6 }
pub extern "win64" fn get_trace_logger_handle(_buffer: usize) -> u64 {
    crate::api::system::error::set(6);
    u64::MAX
}
pub extern "win64" fn get_trace_enable_level(_handle: u64) -> u8 {
    crate::api::system::error::set(6);
    0
}
pub extern "win64" fn get_trace_enable_flags(_handle: u64) -> u32 {
    crate::api::system::error::set(6);
    0
}
/// TraceMessage is variadic on Windows; only its four fixed arguments are
/// inspected here. No varargs are read because registration never succeeds.
pub extern "win64" fn trace_message(
    _handle: u64, _flags: u32, _guid: usize, _number: u16,
) -> u32 { 6 }

/// AreFileApisANSI: the ANSI code page is UTF-8 here, as GetACP reports.
pub extern "win64" fn file_apis_ansi() -> i32 {
    1
}
