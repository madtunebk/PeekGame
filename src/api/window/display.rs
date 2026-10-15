//! Monitors, system metrics and display parameters (after peek's user32_more.inc).
//! There is one monitor: the default X screen, at the origin, with the whole screen
//! as its work area. Windows have no non-client area: the compositor draws frames.
use crate::{
    api::{system::error, text},
    host::x11,
};
use std::sync::Mutex;
const MONITOR: usize = 0x2_0001;
const MONITORINFOF_PRIMARY: u32 = 1;

fn screen() -> (i32, i32) {
    x11::screen_size().unwrap_or((0, 0))
}
fn have_display() -> bool {
    x11::screen_size().is_some()
}

pub extern "win64" fn monitor_from_window(_hwnd: usize, flags: u32) -> usize {
    if have_display() || flags != 0 {
        MONITOR
    } else {
        0
    }
}
/// MONITOR_DEFAULTTONULL (0) gives NULL off-screen; PRIMARY (1) and NEAREST (2)
/// give the one monitor.
fn monitor_for(x0: i32, y0: i32, x1: i32, y1: i32, flags: u32) -> usize {
    if flags > 2 {
        error::set(87);
        return 0;
    }
    let (w, h) = screen();
    let inside = x1 > 0 && y1 > 0 && x0 < w && y0 < h;
    if inside || flags != 0 {
        MONITOR
    } else {
        0
    }
}
pub extern "win64" fn monitor_from_point(point: i64, flags: u32) -> usize {
    let (x, y) = (point as i32, (point >> 32) as i32);
    monitor_for(x, y, x + 1, y + 1, flags)
}
pub unsafe extern "win64" fn monitor_from_rect(rect: *const i32, flags: u32) -> usize {
    if rect.is_null() {
        error::set(87);
        return 0;
    }
    let r = unsafe { std::slice::from_raw_parts(rect, 4) };
    monitor_for(r[0], r[1], r[2].max(r[0] + 1), r[3].max(r[1] + 1), flags)
}
/// MONITORINFO (40 bytes) or MONITORINFOEX (W 104, A 72): rcMonitor, rcWork,
/// dwFlags, szDevice "\\.\DISPLAY1".
unsafe fn monitor_info(monitor: usize, out: *mut u8, wide: bool) -> i32 {
    if monitor != MONITOR || out.is_null() {
        error::set(87);
        return 0;
    }
    let size = unsafe { (out as *const u32).read_unaligned() };
    if size != 40 && size != if wide { 104 } else { 72 } {
        error::set(87);
        return 0;
    }
    let (w, h) = screen();
    unsafe {
        let r = out.add(4).cast::<i32>();
        for (i, v) in [0, 0, w, h, 0, 0, w, h].into_iter().enumerate() {
            r.add(i).write_unaligned(v);
        }
        (out.add(36) as *mut u32).write_unaligned(MONITORINFOF_PRIMARY);
        if size > 40 {
            let name = "\\\\.\\DISPLAY1";
            if wide {
                out.add(40).write_bytes(0, 64);
                for (i, u) in name.encode_utf16().enumerate() {
                    (out.add(40 + i * 2) as *mut u16).write_unaligned(u);
                }
            } else {
                out.add(40).write_bytes(0, 32);
                std::ptr::copy_nonoverlapping(name.as_ptr(), out.add(40), name.len());
            }
        }
    }
    1
}
pub unsafe extern "win64" fn monitor_info_w(monitor: usize, out: *mut u8) -> i32 {
    unsafe { monitor_info(monitor, out, true) }
}
pub unsafe extern "win64" fn monitor_info_a(monitor: usize, out: *mut u8) -> i32 {
    unsafe { monitor_info(monitor, out, false) }
}
type MonitorEnum = extern "win64" fn(usize, usize, *mut i32, isize) -> i32;
pub unsafe extern "win64" fn enum_display_monitors(
    hdc: usize,
    clip: *const i32,
    proc: Option<MonitorEnum>,
    data: isize,
) -> i32 {
    if hdc != 0 || !clip.is_null() {
        eprintln!("EnumDisplayMonitors with an HDC or clip rectangle is not supported");
        error::set(50);
        return 0;
    }
    let Some(proc) = proc else {
        error::set(87);
        return 0;
    };
    if have_display() {
        let (w, h) = screen();
        let mut rect = [0, 0, w, h];
        proc(MONITOR, 0, rect.as_mut_ptr(), data);
    }
    1
}
/// DISPLAY_DEVICEW (840 bytes) / DISPLAY_DEVICEA (424): one adapter "\\.\DISPLAY1"
/// (ATTACHED_TO_DESKTOP | PRIMARY_DEVICE), and its monitor "\\.\DISPLAY1\Monitor0".
unsafe fn display_device(device: Option<String>, index: u32, out: *mut u8, wide: bool) -> i32 {
    let needed = if wide { 840 } else { 424 };
    if out.is_null() || unsafe { (out as *const u32).read_unaligned() } < needed {
        error::set(87);
        return 0;
    }
    let (name, description, flags) = match device.as_deref() {
        None | Some("") if index == 0 && have_display() => {
            ("\\\\.\\DISPLAY1".to_owned(), "X11 display adapter", 1 | 4)
        }
        Some("\\\\.\\DISPLAY1") if index == 0 => (
            "\\\\.\\DISPLAY1\\Monitor0".to_owned(),
            "Generic monitor",
            1 | 2,
        ),
        _ => return 0, // end of the list, no error code, as on Windows
    };
    unsafe {
        out.add(4).write_bytes(0, needed as usize - 4);
        let put = |at: usize, text: &str| {
            if wide {
                for (i, u) in text.encode_utf16().enumerate() {
                    (out.add(at + i * 2) as *mut u16).write_unaligned(u);
                }
            } else {
                std::ptr::copy_nonoverlapping(text.as_ptr(), out.add(at), text.len());
            }
        };
        if wide {
            put(4, &name);
            put(68, description);
            (out.add(324) as *mut u32).write_unaligned(flags);
        } else {
            put(4, &name);
            put(36, description);
            (out.add(164) as *mut u32).write_unaligned(flags);
        }
    }
    1
}
pub unsafe extern "win64" fn enum_display_devices_w(
    device: *const u16,
    index: u32,
    out: *mut u8,
    _flags: u32,
) -> i32 {
    unsafe { display_device(text::string(device), index, out, true) }
}
pub unsafe extern "win64" fn enum_display_devices_a(
    device: *const std::ffi::c_char,
    index: u32,
    out: *mut u8,
    _flags: u32,
) -> i32 {
    let device = (!device.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(device) }
            .to_string_lossy()
            .into_owned()
    });
    unsafe { display_device(device, index, out, false) }
}
/// DEVMODEW (220 bytes): the current mode of the X screen only (ENUM_CURRENT_SETTINGS
/// -1, ENUM_REGISTRY_SETTINGS -2, or mode 0); 32 bpp, 60 Hz.
unsafe fn display_settings_w(out: *mut u8, mode: u32) -> i32 {
    if !(mode == 0 || mode == u32::MAX || mode == u32::MAX - 1) || !have_display() {
        return 0;
    }
    let (w, h) = screen();
    unsafe {
        let extra = (out.add(70) as *const u16).read_unaligned();
        out.write_bytes(0, 220);
        for (i, u) in "\\\\.\\DISPLAY1".encode_utf16().enumerate() {
            (out.add(i * 2) as *mut u16).write_unaligned(u);
        }
        (out.add(68) as *mut u16).write_unaligned(220);
        (out.add(70) as *mut u16).write_unaligned(extra);
        (out.add(72) as *mut u32).write_unaligned(0x20 | 0x40000 | 0x80000 | 0x100000 | 0x400000);
        for (at, v) in [(168, 32u32), (172, w as u32), (176, h as u32), (184, 60)] {
            (out.add(at) as *mut u32).write_unaligned(v);
        }
    }
    1
}
pub unsafe extern "win64" fn enum_display_settings_w(
    device: *const u16,
    mode: u32,
    out: *mut u8,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    if let Some(name) = unsafe { text::string(device) } {
        if name != "\\\\.\\DISPLAY1" {
            error::set(87);
            return 0;
        }
    }
    unsafe { display_settings_w(out, mode) }
}
/// DEVMODEA (156 bytes): DEVMODEW with 32-byte names; dmSpecVersion..dmCollate move
/// from 64 to 32 and dmLogPixels..dmPanningHeight from 166 to 102.
pub unsafe extern "win64" fn enum_display_settings_a(
    device: *const std::ffi::c_char,
    mode: u32,
    out: *mut u8,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    if !device.is_null()
        && unsafe { std::ffi::CStr::from_ptr(device) }.to_bytes() != b"\\\\.\\DISPLAY1"
    {
        error::set(87);
        return 0;
    }
    let mut w = [0u8; 220];
    if unsafe { display_settings_w(w.as_mut_ptr(), mode) } == 0 {
        return 0;
    }
    unsafe {
        let extra = (out.add(38) as *const u16).read_unaligned();
        out.write_bytes(0, 156);
        for i in 0..31 {
            *out.add(i) = w[i * 2];
        }
        std::ptr::copy_nonoverlapping(w.as_ptr().add(64), out.add(32), 38);
        std::ptr::copy_nonoverlapping(w.as_ptr().add(166), out.add(102), 54);
        (out.add(36) as *mut u16).write_unaligned(156);
        (out.add(38) as *mut u16).write_unaligned(extra);
    }
    1
}
pub extern "win64" fn system_metrics(index: i32) -> i32 {
    let (w, h) = screen();
    match index {
        0 | 78 => w,                     // SM_CXSCREEN, SM_CXVIRTUALSCREEN
        1 | 79 => h,                     // SM_CYSCREEN, SM_CYVIRTUALSCREEN
        76 | 77 => 0,                    // virtual screen origin
        80 => have_display() as i32,     // SM_CMONITORS
        81 | 82 | 75 | 19 => 1,          // same format, IMM, wheel, mouse present
        43 => 5,                         // SM_CMOUSEBUTTONS
        23 | 86 | 94 | 95 | 0x1000 => 0, // swap buttons, tablet, digitizer, touches, remote
        4..=8 | 32 | 33 | 92 => 0,       // caption/borders/frame/padding: no non-client area
        _ => {
            eprintln!("STOP: GetSystemMetrics({index}) is not mapped");
            std::process::exit(78)
        }
    }
}
/// FILTERKEYS (24 bytes), TOGGLEKEYS (8), STICKYKEYS (8): no such features exist,
/// so their flags start without *_AVAILABLE; SET stores and GET returns them.
static ACCESSIBILITY: Mutex<[[u32; 6]; 3]> =
    Mutex::new([[24, 0, 0, 0, 0, 0], [8, 0, 0, 0, 0, 0], [8, 0, 0, 0, 0, 0]]);
pub unsafe extern "win64" fn system_parameters_info(
    action: u32,
    param: u32,
    out: *mut u8,
    _ini: u32,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let keys = match action {
        0x32 | 0x33 => Some(0),
        0x34 | 0x35 => Some(1),
        0x3A | 0x3B => Some(2),
        _ => None,
    };
    let put = |v: u32| unsafe { (out as *mut u32).write_unaligned(v) };
    if let Some(k) = keys {
        let mut all = ACCESSIBILITY.lock().unwrap();
        let size = all[k][0];
        if param != size || unsafe { (out as *const u32).read_unaligned() } != size {
            error::set(87);
            return 0;
        }
        let words = size as usize / 4;
        unsafe {
            if action & 1 != 0 {
                std::ptr::copy_nonoverlapping(out as *const u32, all[k].as_mut_ptr(), words);
            } else {
                std::ptr::copy_nonoverlapping(all[k].as_ptr(), out as *mut u32, words);
            }
        }
        return 1;
    }
    match action {
        0x30 => {
            // SPI_GETWORKAREA: the whole screen.
            let (w, h) = screen();
            for (i, v) in [0, 0, w, h].into_iter().enumerate() {
                unsafe { (out as *mut i32).add(i).write_unaligned(v) };
            }
        }
        0x5E | 0x46 => put(0), // mouse trails off; no screen reader
        0x66 => put(400),      // hover time (Windows default)
        0x68 | 0x6C => put(3), // wheel scroll lines/chars
        _ => {
            eprintln!("STOP: SystemParametersInfo({action:#x}) is not mapped");
            std::process::exit(78)
        }
    }
    1
}
pub extern "win64" fn double_click_time() -> u32 {
    500
}
pub extern "win64" fn caret_blink_time() -> u32 {
    530
}
/// Coordinates are physical pixels everywhere, which is what a DPI-aware process sees.
pub extern "win64" fn is_process_dpi_aware() -> i32 {
    1
}
