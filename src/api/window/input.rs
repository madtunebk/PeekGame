//! Keyboard and mouse state, focus, capture, cursors and raw input.
use super::{
    queue,
    state::{self, connection, windows},
    strings,
    types::*,
};
use crate::{api::system::error, host::x11::keyboard};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicI32, AtomicUsize, Ordering},
        Mutex,
    },
};
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}

// ---- key state ------------------------------------------------------------------
/// Per virtual key: 0x80 down, 0x01 toggled.
static KEYS: Mutex<[u8; 256]> = Mutex::new([0; 256]);
pub(super) fn set_key(vk: u8, down: bool) {
    let mut keys = KEYS.lock().unwrap();
    if down && keys[vk as usize] & 0x80 == 0 {
        keys[vk as usize] ^= 1;
    }
    keys[vk as usize] = (keys[vk as usize] & 1) | if down { 0x80 } else { 0 };
    // The generic modifier keys follow their left/right keys.
    for (generic, left, right) in [(0x10, 0xa0, 0xa1), (0x11, 0xa2, 0xa3), (0x12, 0xa4, 0xa5)] {
        let any = (keys[left] | keys[right]) & 0x80;
        keys[generic] = (keys[generic] & 1) | any;
    }
}
/// Caps Lock and Num Lock toggles from the X modifier state (LockMask, Mod2Mask).
pub(super) fn sync_toggles(state: u32) {
    let mut keys = KEYS.lock().unwrap();
    for (vk, on) in [(0x14usize, state & 2 != 0), (0x90, state & 0x10 != 0)] {
        keys[vk] = (keys[vk] & 0x80) | on as u8;
    }
}
/// Every key up (focus left the program: no stuck keys).
pub(super) fn release_keys() {
    let mut keys = KEYS.lock().unwrap();
    for k in keys.iter_mut() {
        *k &= 1;
    }
}
pub(super) fn key_down(vk: u8) -> bool {
    KEYS.lock().unwrap()[vk as usize] & 0x80 != 0
}
pub(crate) extern "win64" fn get_key_state(vk: i32) -> i16 {
    let k = KEYS.lock().unwrap()[(vk & 0xff) as usize];
    ((if k & 0x80 != 0 { 0x8000u16 } else { 0 }) | (k & 1) as u16) as i16
}
pub(crate) extern "win64" fn get_async_key_state(vk: i32) -> i16 {
    if KEYS.lock().unwrap()[(vk & 0xff) as usize] & 0x80 != 0 {
        0x8000u16 as i16
    } else {
        0
    }
}
pub(crate) unsafe extern "win64" fn get_keyboard_state(out: *mut u8) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let keys = *KEYS.lock().unwrap();
    unsafe { std::ptr::copy_nonoverlapping(keys.as_ptr(), out, 256) };
    1
}
/// The keyboard layout: the user's language (HKL = language in both words).
pub(crate) extern "win64" fn get_keyboard_layout(_thread: u32) -> usize {
    let id = crate::api::system::locale::langid() as usize;
    id << 16 | id
}
pub(crate) unsafe extern "win64" fn get_keyboard_layout_name_w(out: *mut u16) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let name = format!("{:08X}", crate::api::system::locale::langid());
    unsafe { strings::copy_w(&name, out, 9) };
    1
}
pub(crate) unsafe extern "win64" fn get_keyboard_layout_list(count: i32, out: *mut usize) -> i32 {
    if count > 0 && !out.is_null() {
        unsafe { *out = get_keyboard_layout(0) };
    }
    1
}
pub(crate) extern "win64" fn activate_keyboard_layout(_layout: usize, _flags: u32) -> usize {
    get_keyboard_layout(0)
}
/// MapVirtualKey: VK_TO_VSC 0, VSC_TO_VK 1, VK_TO_CHAR 2, VSC_TO_VK_EX 3,
/// VK_TO_VSC_EX 4.
pub(crate) extern "win64" fn map_virtual_key_ex_w(code: u32, kind: u32, _layout: usize) -> u32 {
    match kind {
        0 => (keyboard::scan_of_vk(code as u8) & 0xff) as u32,
        4 => {
            let s = keyboard::scan_of_vk(code as u8);
            if s & 0xe000 != 0 {
                0xe000 | (s & 0xff) as u32
            } else {
                s as u32
            }
        }
        1 | 3 => {
            let scan = if code & 0xe000 == 0xe000 {
                0xe000 | (code & 0xff) as u16
            } else {
                (code & 0xff) as u16
            };
            keyboard::positional_vk(scan, kind == 1) as u32
        }
        2 => {
            // The unshifted character, uppercase for letters.
            let scan = keyboard::scan_of_vk(code as u8);
            match character_for(scan, false, false) {
                Some(c) if (b'a' as u32..=b'z' as u32).contains(&c) => c - 32,
                Some(c) => c,
                None => 0,
            }
        }
        _ => 0,
    }
}
pub(crate) extern "win64" fn map_virtual_key_w(code: u32, kind: u32) -> u32 {
    map_virtual_key_ex_w(code, kind, 0)
}
/// The character a key types with the current X layout.
fn character_for(scan: u16, shift: bool, altgr: bool) -> Option<u32> {
    let c = connection().ok()?;
    let keycode = keyboard::keycode(scan)?;
    let level = if altgr { 2 } else { shift as i32 };
    keyboard::character(c.keysym(keycode, level))
}
/// ToUnicodeEx: the characters a key types with `state` (Shift, Caps Lock, Ctrl,
/// AltGr = Ctrl+Alt) under the X layout. Dead keys are not composed.
pub(crate) unsafe extern "win64" fn to_unicode_ex(
    vk: u32,
    scan: u32,
    state: *const u8,
    out: *mut u16,
    capacity: i32,
    _flags: u32,
    _layout: usize,
) -> i32 {
    if state.is_null() || out.is_null() || capacity <= 0 {
        return 0;
    }
    let keys = unsafe { std::slice::from_raw_parts(state, 256) };
    if scan & 0x8000 != 0 {
        return 0; // key up
    }
    let scan_code = if scan & 0xff == 0 {
        keyboard::scan_of_vk(vk as u8)
    } else {
        (scan & 0xff) as u16 | if scan & 0xe000 == 0xe000 { 0xe000 } else { 0 }
    };
    let shift = keys[0x10] & 0x80 != 0;
    let ctrl = keys[0x11] & 0x80 != 0;
    let alt = keys[0x12] & 0x80 != 0;
    let caps = keys[0x14] & 1 != 0;
    let altgr = ctrl && alt;
    let Some(mut c) = character_for(scan_code, shift, altgr) else {
        return 0;
    };
    if caps && !altgr {
        if let Some(ch) = char::from_u32(c) {
            if ch.is_alphabetic() {
                // Caps Lock inverts Shift for letters.
                if let Some(other) = character_for(scan_code, !shift, false) {
                    c = other;
                }
            }
        }
    }
    if ctrl && !alt {
        // Ctrl+letter: the control character.
        match c {
            0x61..=0x7a => c -= 0x60,
            0x41..=0x5a => c -= 0x40,
            _ => return 0,
        }
    }
    let mut units = [0u16; 2];
    let encoded = char::from_u32(c)
        .map(|ch| ch.encode_utf16(&mut units).len())
        .unwrap_or(0);
    let n = encoded.min(capacity as usize);
    unsafe { std::ptr::copy_nonoverlapping(units.as_ptr(), out, n) };
    n as i32
}
pub(crate) unsafe extern "win64" fn to_unicode(
    vk: u32,
    scan: u32,
    state: *const u8,
    out: *mut u16,
    capacity: i32,
    flags: u32,
) -> i32 {
    unsafe { to_unicode_ex(vk, scan, state, out, capacity, flags, 0) }
}
/// GetKeyNameTextW: lParam's scan code (bits 16-23) and extended bit (24).
pub(crate) unsafe extern "win64" fn get_key_name_text_w(
    lparam: i32,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    let scan = ((lparam >> 16) & 0xff) as u16 | if lparam & (1 << 24) != 0 { 0xe000 } else { 0 };
    let name = keyboard::name(scan, character_for(scan, false, false));
    match name {
        Some(n) => unsafe { strings::copy_w(&n, out, capacity) },
        None => 0,
    }
}

// ---- focus, activation, capture -------------------------------------------------
static FOCUS: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static CAPTURE: AtomicUsize = AtomicUsize::new(0);
static ACTIVATE_ON_MAP: Mutex<Vec<usize>> = Mutex::new(Vec::new());
pub(super) fn capture() -> usize {
    CAPTURE.load(Ordering::Acquire)
}
pub(super) fn focus() -> usize {
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
pub(super) fn activate_on_map(hwnd: usize) {
    ACTIVATE_ON_MAP.lock().unwrap().push(hwnd);
}
pub(super) fn mapped(hwnd: usize) {
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
pub(super) fn focus_changed(hwnd: usize, gained: bool) {
    let top = top_level(hwnd);
    if gained {
        let previous = FOCUS.swap(hwnd, Ordering::AcqRel);
        let previous_active = ACTIVE.swap(top, Ordering::AcqRel);
        if previous == hwnd {
            return;
        }
        if previous_active == 0 {
            queue::notify_window(top, WM_ACTIVATEAPP, 1, 0);
        }
        if previous_active != top {
            queue::notify_window(top, WM_NCACTIVATE, 1, 0);
            queue::notify_window(top, WM_ACTIVATE, 1, previous_active as isize);
        }
        queue::notify_window(hwnd, WM_SETFOCUS, previous, 0);
    } else {
        if FOCUS
            .compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        ACTIVE.store(0, Ordering::Release);
        release_keys();
        queue::notify_window(hwnd, WM_KILLFOCUS, 0, 0);
        queue::notify_window(top, WM_NCACTIVATE, 0, 0);
        queue::notify_window(top, WM_ACTIVATE, 0, 0);
        queue::notify_window(top, WM_ACTIVATEAPP, 0, 0);
    }
}
pub(super) fn forget_window(hwnd: usize) {
    let _ = FOCUS.compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire);
    let _ = ACTIVE.compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire);
    let _ = CAPTURE.compare_exchange(hwnd, 0, Ordering::AcqRel, Ordering::Acquire);
    ACTIVATE_ON_MAP.lock().unwrap().retain(|&h| h != hwnd);
    RAW.lock().unwrap().devices.retain(|d| d.target != hwnd);
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

// ---- pointer and cursors --------------------------------------------------------
pub(crate) unsafe extern "win64" fn get_cursor_pos(out: *mut Point) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let (x, y) = match connection() {
        Ok(c) => {
            let (x, y, _) = c.pointer();
            (x, y)
        }
        Err(_) => (0, 0),
    };
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
            RAW.lock().unwrap().last = Some((x, y));
            1
        }
        Err(e) => fail(e, 0),
    }
}
static CLIP: Mutex<Option<Rect>> = Mutex::new(None);
/// ClipCursor: confines the pointer to the program window the rectangle lies in;
/// NULL releases it.
pub(crate) unsafe extern "win64" fn clip_cursor(rect: *const Rect) -> i32 {
    let Ok(c) = connection() else {
        return 1;
    };
    if rect.is_null() {
        *CLIP.lock().unwrap() = None;
        c.confine(None);
        return 1;
    }
    let r = unsafe { *rect };
    *CLIP.lock().unwrap() = Some(r);
    let screen = crate::host::x11::screen_size().unwrap_or((0, 0));
    if r.left <= 0 && r.top <= 0 && r.right >= screen.0 && r.bottom >= screen.1 {
        c.confine(None);
        return 1;
    }
    let candidates: Vec<(usize, u64)> = windows()
        .iter()
        .filter(|(_, w)| w.native != 0 && w.visible && w.parent == 0)
        .map(|(&h, w)| (h, w.native))
        .collect();
    let target = candidates.into_iter().find(|&(h, _)| {
        super::windows::screen_rect(h).is_some_and(|w| {
            r.left >= w.left - 1
                && r.top >= w.top - 1
                && r.right <= w.right + 1
                && r.bottom <= w.bottom + 1
        })
    });
    if let Some((_, native)) = target {
        c.confine(Some(native));
    }
    1
}
pub(crate) unsafe extern "win64" fn get_clip_cursor(out: *mut Rect) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let r = CLIP.lock().unwrap().unwrap_or_else(|| {
        let (w, h) = crate::host::x11::screen_size().unwrap_or((0, 0));
        Rect {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        }
    });
    unsafe { out.write(r) };
    1
}
/// Cursor handles: system cursors (IDC_* ids) as X font cursors; cursors from
/// program resources or bitmaps show the arrow.
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
pub(super) fn x_cursor_for(handle: usize) -> u64 {
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
pub(super) fn set_cursor(handle: usize) -> usize {
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
pub(crate) extern "win64" fn destroy_cursor(_handle: usize) -> i32 {
    1
}

// ---- raw input --------------------------------------------------------------------
const MOUSE_DEVICE: usize = 0x0007_0001;
const KEYBOARD_DEVICE: usize = 0x0007_0002;
#[derive(Clone, Copy)]
struct Registration {
    page: u16,
    usage: u16,
    flags: u32,
    target: usize,
}
struct RawState {
    devices: Vec<Registration>,
    /// Recent events by handle (RAWINPUT bytes), for GetRawInputData.
    events: VecDeque<(usize, Vec<u8>)>,
    /// Events not yet read through GetRawInputBuffer, per thread.
    unread: HashMap<u32, VecDeque<usize>>,
    next: usize,
    last: Option<(i32, i32)>,
}
static RAW: std::sync::LazyLock<Mutex<RawState>> = std::sync::LazyLock::new(|| {
    Mutex::new(RawState {
        devices: Vec::new(),
        events: VecDeque::new(),
        unread: HashMap::new(),
        next: 0x0008_0000,
        last: None,
    })
});
/// The window registered for a generic-desktop usage (2 mouse, 6 keyboard), or the
/// focus window when registered without a target.
fn raw_target(usage: u16, event_window: usize) -> Option<usize> {
    let raw = RAW.lock().unwrap();
    let r = raw
        .devices
        .iter()
        .find(|d| d.page == 1 && d.usage == usage && d.flags & 1 == 0)?;
    Some(if r.target != 0 {
        r.target
    } else {
        event_window
    })
}
fn deliver(target: usize, device: usize, kind: u32, body: &[u8]) {
    let Some(thread) = queue::owner(target) else {
        return;
    };
    // RAWINPUTHEADER: dwType, dwSize, hDevice, wParam (RIM_INPUT 0).
    let mut bytes = Vec::with_capacity(24 + body.len());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&((24 + body.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(&device.to_le_bytes());
    bytes.extend_from_slice(&0usize.to_le_bytes());
    bytes.extend_from_slice(body);
    let handle = {
        let mut raw = RAW.lock().unwrap();
        raw.next += 4;
        let handle = raw.next;
        raw.events.push_back((handle, bytes));
        if raw.events.len() > 4096 {
            raw.events.pop_front();
        }
        let unread = raw.unread.entry(thread).or_default();
        unread.push_back(handle);
        if unread.len() > 4096 {
            unread.pop_front();
        }
        handle
    };
    queue::post_to(thread, queue::message(target, WM_INPUT, 0, handle as isize));
}
/// RAWMOUSE: usFlags, (pad), usButtonFlags, usButtonData, ulRawButtons, lLastX,
/// lLastY, ulExtraInformation.
pub(super) fn raw_mouse(event_window: usize, buttons: u16, data: i16, x_root: Option<(i32, i32)>) {
    let Some(target) = raw_target(2, event_window) else {
        if let Some(p) = x_root {
            RAW.lock().unwrap().last = Some(p);
        }
        return;
    };
    let (dx, dy) = match x_root {
        Some((x, y)) => {
            let mut raw = RAW.lock().unwrap();
            let delta = raw.last.map_or((0, 0), |(lx, ly)| (x - lx, y - ly));
            raw.last = Some((x, y));
            delta
        }
        None => (0, 0),
    };
    if buttons == 0 && dx == 0 && dy == 0 {
        return;
    }
    let mut body = [0u8; 24];
    body[4..6].copy_from_slice(&buttons.to_le_bytes());
    body[6..8].copy_from_slice(&data.to_le_bytes());
    body[12..16].copy_from_slice(&dx.to_le_bytes());
    body[16..20].copy_from_slice(&dy.to_le_bytes());
    deliver(target, MOUSE_DEVICE, 0, &body);
}
/// RAWKEYBOARD: MakeCode, Flags (RI_KEY_BREAK 1, RI_KEY_E0 2), Reserved, VKey,
/// Message, ExtraInformation.
pub(super) fn raw_keyboard(event_window: usize, scan: u16, vk: u8, down: bool, message: u32) {
    let Some(target) = raw_target(6, event_window) else {
        return;
    };
    let mut body = [0u8; 16];
    body[0..2].copy_from_slice(&(scan & 0xff).to_le_bytes());
    let flags: u16 = (!down) as u16 | if scan & 0xe000 != 0 { 2 } else { 0 };
    body[2..4].copy_from_slice(&flags.to_le_bytes());
    body[6..8].copy_from_slice(&(vk as u16).to_le_bytes());
    body[8..12].copy_from_slice(&message.to_le_bytes());
    deliver(target, KEYBOARD_DEVICE, 1, &body);
}
/// RAWINPUTDEVICE: usUsagePage, usUsage, dwFlags (RIDEV_REMOVE 1), hwndTarget.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawDevice {
    page: u16,
    usage: u16,
    flags: u32,
    target: usize,
}
pub(crate) unsafe extern "win64" fn register_raw_input_devices(
    devices: *const RawDevice,
    count: u32,
    size: u32,
) -> i32 {
    if devices.is_null() || size as usize != std::mem::size_of::<RawDevice>() {
        return fail(87, 0);
    }
    let list = unsafe { std::slice::from_raw_parts(devices, count as usize) };
    let mut raw = RAW.lock().unwrap();
    for d in list {
        raw.devices
            .retain(|r| !(r.page == d.page && r.usage == d.usage));
        if d.flags & 1 == 0 {
            raw.devices.push(Registration {
                page: d.page,
                usage: d.usage,
                flags: d.flags,
                target: d.target,
            });
        }
    }
    1
}
pub(crate) unsafe extern "win64" fn get_registered_raw_input_devices(
    out: *mut RawDevice,
    count: *mut u32,
    size: u32,
) -> i32 {
    if count.is_null() || size as usize != std::mem::size_of::<RawDevice>() {
        return fail(87, -1);
    }
    let raw = RAW.lock().unwrap();
    let n = raw.devices.len() as u32;
    if out.is_null() || unsafe { *count } < n {
        unsafe { *count = n };
        return if out.is_null() { 0 } else { fail(122, -1) };
    }
    for (i, d) in raw.devices.iter().enumerate() {
        unsafe {
            out.add(i).write(RawDevice {
                page: d.page,
                usage: d.usage,
                flags: d.flags,
                target: d.target,
            })
        };
    }
    n as i32
}
/// GetRawInputData: RID_INPUT (0x10000003) the whole RAWINPUT, RID_HEADER
/// (0x10000005) its header.
pub(crate) unsafe extern "win64" fn get_raw_input_data(
    handle: usize,
    command: u32,
    data: *mut u8,
    size: *mut u32,
    header_size: u32,
) -> u32 {
    if size.is_null() || header_size != 24 {
        return fail(87, u32::MAX);
    }
    let raw = RAW.lock().unwrap();
    let Some((_, bytes)) = raw.events.iter().find(|(h, _)| *h == handle) else {
        return fail(6, u32::MAX);
    };
    let n = match command {
        0x1000_0003 => bytes.len(),
        0x1000_0005 => 24,
        _ => return fail(87, u32::MAX),
    } as u32;
    if data.is_null() {
        unsafe { *size = n };
        return 0;
    }
    if unsafe { *size } < n {
        return fail(122, u32::MAX);
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, n as usize) };
    n
}
/// GetRawInputBuffer: this thread's unread RAWINPUTs, each 8-byte aligned; reading
/// them removes their WM_INPUT messages.
pub(crate) unsafe extern "win64" fn get_raw_input_buffer(
    data: *mut u8,
    size: *mut u32,
    header_size: u32,
) -> u32 {
    if size.is_null() || header_size != 24 {
        return fail(87, u32::MAX);
    }
    let me = state::current_thread();
    let mut raw = RAW.lock().unwrap();
    let pending: Vec<usize> = raw
        .unread
        .get(&me)
        .map(|q| q.iter().copied().collect())
        .unwrap_or_default();
    let sizes: Vec<(usize, Vec<u8>)> = pending
        .iter()
        .filter_map(|h| raw.events.iter().find(|(e, _)| e == h).cloned())
        .collect();
    if data.is_null() {
        unsafe { *size = sizes.first().map_or(0, |(_, b)| (b.len() as u32 + 7) & !7) };
        return 0;
    }
    let capacity = unsafe { *size } as usize;
    let mut offset = 0;
    let mut count = 0;
    for (handle, bytes) in &sizes {
        let length = (bytes.len() + 7) & !7;
        if offset + length > capacity {
            break;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.add(offset), bytes.len());
            data.add(offset + bytes.len())
                .write_bytes(0, length - bytes.len());
        }
        offset += length;
        count += 1;
        if let Some(q) = raw.unread.get_mut(&me) {
            q.retain(|h| h != handle);
        }
    }
    drop(raw);
    if count == 0 && !sizes.is_empty() {
        unsafe { *size = (sizes[0].1.len() as u32 + 7) & !7 };
        return fail(122, u32::MAX);
    }
    count
}
/// The WM_INPUT message was retrieved: its event is no longer unread.
pub(super) fn raw_read(handle: usize) {
    let me = state::current_thread();
    if let Some(q) = RAW.lock().unwrap().unread.get_mut(&me) {
        q.retain(|&h| h != handle);
    }
}
/// RAWINPUTDEVICELIST: hDevice, dwType (0 mouse, 1 keyboard).
pub(crate) unsafe extern "win64" fn get_raw_input_device_list(
    out: *mut u8,
    count: *mut u32,
    size: u32,
) -> u32 {
    if count.is_null() || size != 16 {
        return fail(87, u32::MAX);
    }
    if out.is_null() {
        unsafe { *count = 2 };
        return 0;
    }
    if unsafe { *count } < 2 {
        unsafe { *count = 2 };
        return fail(122, u32::MAX);
    }
    for (i, (device, kind)) in [(MOUSE_DEVICE, 0u32), (KEYBOARD_DEVICE, 1)]
        .iter()
        .enumerate()
    {
        unsafe {
            out.add(16 * i).cast::<usize>().write_unaligned(*device);
            out.add(16 * i + 8).cast::<u32>().write_unaligned(*kind);
        }
    }
    2
}
/// RIDI_DEVICENAME (0x20000007), RIDI_DEVICEINFO (0x2000000b), RIDI_PREPARSEDDATA
/// (0x20000005, none for mouse and keyboard).
pub(crate) unsafe extern "win64" fn get_raw_input_device_info_w(
    device: usize,
    command: u32,
    data: *mut u8,
    size: *mut u32,
) -> u32 {
    if size.is_null() || (device != MOUSE_DEVICE && device != KEYBOARD_DEVICE) {
        return fail(6, u32::MAX);
    }
    let mouse = device == MOUSE_DEVICE;
    let bytes: Vec<u8> = match command {
        0x2000_0007 => {
            let name = if mouse {
                r"\\?\HID#PEEKGAME_MOUSE#0#{378de44c-56ef-11d1-bc8c-00a0c91405dd}"
            } else {
                r"\\?\HID#PEEKGAME_KEYBOARD#0#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}"
            };
            // The size is in characters for this command.
            let units: Vec<u16> = name.encode_utf16().chain([0]).collect();
            let n = units.len() as u32;
            if data.is_null() || unsafe { *size } < n {
                unsafe { *size = n };
                return if data.is_null() {
                    0
                } else {
                    fail(122, u32::MAX)
                };
            }
            unsafe {
                std::ptr::copy_nonoverlapping(units.as_ptr(), data.cast::<u16>(), units.len())
            };
            return n;
        }
        0x2000_000b => {
            let mut info = vec![0u8; 32];
            info[0..4].copy_from_slice(&32u32.to_le_bytes());
            info[4..8].copy_from_slice(&(if mouse { 0u32 } else { 1 }).to_le_bytes());
            let fields: [u32; 6] = if mouse {
                [0x100, 5, 0, 1, 0, 0]
            } else {
                [4, 0, 1, 12, 3, 101]
            };
            for (i, f) in fields.iter().enumerate() {
                info[8 + 4 * i..12 + 4 * i].copy_from_slice(&f.to_le_bytes());
            }
            info
        }
        0x2000_0005 => Vec::new(),
        _ => return fail(87, u32::MAX),
    };
    let n = bytes.len() as u32;
    if data.is_null() {
        unsafe { *size = n };
        return 0;
    }
    if unsafe { *size } < n {
        unsafe { *size = n };
        return fail(122, u32::MAX);
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
    n
}
pub(crate) unsafe extern "win64" fn get_raw_input_device_info_a(
    device: usize,
    command: u32,
    data: *mut u8,
    size: *mut u32,
) -> u32 {
    if command != 0x2000_0007 {
        return unsafe { get_raw_input_device_info_w(device, command, data, size) };
    }
    let mut wide = [0u16; 128];
    let mut n = 128u32;
    if unsafe { get_raw_input_device_info_w(device, command, wide.as_mut_ptr().cast(), &mut n) }
        == u32::MAX
    {
        return u32::MAX;
    }
    if data.is_null() || unsafe { *size } < n {
        unsafe { *size = n };
        return if data.is_null() {
            0
        } else {
            fail(122, u32::MAX)
        };
    }
    for (i, unit) in wide.iter().take(n as usize).enumerate() {
        unsafe { *data.add(i) = *unit as u8 };
    }
    n
}
pub(crate) extern "win64" fn define_raw_input_proc() -> isize {
    0
}
