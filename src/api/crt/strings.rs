//! C-locale character classification and bounded narrow/wide strings.
use std::ffi::{c_char, CStr};
fn err(code: i32) -> i32 {
    unsafe {
        *super::stdio::errno() = code;
    }
    code
}
fn ascii(c: i32) -> Option<u8> {
    (0..128).contains(&c).then_some(c as u8)
}
pub extern "win64" fn isalnum(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_alphanumeric()) as i32
}
pub extern "win64" fn isalpha(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_alphabetic()) as i32
}
pub extern "win64" fn isdigit(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_digit()) as i32
}
pub extern "win64" fn isspace(c: i32) -> i32 {
    ascii(c).is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12)) as i32
}
pub extern "win64" fn isxdigit(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_hexdigit()) as i32
}
pub extern "win64" fn iswspace(c: u16) -> i32 {
    isspace(c as i32)
}
pub extern "win64" fn tolower(c: i32) -> i32 {
    ascii(c).map_or(c, |c| c.to_ascii_lowercase() as i32)
}
pub extern "win64" fn toupper(c: i32) -> i32 {
    ascii(c).map_or(c, |c| c.to_ascii_uppercase() as i32)
}
pub unsafe extern "win64" fn memicmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let d = tolower(unsafe { *a.add(i) } as i32) - tolower(unsafe { *b.add(i) } as i32);
        if d != 0 {
            return d;
        }
    }
    0
}
pub unsafe extern "win64" fn stricmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let (x, y) = unsafe { (*a.add(i), *b.add(i)) };
        let d = tolower(x as i32) - tolower(y as i32);
        if d != 0 || x == 0 {
            return d;
        }
        i += 1;
    }
}
pub unsafe extern "win64" fn wcscpy(out: *mut u16, input: *const u16) -> *mut u16 {
    let mut i = 0;
    loop {
        let c = unsafe { *input.add(i) };
        unsafe {
            *out.add(i) = c;
        }
        if c == 0 {
            break;
        }
        i += 1;
    }
    out
}
pub unsafe extern "win64" fn strcpy_s(out: *mut u8, n: usize, input: *const c_char) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    if input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let bytes = unsafe { CStr::from_ptr(input) }.to_bytes_with_nul();
    if bytes.len() > n {
        unsafe {
            *out = 0;
        }
        return err(34);
    }
    unsafe {
        std::ptr::copy(bytes.as_ptr(), out, bytes.len());
    }
    0
}
pub unsafe extern "win64" fn strcat_s(out: *mut u8, n: usize, input: *const c_char) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    let mut len = 0;
    while len < n && unsafe { *out.add(len) } != 0 {
        len += 1;
    }
    if len == n || input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let r = unsafe { strcpy_s(out.add(len), n - len, input) };
    if r != 0 {
        unsafe {
            *out = 0;
        }
    }
    r
}
pub unsafe extern "win64" fn wcscpy_s(out: *mut u16, n: usize, input: *const u16) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    if input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let len = unsafe { crate::api::text::units(input) }.len();
    if len >= n {
        unsafe {
            *out = 0;
        }
        return err(34);
    }
    unsafe {
        std::ptr::copy(input, out, len + 1);
    }
    0
}
