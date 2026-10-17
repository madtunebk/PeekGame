//! Focus input implementation and owned state.
use super::{fail, keyboard::release_keys, raw, trace_input};
use crate::api::window::{
    queue,
    state::{self, connection, windows},
    types::*,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

// ---- focus, activation, capture -------------------------------------------------
static FOCUS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static CAPTURE: AtomicUsize = AtomicUsize::new(0);
static ACTIVATE_ON_MAP: Mutex<Vec<usize>> = Mutex::new(Vec::new());
pub(in crate::api::window) fn capture() -> usize {
    CAPTURE.load(Ordering::Acquire)
}
pub(in crate::api::window) fn focus() -> usize {
    FOCUS.load(Ordering::Acquire)
}
fn top_level(hwnd: usize) -> usize {
    let windows = windows();
    let mut current = hwnd;
    while let Some(p) = windows.get(&current).map(|w| w.parent).filter(|&p| p != 0) {
        current = p;
    }
    current
}
/// A shown window asks the window manager for focus once it is mapped (X refuses
/// focus for windows not yet viewable).
pub(in crate::api::window) fn activate_on_map(hwnd: usize) {
    ACTIVATE_ON_MAP.lock().unwrap().push(hwnd);
}
pub(in crate::api::window) fn mapped(hwnd: usize) {
    let wanted = {
        let mut list = ACTIVATE_ON_MAP.lock().unwrap();
        let wanted = list.contains(&hwnd);
        list.retain(|&h| h != hwnd);
        wanted
    };
    if wanted {
        if let (Ok(c), Some(native)) = (connection(), windows().get(&hwnd).map(|w| w.native)) {
            c.raise_and_focus(native);
        }
    }
}
/// The X server moved keyboard focus: WM_ACTIVATEAPP, WM_ACTIVATE, WM_SETFOCUS /
/// WM_KILLFOCUS, as Windows sends them (to the owner threads).
pub(in crate::api::window) fn focus_changed(hwnd: usize, gained: bool) {
    trace_input(format_args!("focus hwnd={hwnd:#x} gained={gained}"));
    let top = top_level(hwnd);
    if gained {
        let previous = FOCUS.swap(hwnd, Ordering::AcqRel);
        let previous_active = ACTIVE.swap(top, Ordering::AcqRel);
        if previous == hwnd {
            return;
        }
        // Focus on a window still marked minimized is its restore, as on Windows. The
        // window manager may have ignored the iconify request (Mutter keeps fullscreen
        // windows mapped), so no map event would ever clear it: games that minimize
        // themselves on deactivation then wait for the restore forever (DOOM Eternal
        // re-registers raw keyboard/mouse only after it).
        let restored = state::with_window(top, |w| {
            std::mem::replace(&mut w.minimized, false).then_some((w.width, w.height))
        });
        if let Ok(Some((width, height))) = restored {
            trace_input(format_args!("focus restores minimized hwnd={top:#x}"));
            queue::notify_window(top, WM_SIZE, 0, pack(width, height));
        }
        if previous_active == 0 {
            queue::notify_window(top, WM_ACTIVATEAPP, 1, 0);
        }
        if previous_active != top {
            queue::notify_window(top, WM_NCACTIVATE, 1, 0);
            queue::notify_window(top, WM_ACTIVATE, 1, previous_active as isize);
        }
        queue::notify_window(hwnd, WM_SETFOCUS, previous, 0);
        super::cursor::resume_clip();
    } else {
        if FOCUS
            .compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        ACTIVE.store(0, Ordering::Release);
        release_keys();
        super::cursor::suspend_clip();
        queue::notify_window(hwnd, WM_KILLFOCUS, 0, 0);
        queue::notify_window(top, WM_NCACTIVATE, 0, 0);
        queue::notify_window(top, WM_ACTIVATE, 0, 0);
        queue::notify_window(top, WM_ACTIVATEAPP, 0, 0);
    }
}
pub(in crate::api::window) fn forget_window(hwnd: usize) {
    let _ = FOCUS.compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire);
    let _ = ACTIVE.compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire);
    let _ = CAPTURE.compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire);
    ACTIVATE_ON_MAP.lock().unwrap().retain(|&h| h != hwnd);
    raw::forget_window(hwnd);
}
pub(crate) extern "win64" fn get_focus() -> usize {
    let f = focus();
    if f != 0 && queue::owner(f) == Some(state::current_thread()) {
        f
    } else {
        0
    }
}
pub(crate) extern "win64" fn set_focus(hwnd: usize) -> usize {
    let previous = focus();
    if hwnd == 0 {
        return previous;
    }
    let Some((native, visible)) = windows().get(&hwnd).map(|w| (w.native, w.visible)) else {
        return fail(ERROR_WINDOW, 0);
    };
    if native != 0 && visible {
        if let Ok(c) = connection() {
            c.focus(native);
        }
    }
    previous
}
pub(crate) extern "win64" fn get_active_window() -> usize {
    let a = ACTIVE.load(Ordering::Acquire);
    if a != 0 && queue::owner(a) == Some(state::current_thread()) {
        a
    } else {
        0
    }
}
pub(crate) extern "win64" fn get_foreground_window() -> usize {
    ACTIVE.load(Ordering::Acquire)
}
pub(crate) extern "win64" fn set_foreground_window(hwnd: usize) -> i32 {
    let Some(native) = windows().get(&hwnd).map(|w| w.native) else {
        return 0;
    };
    if native != 0 {
        if let Ok(c) = connection() {
            c.raise_and_focus(native);
        }
    }
    1
}
pub(crate) extern "win64" fn set_active_window(hwnd: usize) -> usize {
    let previous = ACTIVE.load(Ordering::Acquire);
    set_foreground_window(hwnd);
    previous
}
pub(crate) extern "win64" fn allow_set_foreground_window(_process: u32) -> i32 {
    1
}
pub(crate) extern "win64" fn lock_set_foreground_window(_code: u32) -> i32 {
    1
}
/// SetCapture: mouse input goes to this window until ReleaseCapture (X's implicit
/// grab already keeps a dragging pointer with the window).
pub(crate) extern "win64" fn set_capture(hwnd: usize) -> usize {
    if !state::exists(hwnd) {
        return fail(ERROR_WINDOW, 0);
    }
    CAPTURE.swap(hwnd, Ordering::AcqRel)
}
pub(crate) extern "win64" fn release_capture() -> i32 {
    CAPTURE.store(0, Ordering::Release);
    1
}
pub(crate) extern "win64" fn get_capture() -> usize {
    capture()
}
/// TRACKMOUSEEVENT: cbSize, dwFlags (TME_LEAVE 2, TME_CANCEL 0x80000000), hwnd, hover.
pub(crate) unsafe extern "win64" fn track_mouse_event(event: *const u8) -> i32 {
    if event.is_null() {
        return fail(87, 0);
    }
    let flags = unsafe { event.add(4).cast::<u32>().read_unaligned() };
    let hwnd = unsafe { event.add(8).cast::<usize>().read_unaligned() };
    if flags & 2 != 0 {
        let cancel = flags & 0x8000_0000 != 0;
        if state::with_window(hwnd, |w| w.track_leave = !cancel).is_err() {
            return fail(ERROR_WINDOW, 0);
        }
    }
    1
}
