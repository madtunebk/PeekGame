//! Keyboard input implementation and owned state.
use super::fail;
use crate::api::window::{state::connection, strings};
use crate::host::x11::keyboard;
use std::sync::Mutex;

// ---- key state ------------------------------------------------------------------
/// Per virtual key: 0x80 down, 0x01 toggled.
static KEYS: Mutex<[u8; 256]> = Mutex::new([0; 256]);
pub(in crate::api::window) fn set_key(vk: u8, down: bool) {
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
pub(in crate::api::window) fn sync_toggles(state: u32) {
    let mut keys = KEYS.lock().unwrap();
    for (vk, on) in [(0x14usize, state & 2 != 0), (0x90, state & 0x10 != 0)] {
        keys[vk] = (keys[vk] & 0x80) | on as u8;
    }
}
/// Every key up (focus left the program: no stuck keys).
pub(in crate::api::window) fn release_keys() {
    let mut keys = KEYS.lock().unwrap();
    for k in keys.iter_mut() {
        *k &= 1;
    }
}
pub(crate) fn key_down(vk: u8) -> bool {
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
