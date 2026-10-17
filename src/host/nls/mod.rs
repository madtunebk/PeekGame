//! Locale metadata from the host ICU database, kept behind a System V adapter.
use std::ffi::{c_char, CString};
unsafe extern "C" {
    fn peekgame_nls_number(locale: *const c_char, kind: u32, out: *mut u16, capacity: i32) -> i32;
}
pub(crate) fn number(locale: &str, kind: u32) -> Option<String> {
    let locale = CString::new(locale.replace('-', "_")).ok()?;
    let mut out = [0u16; 512];
    let count =
        unsafe { peekgame_nls_number(locale.as_ptr(), kind, out.as_mut_ptr(), out.len() as i32) };
    if count < 0 || count as usize >= out.len() {
        return None;
    }
    String::from_utf16(&out[..count as usize]).ok()
}
