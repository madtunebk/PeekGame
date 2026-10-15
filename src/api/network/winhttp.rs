//! winhttp.dll proxy queries: no proxy is configured (direct connections). HTTP
//! requests themselves go through the program's own stack (Unity uses curl).
use crate::api::system::error;
/// WINHTTP_CURRENT_USER_IE_PROXY_CONFIG: fAutoDetect, then three string pointers;
/// all empty.
pub unsafe extern "win64" fn ie_proxy_config(out: *mut u8) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { out.write_bytes(0, 32) };
    1
}
pub extern "win64" fn open(
    _agent: usize,
    _access: u32,
    _proxy: usize,
    _bypass: usize,
    _flags: u32,
) -> usize {
    0x000f_0010
}
pub extern "win64" fn close_handle(_handle: usize) -> i32 {
    1
}
/// WinHttpGetProxyForUrl: ERROR_WINHTTP_AUTODETECTION_FAILED (no proxy script).
pub extern "win64" fn proxy_for_url(
    _session: usize,
    _url: usize,
    _options: usize,
    _info: usize,
) -> i32 {
    error::set(12180);
    0
}
