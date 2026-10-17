//! Process environment (ported from peek). Windows names are case-insensitive: an
//! exact match is tried first, then a case-insensitive one. Missing variables fail
//! with ERROR_ENVVAR_NOT_FOUND.
use crate::api::{system::error, text};
use std::{
    ffi::{c_char, CStr, OsStr},
    sync::Mutex,
};
const ERROR_ENVVAR_NOT_FOUND: u32 = 203;

fn find(name: &str) -> Option<String> {
    if name.is_empty() || name.contains('=') {
        return None;
    }
    std::env::var(name).ok().or_else(|| {
        std::env::vars_os()
            .find(|(k, _)| k.to_string_lossy().eq_ignore_ascii_case(name))
            .map(|(_, v)| v.to_string_lossy().into_owned())
    })
}
pub unsafe extern "win64" fn get_a(name: *const c_char, out: *mut u8, capacity: u32) -> u32 {
    if name.is_null() {
        error::set(87);
        return 0;
    }
    match find(&unsafe { CStr::from_ptr(name) }.to_string_lossy()) {
        Some(v) => unsafe { text::copy_out_narrow(v.as_bytes(), out, capacity) },
        None => {
            error::set(ERROR_ENVVAR_NOT_FOUND);
            0
        }
    }
}
pub unsafe extern "win64" fn get_w(name: *const u16, out: *mut u16, capacity: u32) -> u32 {
    let Some(name) = (unsafe { text::string(name) }) else {
        error::set(87);
        return 0;
    };
    match find(&name) {
        Some(v) => unsafe { text::copy_out(&text::wide(&v), out, capacity) },
        None => {
            error::set(ERROR_ENVVAR_NOT_FOUND);
            0
        }
    }
}
fn set(name: Option<String>, value: Option<String>) -> i32 {
    let Some(name) = name.filter(|n| !n.is_empty() && !n.contains('=') && !n.contains('\0')) else {
        error::set(87);
        return 0;
    };
    // Single process-wide environment; Windows code expects immediate visibility.
    match value {
        Some(v) if !v.contains('\0') => unsafe { std::env::set_var(OsStr::new(&name), v) },
        Some(_) => {
            error::set(87);
            return 0;
        }
        None => unsafe { std::env::remove_var(OsStr::new(&name)) },
    }
    1
}
pub unsafe extern "win64" fn set_w(name: *const u16, value: *const u16) -> i32 {
    set(unsafe { text::string(name) }, unsafe {
        text::string(value)
    })
}
pub unsafe extern "win64" fn set_a(name: *const c_char, value: *const c_char) -> i32 {
    let read = |p: *const c_char| {
        (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    };
    set(read(name), read(value))
}
/// Blocks handed out by GetEnvironmentStringsW, by address, with their length.
static BLOCKS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
/// GetEnvironmentStringsW: `NAME=value\0...\0\0`, freed with FreeEnvironmentStringsW.
pub extern "win64" fn strings_w() -> *mut u16 {
    let mut block: Vec<u16> = Vec::new();
    for (k, v) in std::env::vars_os() {
        block.extend(text::wide(&format!(
            "{}={}",
            k.to_string_lossy(),
            v.to_string_lossy()
        )));
        block.push(0);
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    let length = block.len();
    let p: *mut u16 = Box::into_raw(block.into_boxed_slice()).cast();
    BLOCKS.lock().unwrap().push((p as usize, length));
    p
}
pub unsafe extern "win64" fn free_strings_w(block: *mut u16) -> i32 {
    let mut blocks = BLOCKS.lock().unwrap();
    let Some(index) = blocks.iter().position(|(p, _)| *p == block as usize) else {
        error::set(87);
        return 0;
    };
    let (_, length) = blocks.swap_remove(index);
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(block, length)) });
    1
}
static BLOCKS_A: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
pub extern "win64" fn strings_a() -> *mut u8 {
    let mut block: Vec<u8> = Vec::new();
    for (k, v) in std::env::vars_os() {
        block.extend_from_slice(
            format!("{}={}", k.to_string_lossy(), v.to_string_lossy()).as_bytes(),
        );
        block.push(0);
    }
    block.push(0);
    if block.len() == 1 {
        block.push(0);
    }
    let length = block.len();
    let p: *mut u8 = Box::into_raw(block.into_boxed_slice()).cast();
    BLOCKS_A.lock().unwrap().push((p as usize, length));
    p
}
pub unsafe extern "win64" fn free_strings_a(block: *mut u8) -> i32 {
    let mut blocks = BLOCKS_A.lock().unwrap();
    let Some(index) = blocks.iter().position(|(p, _)| *p == block as usize) else {
        error::set(87);
        return 0;
    };
    let (_, length) = blocks.swap_remove(index);
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(block, length)) });
    1
}
/// `%NAME%` replaced by the variable's value; unknown names stay as written.
pub(crate) fn expand(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    while let Some(start) = rest.find('%') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => match find(&after[..end]) {
                Some(value) => {
                    result.push_str(&value);
                    rest = &after[end + 1..];
                }
                None => {
                    result.push('%');
                    rest = after;
                }
            },
            None => {
                result.push('%');
                rest = after;
            }
        }
    }
    result.push_str(rest);
    result
}
/// ExpandEnvironmentStringsW: `%NAME%` becomes the variable's value; unknown names
/// stay as written. Returns the size needed including the NUL.
pub unsafe extern "win64" fn expand_w(source: *const u16, out: *mut u16, capacity: u32) -> u32 {
    let Some(source) = (unsafe { text::string(source) }) else {
        error::set(87);
        return 0;
    };
    let result = expand(&source);
    let wide = text::wide(&result);
    let needed = wide.len() as u32 + 1;
    if !out.is_null() && capacity >= needed {
        unsafe { text::copy_out(&wide, out, capacity) };
    }
    needed
}
