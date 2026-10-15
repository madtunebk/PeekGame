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
