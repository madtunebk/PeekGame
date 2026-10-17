//! CRYPT32 Base64/PEM encoding with Windows character-count and buffer contracts.
use crate::api::system::error;
fn encode(input: &[u8], flags: u32) -> Result<Vec<u8>, u32> {
    if flags & !0xc000ffff != 0 {
        return Err(87);
    }
    let kind = flags & 0xffff;
    if !matches!(kind, 0 | 1) {
        return Err(87);
    }
    let eol: &[u8] = if flags & 0x40000000 != 0 {
        b""
    } else if flags & 0x80000000 != 0 {
        b"\n"
    } else {
        b"\r\n"
    };
    let chars = input.len().div_ceil(3).checked_mul(4).ok_or(8u32)?;
    let lines = chars.div_ceil(64);
    let size = chars
        .checked_add(lines.checked_mul(eol.len()).ok_or(8u32)?)
        .and_then(|n| n.checked_add(if kind == 0 { 52 + 2 * eol.len() } else { 0 }))
        .ok_or(8u32)?;
    if size >= u32::MAX as usize {
        return Err(8);
    }
    let mut out = Vec::new();
    out.try_reserve(size).map_err(|_| 8u32)?;
    if kind == 0 {
        out.extend_from_slice(b"-----BEGIN CERTIFICATE-----");
        out.extend_from_slice(eol);
    }
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut column = 0;
    for part in input.chunks(3) {
        let v = ((part[0] as u32) << 16)
            | ((part.get(1).copied().unwrap_or(0) as u32) << 8)
            | part.get(2).copied().unwrap_or(0) as u32;
        out.push(ALPHABET[((v >> 18) & 63) as usize]);
        out.push(ALPHABET[((v >> 12) & 63) as usize]);
        out.push(if part.len() > 1 {
            ALPHABET[((v >> 6) & 63) as usize]
        } else {
            b'='
        });
        out.push(if part.len() > 2 {
            ALPHABET[(v & 63) as usize]
        } else {
            b'='
        });
        column += 4;
        if column == 64 {
            out.extend_from_slice(eol);
            column = 0;
        }
    }
    if column != 0 {
        out.extend_from_slice(eol);
    }
    if kind == 0 {
        out.extend_from_slice(b"-----END CERTIFICATE-----");
        out.extend_from_slice(eol);
    }
    Ok(out)
}
unsafe fn convert<T: Copy + From<u8>>(
    input: *const u8,
    length: u32,
    flags: u32,
    out: *mut T,
    size: *mut u32,
) -> i32 {
    let fail = |code| {
        error::set(code);
        0
    };
    if size.is_null() || (input.is_null() && length != 0) {
        return fail(87);
    }
    let input = if length == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(input, length as usize) }
    };
    let bytes = match encode(input, flags) {
        Ok(bytes) => bytes,
        Err(code) => return fail(code),
    };
    let capacity = unsafe { *size };
    let needed = bytes.len() as u32 + 1;
    if out.is_null() {
        unsafe {
            *size = needed;
        }
        return 1;
    }
    if capacity < needed {
        unsafe {
            *size = needed;
        }
        return fail(234);
    }
    for (i, &byte) in bytes.iter().enumerate() {
        unsafe {
            *out.add(i) = T::from(byte);
        }
    }
    unsafe {
        *out.add(bytes.len()) = T::from(0);
        *size = bytes.len() as u32;
    }
    1
}
pub unsafe extern "win64" fn binary_to_string_a(
    input: *const u8,
    length: u32,
    flags: u32,
    out: *mut u8,
    size: *mut u32,
) -> i32 {
    unsafe { convert(input, length, flags, out, size) }
}
pub unsafe extern "win64" fn binary_to_string_w(
    input: *const u8,
    length: u32,
    flags: u32,
    out: *mut u16,
    size: *mut u32,
) -> i32 {
    unsafe { convert(input, length, flags, out, size) }
}
