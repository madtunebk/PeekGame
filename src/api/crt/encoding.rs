//! UTF-16/UTF-8 CRT conversions, matching this runtime's narrow-string encoding.
use super::stdio;
pub unsafe extern "win64" fn wcstombs(out: *mut u8, input: *const u16, capacity: usize) -> usize {
    if input.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return usize::MAX;
    }
    let mut cursor = input;
    let mut written = 0usize;
    loop {
        if !out.is_null() && written == capacity {
            return written;
        }
        let first = unsafe { *cursor };
        if first == 0 {
            if !out.is_null() {
                unsafe {
                    *out.add(written) = 0;
                }
            }
            return written;
        }
        let (code, units) = if (0xd800..=0xdbff).contains(&first) {
            let second = unsafe { *cursor.add(1) };
            if !(0xdc00..=0xdfff).contains(&second) {
                unsafe {
                    *stdio::errno() = 42;
                }
                return usize::MAX;
            }
            (
                0x10000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00)),
                2,
            )
        } else {
            (first as u32, 1)
        };
        let Some(ch) = char::from_u32(code) else {
            unsafe {
                *stdio::errno() = 42;
            }
            return usize::MAX;
        };
        let mut buffer = [0u8; 4];
        let bytes = ch.encode_utf8(&mut buffer).as_bytes();
        if !out.is_null() {
            if bytes.len() > capacity - written {
                return written;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.add(written), bytes.len());
            }
        }
        written += bytes.len();
        cursor = unsafe { cursor.add(units) };
    }
}

pub unsafe extern "win64" fn mbstowcs(
    out: *mut u16,
    input: *const std::ffi::c_char,
    capacity: usize,
) -> usize {
    if input.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return usize::MAX;
    }
    let Ok(text) = (unsafe { std::ffi::CStr::from_ptr(input) }).to_str() else {
        unsafe {
            *stdio::errno() = 42;
        }
        return usize::MAX;
    };
    let mut written = 0;
    for ch in text.chars() {
        let mut temp = [0u16; 2];
        let units = ch.encode_utf16(&mut temp);
        if !out.is_null() {
            if units.len() > capacity - written {
                return written;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(units.as_ptr(), out.add(written), units.len());
            }
        }
        written += units.len();
    }
    if !out.is_null() && written < capacity {
        unsafe {
            *out.add(written) = 0;
        }
    }
    written
}
