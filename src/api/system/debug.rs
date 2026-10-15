use std::{
    ffi::{c_char, CStr},
    io::Write,
};
// The prototype's debug sink is Linux stderr, separate from Windows standard handles.
// Like the Windows void API, a failed debug sink cannot report a write result.
pub(crate) unsafe extern "win64" fn output_debug_string_a(text: *const c_char) {
    if text.is_null() {
        return;
    }
    let bytes = unsafe { CStr::from_ptr(text) }.to_bytes();
    let _ = std::io::stderr().lock().write_all(bytes);
}
