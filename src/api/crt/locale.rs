//! CRT locale policy. Only C/POSIX is supported; no host setlocale mutation.
use std::cell::Cell;
thread_local! {
    // _DISABLE_PER_THREAD_LOCALE; fresh threads use the shared C locale.
    static THREAD_LOCALE: Cell<i32> = const { Cell::new(2) };
}
/// Query (0), enable (1) or disable (2); return the previous policy.
/// Locale content is currently C-only, so both policies select the same locale.
pub extern "win64" fn configure_thread(mode: i32) -> i32 {
    if !matches!(mode, 0..=2) {
        unsafe { *super::stdio::errno() = 22; }
        return -1;
    }
    THREAD_LOCALE.with(|state| {
        let previous = state.get();
        if mode != 0 { state.set(mode); }
        previous
    })
}
use std::ffi::{c_char, CStr};
pub extern "win64" fn codepage() -> u32 {
    0
}
pub extern "win64" fn max_bytes() -> i32 {
    1
}
pub unsafe extern "win64" fn set_locale(category: i32, locale: *const c_char) -> *const c_char {
    if !(0..=5).contains(&category) {
        return std::ptr::null();
    }
    if locale.is_null() {
        return c"C".as_ptr();
    }
    let locale = unsafe { CStr::from_ptr(locale) }.to_bytes();
    if locale == b"C" || locale == b"POSIX" {
        c"C".as_ptr()
    } else {
        std::ptr::null()
    }
}
