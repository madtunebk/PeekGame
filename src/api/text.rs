//! UTF-16 strings as Windows APIs pass them.
/// Units of a NUL-terminated string, without the terminator.
pub(crate) unsafe fn units<'a>(p: *const u16) -> &'a [u16] {
    if p.is_null() {
        return &[];
    }
    let n = (0..).take_while(|&i| unsafe { *p.add(i) } != 0).count();
    unsafe { std::slice::from_raw_parts(p, n) }
}
/// `count` units, or up to the terminator when `count` is -1 (the terminator
/// included when `with_nul`).
pub(crate) unsafe fn counted<'a>(p: *const u16, count: i32, with_nul: bool) -> Option<&'a [u16]> {
    if p.is_null() || count < -1 {
        return None;
    }
    let n = if count == -1 {
        unsafe { units(p) }.len() + with_nul as usize
    } else {
        count as usize
    };
    Some(unsafe { std::slice::from_raw_parts(p, n) })
}
pub(crate) unsafe fn string(p: *const u16) -> Option<String> {
    (!p.is_null()).then(|| String::from_utf16_lossy(unsafe { units(p) }))
}
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}
/// The Windows "copy into a caller buffer" contract shared by many W APIs:
/// writes `text` plus a NUL when it fits and returns the length without the NUL;
/// otherwise returns the size needed including the NUL.
pub(crate) unsafe fn copy_out(text: &[u16], out: *mut u16, capacity: u32) -> u32 {
    let needed = text.len() as u32 + 1;
    if out.is_null() || capacity < needed {
        return needed;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
        *out.add(text.len()) = 0;
    }
    text.len() as u32
}
/// The same contract for narrow strings.
pub(crate) unsafe fn copy_out_narrow(text: &[u8], out: *mut u8, capacity: u32) -> u32 {
    let needed = text.len() as u32 + 1;
    if out.is_null() || capacity < needed {
        return needed;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
        *out.add(text.len()) = 0;
    }
    text.len() as u32
}

pub(crate) mod dwrite;
