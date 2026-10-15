// X11 keysyms -> Windows virtual keys and PC scan codes. No game-specific values.
pub(super) fn key(symbol: u64) -> Option<(usize, u8)> {
    if symbol == 0xff1b {
        return Some((0x1b, 1));
    }
    if symbol == 0x20 {
        return Some((0x20, 0x39));
    }
    if (b'a' as u64..=b'z' as u64).contains(&symbol)
        || (b'A' as u64..=b'Z' as u64).contains(&symbol)
    {
        let letter = (symbol as u8).to_ascii_uppercase();
        const SCANS: [u8; 26] = [
            0x1e, 0x30, 0x2e, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31,
            0x18, 0x19, 0x10, 0x13, 0x1f, 0x14, 0x16, 0x2f, 0x11, 0x2d, 0x15, 0x2c,
        ];
        return Some((letter as usize, SCANS[(letter - b'A') as usize]));
    }
    if (b'0' as u64..=b'9' as u64).contains(&symbol) {
        return Some((
            symbol as usize,
            if symbol == b'0' as u64 {
                11
            } else {
                symbol as u8 - b'1' + 2
            },
        ));
    }
    None
}
