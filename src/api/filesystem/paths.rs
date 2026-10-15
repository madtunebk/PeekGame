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
