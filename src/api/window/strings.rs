pub(super) unsafe fn wide(pointer: *const u16) -> Result<String, u32> {
    if pointer.is_null() || (pointer as usize) < 65536 {
        return Err(87);
    }
    let mut units = Vec::new();
    for offset in 0..32768 {
        let ch = unsafe { pointer.add(offset).read() };
        if ch == 0 {
            return String::from_utf16(&units).map_err(|_| 1113);
        }
        units.push(ch);
    }
    Err(206)
}
