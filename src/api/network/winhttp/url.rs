//! WinHttpCrackUrl for HTTP(S) URLs. Output slices either point into the
//! caller's URL or are copied into caller-provided UTF-16 buffers.
use crate::api::system::error;

#[repr(C)]
struct UrlComponents {
    size: u32,
    scheme: *mut u16,
    scheme_len: u32,
    scheme_id: u32,
    host: *mut u16,
    host_len: u32,
    port: u16,
    user: *mut u16,
    user_len: u32,
    password: *mut u16,
    password_len: u32,
    path: *mut u16,
    path_len: u32,
    extra: *mut u16,
    extra_len: u32,
}
fn fail(code: u32) -> i32 { error::set(code); 0 }
unsafe fn component(input: *const u16, data: &[u16], field: &mut *mut u16, length: &mut u32, start: usize, end: usize) -> Result<(), u32> {
    let count = end - start;
    if (*field).is_null() {
        if *length != 0 {
            *field = unsafe { input.add(start) as *mut u16 };
            *length = count as u32;
        }
        return Ok(());
    }
    let capacity = *length as usize;
    if capacity <= count {
        *length = (count + 1) as u32;
        return Err(122); // ERROR_INSUFFICIENT_BUFFER
    }
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr().add(start), *field, count);
        *(*field).add(count) = 0;
    }
    *length = count as u32;
    Ok(())
}
pub unsafe extern "win64" fn crack_url(input: *const u16, input_length: u32, flags: u32, components: *mut u8) -> i32 {
    if input.is_null() || components.is_null() { return fail(87); }
    if flags != 0 { return fail(50); }
    let c = unsafe { &mut *components.cast::<UrlComponents>() };
    if c.size as usize != std::mem::size_of::<UrlComponents>() { return fail(87); }
    let len = if input_length == 0 {
        let Some(len) = (0..65536).position(|i| unsafe { *input.add(i) } == 0) else { return fail(87); };
        len
    } else if input_length <= 65536 { input_length as usize } else { return fail(87); };
    let data = unsafe { std::slice::from_raw_parts(input, len) };
    let Some(colon) = data.iter().position(|&u| u == b':' as u16) else { return fail(12006); };
    let scheme = String::from_utf16_lossy(&data[..colon]).to_ascii_lowercase();
    c.scheme_id = match scheme.as_str() { "http" => 1, "https" => 2, _ => return fail(12006) };
    if data.get(colon + 1) != Some(&(b'/' as u16)) || data.get(colon + 2) != Some(&(b'/' as u16)) { return fail(12005); }
    let authority_start = colon + 3;
    let authority_end = (authority_start..len).find(|&i| matches!(data[i] as u8, b'/' | b'?' | b'#')).unwrap_or(len);
    let at = (authority_start..authority_end).rev().find(|&i| data[i] == b'@' as u16);
    let host_start = at.map_or(authority_start, |i| i + 1);
    let mut host_end = authority_end;
    let mut port = if c.scheme_id == 2 { 443 } else { 80 };
    if data.get(host_start) == Some(&(b'[' as u16)) {
        let Some(close) = (host_start + 1..authority_end).find(|&i| data[i] == b']' as u16) else { return fail(12005); };
        host_end = close + 1;
        if host_end < authority_end && data[host_end] != b':' as u16 { return fail(12005); }
    } else if let Some(i) = (host_start..authority_end).rev().find(|&i| data[i] == b':' as u16) {
        host_end = i;
    }
    if host_end == host_start { return fail(12005); }
    if host_end < authority_end {
        let port_text = String::from_utf16_lossy(&data[host_end + 1..authority_end]);
        port = match port_text.parse::<u16>() { Ok(n) if n != 0 => n, _ => return fail(12005) };
    }
    c.port = port;
    let extra_start = (authority_end..len).find(|&i| data[i] == b'?' as u16 || data[i] == b'#' as u16).unwrap_or(len);
    let (path_start, path_end) = if authority_end < len && data[authority_end] == b'/' as u16 { (authority_end, extra_start) } else { (authority_end, authority_end) };
    let (user_start, user_end, password_start, password_end) = if let Some(at) = at {
        if let Some(sep) = (authority_start..at).find(|&i| data[i] == b':' as u16) {
            (authority_start, sep, sep + 1, at)
        } else { (authority_start, at, at, at) }
    } else { (authority_start, authority_start, authority_start, authority_start) };
    let fields = [
        (&mut c.scheme, &mut c.scheme_len, 0, colon),
        (&mut c.host, &mut c.host_len, host_start, host_end),
        (&mut c.user, &mut c.user_len, user_start, user_end),
        (&mut c.password, &mut c.password_len, password_start, password_end),
        (&mut c.path, &mut c.path_len, path_start, path_end),
        (&mut c.extra, &mut c.extra_len, extra_start, len),
    ];
    for (field, length, start, end) in fields {
        if let Err(code) = unsafe { component(input, data, field, length, start, end) } { return fail(code); }
    }
    error::set(0);
    1
}
