use super::{state::STATE, types::*};
use crate::api::system::error;
use std::sync::atomic::Ordering;
pub(crate) extern "win64" fn show_window(hwnd: usize, command: i32) -> i32 {
    if ![0, 1, 5, 8].contains(&command) {
        error::set(50);
        return 0;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(window) = state.windows.get_mut(&hwnd) else {
            error::set(ERROR_WINDOW);
            return 0;
        };
        let old = window.visible;
        window.visible = command != 0;
        window
            .surface
            .raw
            .visible
            .store(window.visible as u32, Ordering::Release);
        let (native, visible) = (window.native, window.visible);
        state.connection.as_ref().unwrap().show(native, visible);
        old as i32
    })
}
pub(crate) unsafe extern "win64" fn adjust_window_rect_ex(
    rect: *mut Rect,
    style: u32,
    menu: i32,
    ex_style: u32,
) -> i32 {
    if rect.is_null() {
        error::set(87);
        return 0;
    }
    if menu != 0 || ex_style != 0 || style & !0x10cf0000 != 0 {
        error::set(50);
        return 0;
    }
    // Explicit client-size model: the Linux WM owns external decorations.
    // The same unchanged size is used by CreateWindowExW for its client window.
    1
}
pub(crate) extern "win64" fn destroy_window(hwnd: usize) -> i32 {
    let details = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let window = state.windows.get_mut(&hwnd)?;
        if window.destroying {
            return None;
        }
        window.destroying = true;
        window.surface.raw.alive.store(0, Ordering::Release);
        Some((window.procedure, window.native))
    });
    let Some((procedure, native)) = details else {
        error::set(ERROR_WINDOW);
        return 0;
    };
    unsafe {
        procedure(hwnd, WM_DESTROY, 0, 0);
        procedure(hwnd, WM_NCDESTROY, 0, 0);
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.windows.remove(&hwnd);
        state.queue.retain(|m| m.window != hwnd);
        state.connection.as_ref().unwrap().destroy(native);
    });
    1
}
pub(crate) extern "win64" fn def_window_proc_w(
    hwnd: usize,
    message: u32,
    _wparam: usize,
    _lparam: isize,
) -> isize {
    match message {
        WM_NCCREATE => 1,
        WM_CLOSE => {
            destroy_window(hwnd);
            0
        }
        _ => 0,
    }
}
