//! Keys as Windows sees them: X keycodes to PC scan codes (set 1, 0xE0xx for
//! extended keys: evdev numbers equal set-1 codes for the main block), scan codes
//! to virtual keys (the US positional table Windows layouts start from; letter and
//! digit keys follow the X layout, as Windows layouts move them), and X keysyms to
//! the characters the keys type. These are Windows and X11 ABI facts, the same for
//! every program.

/// Set-1 scan code of an X keycode (evdev + 8); 0 for keys Windows has no code for.
pub(crate) fn scan(keycode: u32) -> u16 {
    let evdev = keycode.wrapping_sub(8);
    match evdev {
        1..=88 => evdev as u16,
        96 => 0xe01c,                             // KP Enter
        97 => 0xe01d,                             // Right Ctrl
        98 => 0xe035,                             // KP /
        99 => 0xe037,                             // Print Screen
        100 => 0xe038,                            // Right Alt
        102 => 0xe047,                            // Home
        103 => 0xe048,                            // Up
        104 => 0xe049,                            // Page Up
        105 => 0xe04b,                            // Left
        106 => 0xe04d,                            // Right
        107 => 0xe04f,                            // End
        108 => 0xe050,                            // Down
        109 => 0xe051,                            // Page Down
        110 => 0xe052,                            // Insert
        111 => 0xe053,                            // Delete
        119 => 0x45,                              // Pause (VK_PAUSE)
        125 => 0xe05b,                            // Left Windows
        126 => 0xe05c,                            // Right Windows
        127 => 0xe05d,                            // Menu
        183..=194 => (0x64 + evdev - 183) as u16, // F13..F24
        _ => 0,
    }
}
/// The X keycode of a scan code (the inverse of [`scan`]).
pub(crate) fn keycode(scan_code: u16) -> Option<u32> {
    (8..256u32).find(|&k| scan(k) == scan_code && scan_code != 0)
}
/// Positional virtual key of a scan code (Windows' US table), with the generic
/// VK_SHIFT/VK_CONTROL/VK_MENU for modifiers when `generic`.
pub(crate) fn positional_vk(scan_code: u16, generic: bool) -> u8 {
    const MAIN: [u8; 0x59] = [
        0, 0x1b, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', 0xbd, 0xbb, 0x08,
        0x09, // 0x00
        b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I', b'O', b'P', 0xdb, 0xdd, 0x0d, 0xa2, b'A',
        b'S', // 0x10
        b'D', b'F', b'G', b'H', b'J', b'K', b'L', 0xba, 0xde, 0xc0, 0xa0, 0xdc, b'Z', b'X', b'C',
        b'V', // 0x20
        b'B', b'N', b'M', 0xbc, 0xbe, 0xbf, 0xa1, 0x6a, 0xa4, 0x20, 0x14, 0x70, 0x71, 0x72, 0x73,
        0x74, // 0x30
        0x75, 0x76, 0x77, 0x78, 0x79, 0x90, 0x91, 0x67, 0x68, 0x69, 0x6d, 0x64, 0x65, 0x66, 0x6b,
        0x61, // 0x40
        0x62, 0x63, 0x60, 0x6e, 0, 0, 0xe2, 0x7a, 0x7b, // 0x50
    ];
    let vk = match scan_code {
        0x45 => 0x90,
        0x01..=0x58 => MAIN[scan_code as usize],
        0x64..=0x6f => 0x7c + (scan_code - 0x64) as u8, // F13..F24
        0xe01c => 0x0d,
        0xe01d => 0xa3,
        0xe035 => 0x6f,
        0xe037 => 0x2c,
        0xe038 => 0xa5,
        0xe047 => 0x24,
        0xe048 => 0x26,
        0xe049 => 0x21,
        0xe04b => 0x25,
        0xe04d => 0x27,
        0xe04f => 0x23,
        0xe050 => 0x28,
        0xe051 => 0x22,
        0xe052 => 0x2d,
        0xe053 => 0x2e,
        0xe05b => 0x5b,
        0xe05c => 0x5c,
        0xe05d => 0x5d,
        _ => 0,
    };
    if !generic {
        return vk;
    }
    match vk {
        0xa0 | 0xa1 => 0x10,
        0xa2 | 0xa3 => 0x11,
        0xa4 | 0xa5 => 0x12,
        v => v,
    }
}
/// The scan code of a virtual key (MapVirtualKey VK_TO_VSC): the first key whose
/// positional key is `vk`; generic modifiers map to their left key.
pub(crate) fn scan_of_vk(vk: u8) -> u16 {
    let vk = match vk {
        0x10 => 0xa0,
        0x11 => 0xa2,
        0x12 => 0xa4,
        v => v,
    };
    (1u16..=0x58)
        .chain(0x64..=0x6f)
        .chain([
            0xe01c, 0xe01d, 0xe035, 0xe037, 0xe038, 0xe047, 0xe048, 0xe049, 0xe04b, 0xe04d, 0xe04f,
            0xe050, 0xe051, 0xe052, 0xe053, 0xe05b, 0xe05c, 0xe05d,
        ])
        .find(|&s| positional_vk(s, false) == vk)
        .unwrap_or(0)
}
/// The virtual key of a key event: letters and digits follow the layout's base
/// symbol; the keypad gives VK_NUMPADn while Num Lock is on; the rest is positional.
pub(crate) fn virtual_key(scan_code: u16, base_symbol: u64, num_lock: bool, shift: bool) -> u8 {
    if (b'a' as u64..=b'z' as u64).contains(&base_symbol) {
        return (base_symbol as u8).to_ascii_uppercase();
    }
    if (b'0' as u64..=b'9' as u64).contains(&base_symbol) && scan_code < 0x47 {
        return base_symbol as u8;
    }
    let vk = positional_vk(scan_code, true);
    if !num_lock || shift {
        // Without Num Lock the keypad keys are navigation keys.
        return match vk {
            0x67 => 0x24,
            0x68 => 0x26,
            0x69 => 0x21,
            0x64 => 0x25,
            0x65 => 0x0c,
            0x66 => 0x27,
            0x61 => 0x23,
            0x62 => 0x28,
            0x63 => 0x22,
            0x60 => 0x2d,
            0x6e => 0x2e,
            v => v,
        };
    }
    vk
}
/// The character a keysym types, if any.
pub(crate) fn character(symbol: u64) -> Option<u32> {
    Some(match symbol {
        0x20..=0x7e | 0xa0..=0xff => symbol as u32,
        0x0100_0000..=0x0110_ffff => (symbol - 0x0100_0000) as u32,
        0x20ac => 0x20ac,
        0xff08 => 8,
        0xff09 => 9,
        0xff0d | 0xff8d => 13,
        0xff1b => 27,
        0xff80 => b' ' as u32,
        0xffaa => b'*' as u32,
        0xffab => b'+' as u32,
        0xffac => b',' as u32,
        0xffad => b'-' as u32,
        0xffae => b'.' as u32,
        0xffaf => b'/' as u32,
        0xffb0..=0xffb9 => (b'0' as u64 + symbol - 0xffb0) as u32,
        0xffbd => b'=' as u32,
        _ => return None,
    })
}
/// GetKeyNameText's name for a scan code (`extended` from lParam bit 24).
pub(crate) fn name(scan_code: u16, character: Option<u32>) -> Option<String> {
    let text = match scan_code {
        0x01 => "Esc",
        0x0e => "Backspace",
        0x0f => "Tab",
        0x1c => "Enter",
        0x1d => "Ctrl",
        0x2a => "Shift",
        0x36 => "Right Shift",
        0x37 => "Num *",
        0x38 => "Alt",
        0x39 => "Space",
        0x3a => "Caps Lock",
        0x3b..=0x44 => return Some(format!("F{}", scan_code - 0x3a)),
        0x45 => "Pause",
        0x46 => "Scroll Lock",
        0x47 => "Num 7",
        0x48 => "Num 8",
        0x49 => "Num 9",
        0x4a => "Num -",
        0x4b => "Num 4",
        0x4c => "Num 5",
        0x4d => "Num 6",
        0x4e => "Num +",
        0x4f => "Num 1",
        0x50 => "Num 2",
        0x51 => "Num 3",
        0x52 => "Num 0",
        0x53 => "Num Del",
        0x57 => "F11",
        0x58 => "F12",
        0x64..=0x6f => return Some(format!("F{}", scan_code - 0x64 + 13)),
        0xe01c => "Num Enter",
        0xe01d => "Right Ctrl",
        0xe035 => "Num /",
        0xe037 => "Prnt Scrn",
        0xe038 => "Right Alt",
        0xe047 => "Home",
        0xe048 => "Up",
        0xe049 => "Page Up",
        0xe04b => "Left",
        0xe04d => "Right",
        0xe04f => "End",
        0xe050 => "Down",
        0xe051 => "Page Down",
        0xe052 => "Insert",
        0xe053 => "Delete",
        0xe05b => "Left Windows",
        0xe05c => "Right Windows",
        0xe05d => "Application",
        _ => {
            let c = char::from_u32(character?)?;
            return (!c.is_control()).then(|| c.to_uppercase().collect());
        }
    };
    Some(text.into())
}
