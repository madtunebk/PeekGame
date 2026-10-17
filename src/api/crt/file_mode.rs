//! Process CRT default translation-mode configuration. Stream translation remains
//! limited by the existing stdio backend; this module owns no FILE descriptors.
use super::{startup::FMODE, stdio};
use std::sync::atomic::Ordering;

pub extern "win64" fn set(mode: i32) -> i32 {
    if !matches!(mode, 0x4000 | 0x8000 | 0x10000) {
        unsafe { *stdio::errno() = 22; }
        return 22;
    }
    FMODE.store(mode, Ordering::Relaxed);
    0
}
pub unsafe extern "win64" fn get(out: *mut i32) -> i32 {
    if out.is_null() {
        unsafe { *stdio::errno() = 22; }
        return 22;
    }
    unsafe { out.write(FMODE.load(Ordering::Relaxed)); }
    0
}
pub extern "win64" fn pointer() -> *mut i32 {
    FMODE.as_ptr()
}

/// Existing CRT commit-mode storage. Automatic commit-on-flush is not implemented.
pub extern "win64" fn commode_pointer() -> *mut i32 {
    super::startup::COMMODE.as_ptr()
}
