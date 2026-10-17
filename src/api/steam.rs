//! Windows steamclient64.dll exports backed by the real Linux Steam client.
//! The native bridge owns ABI translation and remains pinned for process lifetime.
use std::{ffi::{c_char, c_void, CString}, sync::atomic::{AtomicBool, Ordering}};
unsafe extern "C" {
    fn steam_bridge_open(path: *const c_char) -> i32;
    fn steam_bridge_export(name: *const c_char) -> *const c_void;
    fn steam_bridge_steam_pid() -> u32;
}

pub(crate) fn steam_pid() -> u32 {
    unsafe { steam_bridge_steam_pid() }
}
static LOADED: AtomicBool = AtomicBool::new(false);
pub(crate) fn recognizes(name: &str) -> bool {
    let clean = name.trim_end_matches('.');
    let file = clean.rsplit(|c| c == '/' || c == '\\').next().unwrap_or(clean);
    file.eq_ignore_ascii_case("steamclient64.dll") || file.eq_ignore_ascii_case("steamclient64")
}
pub(crate) fn loaded() -> bool { LOADED.load(Ordering::Acquire) }
pub(crate) fn open() -> bool {
    // Failed attempts can be retried. C serializes initialization and dlopen.
    if unsafe { steam_bridge_open(std::ptr::null()) } != 0 { return false; }
    LOADED.store(true, Ordering::Release);
    true
}
pub(crate) fn export(name: &str) -> Option<usize> {
    if !loaded() { return None; }
    let name = CString::new(name).ok()?;
    let p = unsafe { steam_bridge_export(name.as_ptr()) };
    (!p.is_null()).then_some(p as usize)
}
