//! WinHTTP transport backed by host libcurl. GET, HEAD and inline-body POST
//! are supported with TLS peer and hostname verification. No Windows pointers
//! escape to libcurl.
use std::{ffi::{c_char, c_long, c_void, CString}, sync::OnceLock};

const CURLOPT_URL: u32 = 10002;
const CURLOPT_PROXY: u32 = 10004;
const CURLOPT_HTTPHEADER: u32 = 10023;
const CURLOPT_WRITEDATA: u32 = 10001;
const CURLOPT_HEADERDATA: u32 = 10029;
const CURLOPT_HEADERFUNCTION: u32 = 20079;
const CURLOPT_WRITEFUNCTION: u32 = 20011;
const CURLOPT_NOBODY: u32 = 44;
const CURLOPT_POST: u32 = 47;
const CURLOPT_POSTFIELDS: u32 = 10015;
const CURLOPT_POSTFIELDSIZE_LARGE: u32 = 30120;
const CURLOPT_NOSIGNAL: u32 = 99;
const CURLOPT_CONNECTTIMEOUT_MS: u32 = 156;
const CURLOPT_LOW_SPEED_LIMIT: u32 = 19;
const CURLOPT_LOW_SPEED_TIME: u32 = 20;
const CURLINFO_RESPONSE_CODE: u32 = 0x20_0002;
const MAX_BODY: usize = 16 * 1024 * 1024;

#[link(name = ":libcurl.so.4")]
unsafe extern "C" {
    fn curl_global_init(flags: c_long) -> i32;
    fn curl_easy_init() -> *mut c_void;
    fn curl_easy_setopt(handle: *mut c_void, option: u32, ...) -> i32;
    fn curl_easy_perform(handle: *mut c_void) -> i32;
    fn curl_easy_getinfo(handle: *mut c_void, info: u32, ...) -> i32;
    fn curl_easy_cleanup(handle: *mut c_void);
    fn curl_slist_append(list: *mut c_void, value: *const c_char) -> *mut c_void;
    fn curl_slist_free_all(list: *mut c_void);
}

pub(super) struct Response {
    pub status: u32,
    pub headers: Vec<(String, String)>,
    pub status_line: String,
    pub body: Vec<u8>,
    pub cursor: usize,
}

unsafe extern "C" fn write_body(data: *mut c_char, size: usize, count: usize, context: *mut c_void) -> usize {
    let Some(length) = size.checked_mul(count) else { return 0; };
    let body = unsafe { &mut *(context as *mut Vec<u8>) };
    if length > MAX_BODY.saturating_sub(body.len()) { return 0; }
    body.extend_from_slice(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) });
    length
}

unsafe extern "C" fn write_header(data: *mut c_char, size: usize, count: usize, context: *mut c_void) -> usize {
    let Some(length) = size.checked_mul(count) else { return 0; };
    let state = unsafe { &mut *(context as *mut (Vec<(String, String)>, String)) };
    if length > 64 * 1024 { return 0; }
    let line = String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) });
    let line = line.trim_end_matches(['\r', '\n']);
    if line.starts_with("HTTP/") {
        state.0.clear();
        state.1 = line.to_string();
    } else if let Some((name, value)) = line.split_once(':') {
        state.0.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    length
}

struct HeaderList(*mut c_void);
impl Drop for HeaderList {
    fn drop(&mut self) { if !self.0.is_null() { unsafe { curl_slist_free_all(self.0) }; } }
}
struct Easy(*mut c_void);
impl Drop for Easy {
    fn drop(&mut self) { unsafe { curl_easy_cleanup(self.0) }; }
}

/// Returns a Windows WinHTTP error on failure.
pub(super) fn fetch(host: &str, port: u16, method: &str, path: &str, secure: bool, connect_ms: i32, receive_ms: i32, headers: &[String], request_body: &[u8]) -> Result<Response, u32> {
    static INIT: OnceLock<i32> = OnceLock::new();
    if *INIT.get_or_init(|| unsafe { curl_global_init(3) }) != 0 { return Err(12029); }
    if !matches!(method, "GET" | "HEAD" | "POST") || !path.starts_with('/') || host.contains(['/', '\r', '\n', '@']) {
        return Err(87);
    }
    let scheme = if secure { "https" } else { "http" };
    let url = CString::new(format!("{scheme}://{host}:{port}{path}")).map_err(|_| 87u32)?;
    let easy = Easy(unsafe { curl_easy_init() });
    if easy.0.is_null() { return Err(12029); }
    let mut body = Vec::new();
    let mut response_headers: (Vec<(String, String)>, String) = (Vec::new(), String::new());
    let mut header_list = HeaderList(std::ptr::null_mut());
    for header in headers {
        let value = CString::new(header.as_str()).map_err(|_| 87u32)?;
        let next = unsafe { curl_slist_append(header_list.0, value.as_ptr()) };
        if next.is_null() { return Err(12029); }
        header_list.0 = next;
    }
    let no_proxy = CString::new("").unwrap();
    let set = |option, value: c_long| unsafe { curl_easy_setopt(easy.0, option, value) };
    let configured = unsafe { curl_easy_setopt(easy.0, CURLOPT_URL, url.as_ptr()) } == 0
        && unsafe { curl_easy_setopt(easy.0, CURLOPT_PROXY, no_proxy.as_ptr()) } == 0
        && (header_list.0.is_null() || unsafe { curl_easy_setopt(easy.0, CURLOPT_HTTPHEADER, header_list.0) } == 0)
        && unsafe { curl_easy_setopt(easy.0, CURLOPT_WRITEFUNCTION, write_body as *const c_void) } == 0
        && unsafe { curl_easy_setopt(easy.0, CURLOPT_WRITEDATA, (&mut body as *mut Vec<u8>).cast::<c_void>()) } == 0
        && unsafe { curl_easy_setopt(easy.0, CURLOPT_HEADERFUNCTION, write_header as *const c_void) } == 0
        && unsafe { curl_easy_setopt(easy.0, CURLOPT_HEADERDATA, (&mut response_headers as *mut (Vec<(String, String)>, String)).cast::<c_void>()) } == 0
        && set(CURLOPT_NOSIGNAL, 1) == 0
        && (method != "HEAD" || set(CURLOPT_NOBODY, 1) == 0)
        && (method != "POST" || (set(CURLOPT_POST, 1) == 0
            && unsafe { curl_easy_setopt(easy.0, CURLOPT_POSTFIELDS, request_body.as_ptr()) } == 0
            && unsafe { curl_easy_setopt(easy.0, CURLOPT_POSTFIELDSIZE_LARGE, request_body.len() as i64) } == 0))
        && (connect_ms <= 0 || set(CURLOPT_CONNECTTIMEOUT_MS, connect_ms as c_long) == 0)
        // WinHTTP's receive timeout bounds the wait for data, not the whole transfer:
        // fail when nothing arrives for that long (curl's low-speed limit of 1 byte/s).
        && (receive_ms <= 0 || (set(CURLOPT_LOW_SPEED_LIMIT, 1) == 0
            && set(CURLOPT_LOW_SPEED_TIME, ((receive_ms as c_long) + 999) / 1000) == 0));
    if !configured { return Err(12009); }
    let code = unsafe { curl_easy_perform(easy.0) };
    if code != 0 {
        return Err(match code {
            6 => 12007, // NAME_NOT_RESOLVED
            28 => 12002, // TIMEOUT
            60 | 51 => 12175, // SECURE_FAILURE
            _ => 12029, // CANNOT_CONNECT
        });
    }
    let mut status: c_long = 0;
    if unsafe { curl_easy_getinfo(easy.0, CURLINFO_RESPONSE_CODE, &mut status) } != 0 || !(100..=599).contains(&status) {
        return Err(12030); // CONNECTION_ERROR
    }
    Ok(Response { status: status as u32, headers: response_headers.0, status_line: response_headers.1, body, cursor: 0 })
}
