//! AppUserModelID maps to native X11 window grouping, without SDL.
pub unsafe extern "win64" fn set(id: *const u16) -> i32 {
    let invalid = 0x80070057u32 as i32;
    if id.is_null() {
        return invalid;
    }
    let mut length = 0;
    while length <= 128 && unsafe { *id.add(length) } != 0 {
        length += 1;
    }
    if length == 0 || length > 128 {
        return invalid;
    }
    let Ok(text) = String::from_utf16(unsafe { std::slice::from_raw_parts(id, length) }) else {
        return invalid;
    };
    if text.chars().any(char::is_whitespace) {
        return invalid;
    }
    crate::host::x11::identity::set(text);
    0
}

/// Per-window shell property stores are unavailable; do not expose audio stores
/// or a host COM object as a window store.
pub unsafe extern "win64" fn window_property_store(
    _hwnd: usize,
    _iid: *const std::ffi::c_void,
    out: *mut *mut std::ffi::c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32;
    } // E_POINTER
    unsafe {
        *out = std::ptr::null_mut();
    }
    0x80004001u32 as i32 // E_NOTIMPL
}
