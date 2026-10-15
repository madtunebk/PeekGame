use crate::api::system::error;
use std::ffi::{c_char, CStr};
fn page(cp: u32) -> bool {
    matches!(cp, 0 | 1 | 3 | 65001)
}
/// CP_ACP, CP_OEMCP and CP_THREAD_ACP are UTF-8 here, but programs pass them the
/// flags (and default-character arguments) of a legacy code page, which Windows
/// accepts for those: they are ignored. Explicit CP_UTF8 keeps the UTF-8 rules.
fn legacy(cp: u32) -> bool {
    cp != 65001
}
pub extern "win64" fn codepage() -> u32 {
    65001
}
pub unsafe extern "win64" fn wide_to_multi(
    cp: u32,
    flags: u32,
    input: *const u16,
    len: i32,
    out: *mut u8,
    capacity: i32,
    default: *const u8,
    used: *mut i32,
) -> i32 {
    if !page(cp)
        || input.is_null()
        || len == 0
        || len < -1
        || capacity < 0
        || (!legacy(cp) && (!default.is_null() || !used.is_null()))
    {
        error::set(87);
        return 0;
    }
    if !used.is_null() {
        unsafe { *used = 0 };
    }
    // WC_COMPOSITECHECK 0x200, WC_DISCARDNS 0x10, WC_SEPCHARS 0x20, WC_DEFAULTCHAR
    // 0x40 and WC_NO_BEST_FIT_CHARS 0x400 only concern legacy code pages.
    let flags = if legacy(cp) { flags & !0x670 } else { flags };
    if flags & !0x80 != 0 {
        error::set(1004);
        return 0;
    }
    let size = if len == -1 {
        let mut n = 0;
        while unsafe { *input.add(n) } != 0 {
            n += 1;
        }
        n + 1
    } else {
        len as usize
    };
    let input = unsafe { std::slice::from_raw_parts(input, size) };
    let text = if flags & 0x80 != 0 {
        match String::from_utf16(input) {
            Ok(s) => s,
            Err(_) => {
                error::set(1113);
                return 0;
            }
        }
    } else {
        String::from_utf16_lossy(input)
    };
    let size = text.len();
    if size > i32::MAX as usize {
        error::set(87);
        return 0;
    }
    if capacity == 0 {
        return size as i32;
    }
    if out.is_null() {
        error::set(87);
        return 0;
    }
    if capacity < (size as i32) {
        error::set(122);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, size);
    }
    size as i32
}
pub unsafe extern "win64" fn multi_to_wide(
    cp: u32,
    flags: u32,
    input: *const c_char,
    len: i32,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if !page(cp) || input.is_null() || len == 0 || len < -1 || capacity < 0 {
        error::set(87);
        return 0;
    }
    // MB_PRECOMPOSED 1, MB_COMPOSITE 2 and MB_USEGLYPHCHARS 4 concern legacy pages.
    let flags = if legacy(cp) { flags & !7 } else { flags };
    if flags & !8 != 0 {
        error::set(1004);
        return 0;
    }
    let input = if len == -1 {
        unsafe { CStr::from_ptr(input) }.to_bytes_with_nul()
    } else {
        unsafe { std::slice::from_raw_parts(input.cast::<u8>(), len as usize) }
    };
    let text = if flags & 8 != 0 {
        match std::str::from_utf8(input) {
            Ok(s) => std::borrow::Cow::Borrowed(s),
            Err(_) => {
                error::set(1113);
                return 0;
            }
        }
    } else {
        String::from_utf8_lossy(input)
    };
    let encoded = text.encode_utf16().collect::<Vec<_>>();
    let size = encoded.len();
    if size > i32::MAX as usize {
        error::set(87);
        return 0;
    }
    if capacity == 0 {
        return size as i32;
    }
    if out.is_null() {
        error::set(87);
        return 0;
    }
    if capacity < (size as i32) {
        error::set(122);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), out, size);
    }
    size as i32
}
