use super::fail;
use crate::{api::window, host::graphics::glx};
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{LazyLock, Mutex},
};
#[derive(Clone, Copy)]
pub(super) struct Format {
    pub display: usize,
    pub window: u64,
    pub config: usize,
    pub actual: [i32; 7],
    pub set: bool,
}
static FORMATS: LazyLock<Mutex<HashMap<usize, Format>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub(super) fn native(dc: usize) -> Option<(usize, window::NativeWindow)> {
    let hwnd = window::misc::window_from_dc(dc);
    let mut n = window::NativeWindow {
        display: std::ptr::null_mut(),
        window: 0,
    };
    if hwnd == 0 || unsafe { window::peekgame_x11_window(hwnd, &mut n) } != 0 {
        return None;
    }
    Some((hwnd, n))
}
pub(super) fn format(dc: usize) -> Option<Format> {
    let (hwnd, n) = native(dc)?;
    FORMATS
        .lock()
        .unwrap()
        .get(&hwnd)
        .copied()
        .filter(|f| f.set && f.window == n.window && f.display == n.display as usize)
}
pub(crate) unsafe extern "win64" fn choose_pixel_format(dc: usize, pfd: *const u8) -> i32 {
    let Some((hwnd, n)) = native(dc) else {
        return fail(6, 0);
    };
    if pfd.is_null() {
        return fail(87, 0);
    }
    let bytes = unsafe { std::slice::from_raw_parts(pfd, 40) };
    if u16::from_le_bytes([bytes[0], bytes[1]]) != 40
        || u16::from_le_bytes([bytes[2], bytes[3]]) != 1
        || bytes[8] != 0
    {
        return fail(87, 0);
    }
    let flags = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    super::trace(format_args!(
        "ChoosePixelFormat dc={dc:x} flags={flags:x} depth={} stencil={} alpha={}",
        bytes[23], bytes[24], bytes[16]
    ));
    if flags & 0x24 != 0x24 || flags & (0x8 | 0x2 | 0x80) != 0 {
        return fail(50, 0);
    }
    let mut formats = FORMATS.lock().unwrap();
    if formats
        .get(&hwnd)
        .is_some_and(|f| f.set && f.window == n.window)
    {
        return 1;
    }
    let mut actual = [0; 7];
    let config = unsafe {
        glx::peekgame_glx_format(
            n.display,
            n.window,
            bytes[23] as i32,
            bytes[24] as i32,
            bytes[16] as i32,
            (flags & 1 != 0) as i32,
            actual.as_mut_ptr(),
        )
    };
    if config.is_null() {
        return fail(2000, 0);
    }
    formats.insert(
        hwnd,
        Format {
            display: n.display as usize,
            window: n.window,
            config: config as usize,
            actual,
            set: false,
        },
    );
    1
}
pub(crate) unsafe extern "win64" fn describe_pixel_format(
    dc: usize,
    index: i32,
    size: u32,
    pfd: *mut u8,
) -> i32 {
    let Some((hwnd, _)) = native(dc) else {
        return fail(6, 0);
    };
    if index != 1 {
        return fail(2000, 0);
    }
    let Some(f) = FORMATS.lock().unwrap().get(&hwnd).copied() else {
        return fail(2000, 0);
    };
    if pfd.is_null() {
        return 1;
    }
    if size < 40 {
        return fail(87, 0);
    }
    let mut bytes = [0u8; 40];
    bytes[0] = 40;
    bytes[2] = 1;
    bytes[4..8].copy_from_slice(&(0x24u32 | f.actual[6] as u32).to_le_bytes());
    bytes[9] = (f.actual[0] + f.actual[1] + f.actual[2]) as u8;
    bytes[10] = f.actual[0] as u8;
    bytes[11] = 16;
    bytes[12] = f.actual[1] as u8;
    bytes[13] = 8;
    bytes[14] = f.actual[2] as u8;
    bytes[16] = f.actual[3] as u8;
    bytes[17] = 24;
    bytes[23] = f.actual[4] as u8;
    bytes[24] = f.actual[5] as u8;
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), pfd, 40) };
    1
}
pub(crate) unsafe extern "win64" fn set_pixel_format(
    dc: usize,
    index: i32,
    _pfd: *const c_void,
) -> i32 {
    super::trace(format_args!("SetPixelFormat dc={dc:x} index={index}"));
    let Some((hwnd, n)) = native(dc) else {
        return fail(6, 0);
    };
    if index != 1 {
        return fail(2000, 0);
    }
    let mut formats = FORMATS.lock().unwrap();
    let Some(f) = formats.get_mut(&hwnd).filter(|f| f.window == n.window) else {
        return fail(2000, 0);
    };
    // Reapplying the already installed format is idempotent; it cannot change.
    if f.set {
        return 1;
    }
    f.set = true;
    1
}
pub(crate) extern "win64" fn get_pixel_format(dc: usize) -> i32 {
    format(dc).is_some() as i32
}
