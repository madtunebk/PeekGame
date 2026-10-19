//! Stable input entry points; state and implementations live in focused modules.
use crate::api::system::error;
use std::sync::atomic::Ordering;
mod cursor;
mod focus;
mod keyboard;
mod raw;
pub(crate) mod xinput;
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
pub(crate) use cursor::{
    create_cursor,
    clip_cursor, destroy_cursor, get_clip_cursor, get_cursor, get_cursor_info, get_cursor_pos,
    get_physical_cursor_pos, load_cursor_a, load_cursor_w, load_cursor_from_file_a, load_cursor_from_file_w, set_cursor_pos, set_cursor_w,
    show_cursor,
};
pub(super) use cursor::{set_cursor, x_cursor_for};
pub(super) use focus::{activate_on_map, capture, focus_changed, forget_window, mapped};
pub(crate) use focus::{
    allow_set_foreground_window, get_active_window, get_capture, get_focus, get_foreground_window,
    lock_set_foreground_window, release_capture, set_active_window, set_capture, set_focus,
    set_foreground_window, track_mouse_event,
};
pub(crate) use keyboard::{
    activate_keyboard_layout, get_async_key_state, get_key_name_text_w, get_key_state,
    get_keyboard_layout, get_keyboard_layout_list, get_keyboard_layout_name_w, get_keyboard_state,
    map_virtual_key_ex_w, map_virtual_key_w, to_unicode, to_unicode_ex,
};
pub(crate) use keyboard::key_down;
pub(super) use keyboard::{set_key, sync_toggles};
pub(crate) use raw::{
    define_raw_input_proc, get_raw_input_buffer, get_raw_input_data, get_raw_input_device_info_a,
    get_raw_input_device_info_w, get_raw_input_device_list, get_registered_raw_input_devices,
    register_raw_input_devices,
};
pub(super) use raw::{
    is_pending_mouse_motion, merge_raw_mouse, raw_keyboard, raw_motion_core, raw_mouse,
    raw_mouse_relative, raw_read,
};

pub(super) fn trace_input(message: impl std::fmt::Display) {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if *ENABLED.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_INPUT").is_some())
        && COUNT.fetch_add(1, Ordering::Relaxed) < 200
    {
        eprintln!("INPUT: {message}");
    }
}

pub(crate) unsafe extern "win64" fn send_input(
    n_inputs: u32,
    inputs: *const std::ffi::c_void,
    cb_size: i32,
) -> u32 {
    if n_inputs == 0 || inputs.is_null() || cb_size < 40 {
        return 0;
    }
    n_inputs
}

pub(crate) extern "win64" fn get_touch_input_info(
    _touch_input: usize,
    _c_inputs: u32,
    _p_inputs: *mut std::ffi::c_void,
    _cb_size: i32,
) -> i32 {
    fail(6, 0)
}

pub(crate) extern "win64" fn close_touch_input_handle(_touch_input: usize) -> i32 {
    fail(6, 0)
}

pub(crate) extern "win64" fn attach_thread_input(
    id_attach: u32,
    id_attach_to: u32,
    _attach: i32,
) -> i32 {
    if id_attach == id_attach_to {
        fail(87, 0)
    } else {
        1
    }
}



/// PEEKGAME_TRACE_INPUT: an X map/unmap of a program window and its minimized state.
pub(in crate::api::window) fn trace_map(hwnd: usize, mapped: bool) {
    let minimized = super::state::with_window(hwnd, |w| w.minimized).unwrap_or(false);
    trace_input(format_args!("X {} hwnd={hwnd:#x} minimized={minimized}", if mapped { "Map" } else { "Unmap" }));
}
