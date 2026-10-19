use crate::{api::system::error, host::paths};
use std::ffi::c_char;
fn fail(code: u32) -> u32 {
    error::set(code);
    0
}
unsafe fn output(text: &str, out: *mut u16, size: u32) -> u32 {
    let text = text.encode_utf16().collect::<Vec<_>>();
    let needed = text.len() + 1;
    if needed > size as usize {
        return needed as u32;
    }
    if out.is_null() {
        return fail(87);
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
        *out.add(text.len()) = 0;
    }
    text.len() as u32
}
pub unsafe extern "win64" fn current_w(size: u32, out: *mut u16) -> u32 {
    match std::env::current_dir() {
        Ok(path) => unsafe { output(&paths::windows(&path), out, size) },
        Err(e) => fail(paths::error(&e)),
    }
}
pub unsafe extern "win64" fn set_current_w(input: *const u16) -> i32 {
    let path = unsafe { paths::wide(input) }.and_then(|s| paths::host(&s));
    match path {
        Ok(path) => match std::env::set_current_dir(path) {
            Ok(()) => 1,
            Err(e) => {
                fail(paths::error(&e));
                0
            }
        },
        Err(e) => {
            fail(e);
            0
        }
    }
}
pub unsafe extern "win64" fn set_current_a(input: *const c_char) -> i32 {
    if input.is_null() {
        error::set(87);
        return 0;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(input) }.to_string_lossy();
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { set_current_w(wide.as_ptr()) }
}
pub unsafe extern "win64" fn full_w(
    input: *const u16,
    size: u32,
    out: *mut u16,
    part: *mut *mut u16,
) -> u32 {
    if !part.is_null() {
        unsafe {
            *part = std::ptr::null_mut();
        }
    }
    let path = unsafe { paths::wide(input) }.and_then(|s| paths::full(&s));
    match path {
        Ok(path) => {
            let text = paths::windows(&path);
            let result = unsafe { output(&text, out, size) };
            if result > 0 && result < size && !part.is_null() {
                let offset = text
                    .rfind('\\')
                    .map_or(0, |i| text[..i + 1].encode_utf16().count());
                unsafe {
                    *part = out.add(offset);
                }
            }
            result
        }
        Err(e) => fail(e),
    }
}
pub unsafe extern "win64" fn full_a(
    input: *const c_char,
    size: u32,
    out: *mut u8,
    part: *mut *mut u8,
) -> u32 {
    if !part.is_null() {
        unsafe { *part = std::ptr::null_mut() };
    }
    let path = unsafe { paths::narrow(input) }.and_then(|s| paths::full(&s));
    match path {
        Ok(path) => {
            let text = paths::windows(&path);
            let bytes = text.as_bytes();
            let needed = bytes.len() + 1;
            if needed > size as usize {
                return needed as u32;
            }
            if out.is_null() {
                return fail(87);
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
                *out.add(bytes.len()) = 0;
                if !part.is_null() {
                    let offset = text.rfind('\\').map_or(0, |i| i + 1);
                    *part = out.add(offset);
                }
            }
            bytes.len() as u32
        }
        Err(e) => fail(e),
    }
}
pub unsafe extern "win64" fn attributes_w(input: *const u16) -> u32 {
    match unsafe { paths::wide(input) }.and_then(|s| paths::host(&s)) {
        Ok(path) => match std::fs::metadata(path) {
            Ok(m) => {
                let mut value = if m.is_dir() { 0x10 } else { 0x80 };
                if m.permissions().readonly() {
                    value |= 1;
                }
                value
            }
            Err(e) => {
                error::set(paths::error(&e));
                u32::MAX
            }
        },
        Err(e) => {
            error::set(e);
            u32::MAX
        }
    }
}
pub unsafe extern "win64" fn attributes_a(input: *const c_char) -> u32 {
    match unsafe { paths::narrow(input) } {
        Ok(s) => {
            let w = s.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            unsafe { attributes_w(w.as_ptr()) }
        }
        Err(e) => {
            error::set(e);
            u32::MAX
        }
    }
}

pub unsafe extern "win64" fn long_w(input: *const u16, out: *mut u16, size: u32) -> u32 {
    let text = match unsafe { paths::wide(input) } {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let host = match paths::host(&text) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if let Err(e) = std::fs::metadata(host) {
        return fail(paths::error(&e));
    }
    unsafe { output(&text, out, size) }
}

const S_OK: i32 = 0;
const S_FALSE: i32 = 1;
const E_INVALIDARG: i32 = 0x8007_0057u32 as i32;
const STRSAFE_E_INSUFFICIENT_BUFFER: i32 = 0x8007_007au32 as i32;
const PATHCCH_MAX_CCH: usize = 0x8000;

pub unsafe extern "win64" fn path_cch_remove_file_spec(
    psz_path: *mut u16,
    cch_path: usize,
) -> i32 {
    if psz_path.is_null() || cch_path == 0 || cch_path > PATHCCH_MAX_CCH {
        return E_INVALIDARG;
    }
    let mut len = 0;
    while len < cch_path && unsafe { *psz_path.add(len) } != 0 {
        len += 1;
    }
    if len >= cch_path {
        return E_INVALIDARG;
    }
    if len == 0 {
        return S_FALSE;
    }

    let slice = unsafe { std::slice::from_raw_parts_mut(psz_path, len + 1) };

    let is_drive = len >= 2
        && ((slice[0] >= b'A' as u16 && slice[0] <= b'Z' as u16)
            || (slice[0] >= b'a' as u16 && slice[0] <= b'z' as u16))
        && slice[1] == b':' as u16;

    let root_len = if is_drive {
        if len >= 3 && (slice[2] == b'\\' as u16 || slice[2] == b'/' as u16) {
            3
        } else {
            2
        }
    } else if slice[0] == b'\\' as u16 || slice[0] == b'/' as u16 {
        if len >= 2 && (slice[1] == b'\\' as u16 || slice[1] == b'/' as u16) {
            let mut pos = 2;
            let mut count = 0;
            while pos < len {
                if slice[pos] == b'\\' as u16 || slice[pos] == b'/' as u16 {
                    count += 1;
                    if count == 2 {
                        break;
                    }
                }
                pos += 1;
            }
            if count >= 1 {
                pos + 1
            } else {
                len
            }
        } else {
            1
        }
    } else {
        0
    };

    if len <= root_len {
        return S_FALSE;
    }

    let mut last_sep = None;
    for i in (root_len..len).rev() {
        if slice[i] == b'\\' as u16 || slice[i] == b'/' as u16 {
            last_sep = Some(i);
            break;
        }
    }

    if let Some(sep_idx) = last_sep {
        if is_drive && sep_idx == 2 {
            slice[3] = 0;
        } else {
            slice[sep_idx] = 0;
        }
    } else if root_len > 0 {
        slice[root_len] = 0;
    } else {
        slice[0] = 0;
    }

    S_OK
}

pub unsafe extern "win64" fn path_cch_combine_ex(
    psz_path_out: *mut u16,
    cch_path_out: usize,
    psz_path_in: *const u16,
    psz_more: *const u16,
    _dw_flags: u32,
) -> i32 {
    if psz_path_out.is_null() || cch_path_out == 0 || cch_path_out > PATHCCH_MAX_CCH {
        return E_INVALIDARG;
    }
    if psz_path_in.is_null() && psz_more.is_null() {
        unsafe { *psz_path_out = 0 };
        return E_INVALIDARG;
    }

    let read_wide = |p: *const u16| -> Vec<u16> {
        if p.is_null() {
            return Vec::new();
        }
        let mut v = Vec::new();
        let mut i = 0;
        while i < PATHCCH_MAX_CCH {
            let c = unsafe { *p.add(i) };
            if c == 0 {
                break;
            }
            v.push(c);
            i += 1;
        }
        v
    };

    let mut in_str = read_wide(psz_path_in);
    let more_str = read_wide(psz_more);

    let more_is_abs = (more_str.len() >= 2
        && ((more_str[0] >= b'A' as u16 && more_str[0] <= b'Z' as u16)
            || (more_str[0] >= b'a' as u16 && more_str[0] <= b'z' as u16))
        && more_str[1] == b':' as u16)
        || (more_str.len() >= 2
            && (more_str[0] == b'\\' as u16 || more_str[0] == b'/' as u16)
            && (more_str[1] == b'\\' as u16 || more_str[1] == b'/' as u16));

    let raw_combined = if more_is_abs || in_str.is_empty() {
        more_str
    } else if more_str.is_empty() {
        in_str
    } else if more_str[0] == b'\\' as u16 || more_str[0] == b'/' as u16 {
        let is_drive = in_str.len() >= 2 && in_str[1] == b':' as u16;
        if is_drive {
            in_str.truncate(2);
        } else {
            in_str.clear();
        }
        in_str.extend_from_slice(&more_str);
        in_str
    } else {
        let last = *in_str.last().unwrap();
        if last != b'\\' as u16 && last != b'/' as u16 {
            in_str.push(b'\\' as u16);
        }
        in_str.extend_from_slice(&more_str);
        in_str
    };

    let norm: Vec<u16> = raw_combined
        .into_iter()
        .map(|c| if c == b'/' as u16 { b'\\' as u16 } else { c })
        .collect();

    let (prefix, rest_idx) = if norm.len() >= 3 && norm[1] == b':' as u16 && norm[2] == b'\\' as u16 {
        (&norm[..3], 3)
    } else if norm.len() >= 2 && norm[1] == b':' as u16 {
        (&norm[..2], 2)
    } else if !norm.is_empty() && norm[0] == b'\\' as u16 {
        (&norm[..1], 1)
    } else {
        (&norm[..0], 0)
    };

    let mut parts: Vec<&[u16]> = Vec::new();
    let rest = &norm[rest_idx..];
    for part in rest.split(|&c| c == b'\\' as u16) {
        if part.is_empty() || (part.len() == 1 && part[0] == b'.' as u16) {
            continue;
        } else if part.len() == 2 && part[0] == b'.' as u16 && part[1] == b'.' as u16 {
            if !parts.is_empty()
                && !(parts.last().unwrap().len() == 2
                    && parts.last().unwrap()[0] == b'.' as u16
                    && parts.last().unwrap()[1] == b'.' as u16)
            {
                parts.pop();
            } else if prefix.is_empty() {
                parts.push(part);
            }
        } else {
            parts.push(part);
        }
    }

    let mut result = Vec::new();
    result.extend_from_slice(prefix);
    for (idx, p) in parts.iter().enumerate() {
        if idx > 0 || (idx == 0 && !prefix.is_empty() && !prefix.ends_with(&[b'\\' as u16])) {
            result.push(b'\\' as u16);
        }
        result.extend_from_slice(p);
    }
    if result.is_empty() {
        if !prefix.is_empty() {
            result.extend_from_slice(prefix);
        } else {
            result.push(b'.' as u16);
        }
    }

    if result.len() + 1 > cch_path_out {
        unsafe { *psz_path_out = 0 };
        return STRSAFE_E_INSUFFICIENT_BUFFER;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr(), psz_path_out, result.len());
        *psz_path_out.add(result.len()) = 0;
    }

    S_OK
}

pub unsafe extern "win64" fn path_cch_combine(
    psz_path_out: *mut u16,
    cch_path_out: usize,
    psz_path_in: *const u16,
    psz_more: *const u16,
) -> i32 {
    unsafe { path_cch_combine_ex(psz_path_out, cch_path_out, psz_path_in, psz_more, 0) }
}

pub unsafe extern "win64" fn path_cch_add_backslash(
    psz_path: *mut u16,
    cch_path: usize,
) -> i32 {
    if psz_path.is_null() || cch_path == 0 || cch_path > PATHCCH_MAX_CCH {
        return E_INVALIDARG;
    }
    let mut len = 0;
    while len < cch_path && unsafe { *psz_path.add(len) } != 0 {
        len += 1;
    }
    if len >= cch_path {
        return E_INVALIDARG;
    }
    if len > 0 && unsafe { *psz_path.add(len - 1) } == b'\\' as u16 {
        return S_FALSE;
    }
    if len + 2 > cch_path {
        return STRSAFE_E_INSUFFICIENT_BUFFER;
    }
    unsafe {
        *psz_path.add(len) = b'\\' as u16;
        *psz_path.add(len + 1) = 0;
    }
    S_OK
}
