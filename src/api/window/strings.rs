pub(super) unsafe fn wide(pointer: *const u16) -> Result<String, u32> {
    if pointer.is_null() || (pointer as usize) < 65536 {
        return Err(87);
    }
    let mut units = Vec::new();
    for offset in 0..32768 {
        let ch = unsafe { pointer.add(offset).read() };
        if ch == 0 {
            return Ok(String::from_utf16_lossy(&units));
        }
        units.push(ch);
    }
    Err(206)
}
pub(super) unsafe fn narrow(pointer: *const i8) -> Result<String, u32> {
    if pointer.is_null() || (pointer as usize) < 65536 {
        return Err(87);
    }
    Ok(unsafe { std::ffi::CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned())
}
/// Copies `text` into a W buffer of `capacity` characters; the count copied.
pub(super) unsafe fn copy_w(text: &str, out: *mut u16, capacity: i32) -> i32 {
    if out.is_null() || capacity <= 0 {
        return 0;
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    let n = units.len().min(capacity as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(units.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    n as i32
}
pub(super) unsafe fn copy_a(text: &str, out: *mut i8, capacity: i32) -> i32 {
    if out.is_null() || capacity <= 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let n = bytes.len().min(capacity as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), out, n);
        *out.add(n) = 0;
    }
    n as i32
}
