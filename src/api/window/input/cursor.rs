//! Cursor input implementation and owned state.
use super::{fail, raw, trace_input};
use crate::api::window::{
    state::{connection, windows},
    types::*,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI32, AtomicUsize, Ordering},
        Mutex,
    },
};

// ---- pointer and cursors --------------------------------------------------------
pub(crate) unsafe extern "win64" fn get_cursor_pos(out: *mut Point) -> i32 {
    crate::diagnostics::unity_input::sample();
    if out.is_null() {
        return fail(87, 0);
    }
    let (x, y) = match connection() {
        Ok(c) => {
            let (x, y, _) = c.pointer();
            (x, y)
        }
        Err(e) => return fail(e, 0),
    };
    static LAST: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);
    let point = (x as u32 as u64 | ((y as u32 as u64) << 32)) as i64;
    if LAST.swap(point, Ordering::Relaxed) != point {
        trace_input(format_args!("GetCursorPos {x},{y}"));
    }
    unsafe { out.write(Point { x, y }) };
    1
}
pub(crate) unsafe extern "win64" fn get_physical_cursor_pos(out: *mut Point) -> i32 {
    unsafe { get_cursor_pos(out) }
}
pub(crate) extern "win64" fn set_cursor_pos(x: i32, y: i32) -> i32 {
    match connection() {
        Ok(c) => {
            c.warp(x, y);
            raw::warped((x, y));
            1
        }
        Err(e) => fail(e, 0),
    }
}
static CLIP: Mutex<Option<Rect>> = Mutex::new(None);
/// Grabs the pointer for the clip rectangle `r`: confined to the program window the
/// rectangle lies in, or released when it covers the whole virtual screen. False if
/// the X grab failed (another client's grab, or the window is not viewable yet).
fn apply_clip(r: Rect) -> bool {
    let Ok(c) = connection() else {
        return false;
    };
    let screen = crate::api::window::display::virtual_rect();
    if r.left <= screen[0] && r.top <= screen[1] && r.right >= screen[2] && r.bottom >= screen[3] {
        c.confine(None);
        return true;
    }
    let candidates: Vec<(usize, u64)> = windows()
        .iter()
        .filter(|(_, w)| w.native != 0 && w.visible && w.parent == 0)
        .map(|(&h, w)| (h, w.native))
        .collect();
    let target = candidates.into_iter().find(|&(h, _)| {
        crate::api::window::windows::screen_rect(h).is_some_and(|w| {
            r.left >= w.left - 1
                && r.top >= w.top - 1
                && r.right <= w.right + 1
                && r.bottom <= w.bottom + 1
        })
    });
    match target {
        Some((_, native)) => {
            let grabbed = c.confine(Some(native));
            trace_input(format_args!("ClipCursor grab {}", if grabbed { "ok" } else { "failed, retried on activation" }));
            grabbed
        }
        None => true,
    }
}
/// ClipCursor: confines the pointer to the program window the rectangle lies in;
/// NULL releases it. The rectangle is kept while the program is inactive.
pub(crate) unsafe extern "win64" fn clip_cursor(rect: *const Rect) -> i32 {
    if rect.is_null() {
        *CLIP.lock().unwrap() = None;
        if let Ok(c) = connection() {
            c.confine(None);
        }
        return 1;
    }
    let r = unsafe { *rect };
    *CLIP.lock().unwrap() = Some(r);
    apply_clip(r);
    1
}
/// During camera capture — the cursor is hidden (ShowCursor < 0) and confined by
/// ClipCursor — keep the pointer near the centre of the clip rectangle so relative
/// (raw) motion never stops at a screen or window edge, as it would on Windows where
/// the device keeps reporting deltas. Called after a raw motion with the pointer at
/// (`x`, `y`) in screen coordinates; returns the new centre when it warped there, so
/// the raw delta anchor can follow. Only warps once the pointer leaves the central
/// half, to keep warps rare.
pub(super) fn recenter_captured(x: i32, y: i32) -> Option<(i32, i32)> {
    if SHOW.load(Ordering::Acquire) >= 0 {
        return None;
    }
    let r = (*CLIP.lock().unwrap())?;
    let (cx, cy) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    if (x - cx).abs() * 4 <= (r.right - r.left) && (y - cy).abs() * 4 <= (r.bottom - r.top) {
        return None; // still well inside: no warp needed yet
    }
    let c = connection().ok()?;
    c.warp(cx, cy);
    Some((cx, cy))
}
/// Focus left the program: release the X grab so other applications get the pointer
/// (the clip rectangle stays recorded, as for Windows' foreground switch).
pub(in crate::api::window) fn suspend_clip() {
    if CLIP.lock().unwrap().is_some() {
        if let Ok(c) = connection() {
            c.confine(None);
        }
    }
}
/// Focus came back: grab again for the recorded clip rectangle (also retries a grab
/// that failed while the window manager held its own, e.g. during Alt+Tab).
pub(in crate::api::window) fn resume_clip() {
    let clip = *CLIP.lock().unwrap();
    if let Some(r) = clip {
        apply_clip(r);
    }
}
pub(crate) unsafe extern "win64" fn get_clip_cursor(out: *mut Rect) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let r = CLIP.lock().unwrap().unwrap_or_else(|| {
        let [left, top, right, bottom] = crate::api::window::display::virtual_rect();
        Rect {
            left,
            top,
            right,
            bottom,
        }
    });
    unsafe { out.write(r) };
    1
}
/// Cursor handles: system cursors (IDC_* ids) as X font cursors, CreateCursor
/// planes as X bitmap cursors; cursors from program resources show the arrow.
/// Cursor handle -> X cursor.
static CURSORS: Mutex<Option<HashMap<usize, u64>>> = Mutex::new(None);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static SHOW: AtomicI32 = AtomicI32::new(0);
const CURSOR_BASE: usize = 0x0006_0000;
/// The cursorfont shape for an IDC_* id.
fn shape(id: u32) -> u32 {
    match id {
        32513 => 152,         // IBEAM: xterm
        32514 | 32650 => 150, // WAIT, APPSTARTING: watch
        32515 => 34,          // CROSS: crosshair
        32516 => 22,          // UPARROW: center_ptr
        32642 => 14,          // SIZENWSE: bottom_right_corner
        32643 => 12,          // SIZENESW: bottom_left_corner
        32644 => 108,         // SIZEWE: sb_h_double_arrow
        32645 => 116,         // SIZENS: sb_v_double_arrow
        32646 => 52,          // SIZEALL: fleur
        32648 => 0,           // NO: X_cursor
        32649 => 60,          // HAND: hand2
        32651 => 92,          // HELP: question_arrow
        _ => 68,              // ARROW: left_ptr
    }
}
fn cursor_handle(id: u32) -> Result<usize, u32> {
    let c = connection()?;
    let handle = CURSOR_BASE + id as usize;
    let mut cursors = CURSORS.lock().unwrap();
    let map = cursors.get_or_insert_with(HashMap::new);
    if let std::collections::hash_map::Entry::Vacant(e) = map.entry(handle) {
        let native = c.font_cursor(shape(id))?;
        e.insert(native);
    }
    Ok(handle)
}
/// The X cursor a cursor handle shows (0: the X default).
pub(in crate::api::window) fn x_cursor_for(handle: usize) -> u64 {
    CURSORS
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|m| m.get(&handle))
        .copied()
        .unwrap_or(0)
}
fn load_cursor(instance: usize, id: usize) -> usize {
    if id >= 65536 || instance != 0 {
        // A named or program resource cursor: shown as the arrow.
        static WARNED: std::sync::Once = std::sync::Once::new();
        WARNED.call_once(|| eprintln!("LoadCursor: program cursors show the arrow"));
        return cursor_handle(32512).unwrap_or_else(|e| fail(e, 0));
    }
    cursor_handle(id as u32).unwrap_or_else(|e| fail(e, 0))
}
pub(crate) extern "win64" fn load_cursor_w(instance: usize, name: *const u16) -> usize {
    load_cursor(instance, name as usize)
}
pub(crate) extern "win64" fn load_cursor_a(instance: usize, name: *const i8) -> usize {
    load_cursor(instance, name as usize)
}
pub(crate) extern "win64" fn load_cursor_from_file_w(_name: *const u16) -> usize {
    cursor_handle(32512).unwrap_or_else(|e| fail(e, 0))
}
pub(crate) extern "win64" fn load_cursor_from_file_a(_name: *const i8) -> usize {
    cursor_handle(32512).unwrap_or_else(|e| fail(e, 0))
}
/// Applies the current cursor (or the blank one while hidden) to every window.
fn apply_cursor() {
    let Ok(c) = connection() else {
        return;
    };
    let current = CURRENT.load(Ordering::Acquire);
    let native_cursor = if SHOW.load(Ordering::Acquire) < 0 || current == 0 {
        c.blank_cursor()
    } else {
        x_cursor_for(current)
    };
    let natives: Vec<u64> = windows()
        .values()
        .filter(|w| w.native != 0)
        .map(|w| w.native)
        .collect();
    for n in natives {
        c.define_cursor(n, native_cursor);
    }
}
pub(in crate::api::window) fn set_cursor(handle: usize) -> usize {
    let previous = CURRENT.swap(handle, Ordering::AcqRel);
    if previous != handle {
        apply_cursor();
    }
    previous
}
pub(crate) extern "win64" fn set_cursor_w(handle: usize) -> usize {
    set_cursor(handle)
}
pub(crate) extern "win64" fn get_cursor() -> usize {
    CURRENT.load(Ordering::Acquire)
}
/// ShowCursor: the display counter; the cursor shows while it is >= 0.
pub(crate) extern "win64" fn show_cursor(show: i32) -> i32 {
    let count = if show != 0 {
        SHOW.fetch_add(1, Ordering::AcqRel) + 1
    } else {
        SHOW.fetch_sub(1, Ordering::AcqRel) - 1
    };
    if count == 0 || count == -1 {
        apply_cursor();
    }
    count
}
/// CURSORINFO: cbSize, flags (CURSOR_SHOWING 1), hCursor, ptScreenPos.
pub(crate) unsafe extern "win64" fn get_cursor_info(out: *mut u8) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let mut p = Point::default();
    unsafe {
        get_cursor_pos(&mut p);
        out.add(4)
            .cast::<u32>()
            .write_unaligned((SHOW.load(Ordering::Acquire) >= 0) as u32);
        out.add(8).cast::<usize>().write_unaligned(get_cursor());
        out.add(16).cast::<Point>().write_unaligned(p);
    }
    1
}
/// CreateCursor: monochrome AND/XOR planes (rows WORD-aligned, MSB first) as an X
/// bitmap cursor. AND 1/XOR 0 is transparent, XOR 1 white, both 0 black; X cannot
/// invert the screen, so AND 1/XOR 1 pixels show black.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe extern "win64" fn create_cursor(
    _instance: usize,
    hot_x: i32,
    hot_y: i32,
    width: i32,
    height: i32,
    and_plane: *const u8,
    xor_plane: *const u8,
) -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(CURSOR_BASE + 0x1_0000);
    if and_plane.is_null() || xor_plane.is_null() || !(1..=256).contains(&width) || !(1..=256).contains(&height)
        || !(0..width).contains(&hot_x) || !(0..height).contains(&hot_y)
    {
        return fail(87, 0);
    }
    let (w, h) = (width as usize, height as usize);
    let stride = w.div_ceil(16) * 2;
    let x_stride = w.div_ceil(8);
    let (and, xor) = unsafe {
        (
            std::slice::from_raw_parts(and_plane, stride * h),
            std::slice::from_raw_parts(xor_plane, stride * h),
        )
    };
    let mut source = vec![0u8; x_stride * h];
    let mut mask = vec![0u8; x_stride * h];
    for y in 0..h {
        for x in 0..w {
            let bit = |plane: &[u8]| plane[y * stride + x / 8] & (0x80 >> (x % 8)) != 0;
            let (a, x_bit) = (bit(and), bit(xor));
            let at = y * x_stride + x / 8;
            if !a || x_bit {
                mask[at] |= 1 << (x % 8);
            }
            if x_bit && !a {
                source[at] |= 1 << (x % 8);
            }
        }
    }
    let native = match connection().and_then(|c| {
        c.bitmap_cursor(width as u32, height as u32, &source, &mask, hot_x as u32, hot_y as u32)
    }) {
        Ok(native) => native,
        Err(code) => return fail(code, 0),
    };
    let handle = NEXT.fetch_add(4, Ordering::Relaxed);
    CURSORS.lock().unwrap().get_or_insert_with(HashMap::new).insert(handle, native);
    handle
}
pub(crate) extern "win64" fn destroy_cursor(_handle: usize) -> i32 {
    1
}
