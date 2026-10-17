//! No application menus; X11 window decorations belong to the window manager.
use super::state;
use crate::api::system::error;
pub extern "win64" fn system_menu(hwnd: usize, _revert: i32) -> usize {
    error::set(if state::exists(hwnd) {50} else {1400});
    0
}
pub extern "win64" fn menu(hwnd: usize) -> usize {
    if !state::exists(hwnd) {error::set(1400);}
    0
}
pub extern "win64" fn enable(_menu: usize, _item: u32, _flags: u32) -> u32 {
    error::set(1401); // ERROR_INVALID_MENU_HANDLE: no menu handles are allocated.
    u32::MAX
}
