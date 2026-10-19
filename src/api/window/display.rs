//! Monitors, system metrics and display parameters (after peek's user32_more.inc).
//! RandR monitors are distinct from the virtual desktop; the primary starts at 0,0.
//! Work areas currently equal each monitor rectangle. Windows have no non-client area: the compositor draws frames.
use crate::host::x11::monitors::Monitor;
use crate::{
    api::{system::error, text},
    host::x11,
};
use std::sync::Mutex;
fn handle(m: &Monitor) -> usize {
    0x20000 + m.id.max(1) as usize
}
fn all() -> Vec<Monitor> {
    x11::monitors()
}
fn screen() -> (i32, i32) {
    x11::primary_size().unwrap_or((0, 0))
}
pub(super) fn virtual_rect() -> [i32; 4] {
    let monitors = all();
    [
        monitors.iter().map(|m| m.x).min().unwrap_or(0),
        monitors.iter().map(|m| m.y).min().unwrap_or(0),
        monitors.iter().map(|m| m.x + m.width).max().unwrap_or(0),
        monitors.iter().map(|m| m.y + m.height).max().unwrap_or(0),
    ]
}
pub extern "win64" fn monitor_from_window(hwnd: usize, flags: u32) -> usize {
    if let Some(r) = super::windows::screen_rect(hwnd) {
        monitor_for(r.left, r.top, r.right, r.bottom, flags)
    } else if flags == 1 || flags == 2 {
        all().first().map(handle).unwrap_or(0)
    } else {
        0
    }
}
fn monitor_for(x0: i32, y0: i32, x1: i32, y1: i32, flags: u32) -> usize {
    if flags > 2 {
        error::set(87);
        return 0;
    }
    x11::monitors::select(&all(), [x0, y0, x1, y1], flags)
        .map(handle)
        .unwrap_or(0)
}
pub extern "win64" fn monitor_from_point(point: i64, flags: u32) -> usize {
    let (x, y) = (point as i32, (point >> 32) as i32);
    monitor_for(x, y, x.saturating_add(1), y.saturating_add(1), flags)
}
pub unsafe extern "win64" fn monitor_from_rect(rect: *const i32, flags: u32) -> usize {
    if rect.is_null() {
        error::set(87);
        return 0;
    }
    let r = unsafe { std::slice::from_raw_parts(rect, 4) };
    monitor_for(
        r[0],
        r[1],
        r[2].max(r[0].saturating_add(1)),
        r[3].max(r[1].saturating_add(1)),
        flags,
    )
}
fn name(index: usize) -> String {
    format!("\\\\.\\DISPLAY{}", index + 1)
}
fn named(device: Option<&str>) -> Option<(usize, Monitor)> {
    let monitors = all();
    match device {
        None | Some("") => monitors.into_iter().enumerate().next(),
        Some(device) => monitors
            .into_iter()
            .enumerate()
            .find(|(i, _)| name(*i).eq_ignore_ascii_case(device)),
    }
}
/// The size of a display device ("\\.\DISPLAYn", or the primary for none).
pub(crate) fn device_size(device: Option<&str>) -> Option<(i32, i32)> {
    named(device).map(|(_, m)| (m.width, m.height))
}
unsafe fn monitor_info(monitor: usize, out: *mut u8, wide: bool) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let Some((index, m)) = all()
        .into_iter()
        .enumerate()
        .find(|(_, m)| handle(m) == monitor)
    else {
        error::set(87);
        return 0;
    };
    let size = unsafe { out.cast::<u32>().read_unaligned() };
    if size != 40 && size != if wide { 104 } else { 72 } {
        error::set(87);
        return 0;
    }
    unsafe {
        for (i, v) in [
            m.x,
            m.y,
            m.x + m.width,
            m.y + m.height,
            m.x,
            m.y,
            m.x + m.width,
            m.y + m.height,
        ]
        .into_iter()
        .enumerate()
        {
            out.add(4 + i * 4).cast::<i32>().write_unaligned(v);
        }
        out.add(36).cast::<u32>().write_unaligned(m.primary as u32);
        if size > 40 {
            out.add(40).write_bytes(0, size as usize - 40);
            put(out.add(40), &name(index), wide);
        }
    }
    1
}
unsafe fn put(out: *mut u8, text: &str, wide: bool) {
    if wide {
        for (i, u) in text.encode_utf16().enumerate() {
            unsafe { out.add(i * 2).cast::<u16>().write_unaligned(u) };
        }
    } else {
        unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len()) };
    }
}
pub unsafe extern "win64" fn monitor_info_w(m: usize, out: *mut u8) -> i32 {
    unsafe { monitor_info(m, out, true) }
}
pub unsafe extern "win64" fn monitor_info_a(m: usize, out: *mut u8) -> i32 {
    unsafe { monitor_info(m, out, false) }
}
type MonitorEnum = extern "win64" fn(usize, usize, *mut i32, isize) -> i32;
pub unsafe extern "win64" fn enum_display_monitors(
    hdc: usize,
    clip: *const i32,
    proc: Option<MonitorEnum>,
    data: isize,
) -> i32 {
    if hdc != 0 || !clip.is_null() {
        error::set(50);
        return 0;
    }
    let Some(proc) = proc else {
        error::set(87);
        return 0;
    };
    for m in all() {
        let mut r = [m.x, m.y, m.x + m.width, m.y + m.height];
        if proc(handle(&m), 0, r.as_mut_ptr(), data) == 0 {
            return 0;
        }
    }
    1
}
unsafe fn display_device(device: Option<String>, index: u32, out: *mut u8, wide: bool) -> i32 {
    let needed = if wide { 840 } else { 424 };
    if out.is_null() || unsafe { out.cast::<u32>().read_unaligned() } < needed {
        error::set(87);
        return 0;
    }
    let (id, description, flags) = if device.as_deref().is_none_or(|s| s.is_empty()) {
        let monitors = all();
        let Some(m) = monitors.get(index as usize) else {
            return 0;
        };
        (
            name(index as usize),
            "X11 RandR display adapter",
            1 | if m.primary { 4 } else { 0 },
        )
    } else {
        let Some((i, _)) = named(device.as_deref()) else {
            return 0;
        };
        if index != 0 {
            return 0;
        }
        (format!("{}\\Monitor0", name(i)), "X11 RandR monitor", 1 | 2)
    };
    unsafe {
        out.add(4).write_bytes(0, needed as usize - 4);
        put(out.add(4), &id, wide);
        put(out.add(if wide { 68 } else { 36 }), description, wide);
        out.add(if wide { 324 } else { 164 })
            .cast::<u32>()
            .write_unaligned(flags);
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
    let name = (!device.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(device) }
            .to_string_lossy()
            .into_owned()
    });
    unsafe { display_device(name, index, out, false) }
}
/// EnumDisplaySettings: each monitor lists only the mode it is in. There is no
/// mode switching (ChangeDisplaySettingsEx accepts only the current size), so a
/// listed mode the program could not select would make it wait for a switch that
/// never happens (DOOM Eternal kept a 32x32 window).
unsafe fn display_settings_w(out: *mut u8, mode: u32, monitor: &Monitor) -> i32 {
    let (_, current) = x11::modes::list(monitor);
    let selected = if mode == u32::MAX || mode == u32::MAX - 1 || mode == 0 {
        current
    } else {
        return 0;
    };
    unsafe {
        let extra = out.add(70).cast::<u16>().read_unaligned();
        out.write_bytes(0, 220);
        let index = all().iter().position(|m| m.id == monitor.id).unwrap_or(0);
        put(out, &name(index), true);
        out.add(68).cast::<u16>().write_unaligned(220);
        out.add(70).cast::<u16>().write_unaligned(extra);
        out.add(72)
            .cast::<u32>()
            .write_unaligned(0x20 | 0x40000 | 0x80000 | 0x100000 | 0x400000);
        out.add(76).cast::<i32>().write_unaligned(monitor.x);
        out.add(80).cast::<i32>().write_unaligned(monitor.y);
        for (at, v) in [
            (168, 32u32),
            (172, selected.width),
            (176, selected.height),
            (180, selected.flags),
            (184, selected.hz),
        ] {
            out.add(at).cast::<u32>().write_unaligned(v);
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
    let device = unsafe { text::string(device) };
    let Some((_, m)) = named(device.as_deref()) else {
        error::set(87);
        return 0;
    };
    unsafe { display_settings_w(out, mode, &m) }
}
pub unsafe extern "win64" fn enum_display_settings_ex_w(
    device: *const u16,
    mode: u32,
    out: *mut u8,
    _flags: u32,
) -> i32 {
    unsafe { enum_display_settings_w(device, mode, out) }
}
pub unsafe extern "win64" fn enum_display_settings_a(
    device: *const std::ffi::c_char,
    mode: u32,
    out: *mut u8,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let name = (!device.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(device) }
            .to_string_lossy()
            .into_owned()
    });
    let Some((_, m)) = named(name.as_deref()) else {
        error::set(87);
        return 0;
    };
    let mut w = [0u8; 220];
    if unsafe { display_settings_w(w.as_mut_ptr(), mode, &m) } == 0 {
        return 0;
    }
    unsafe {
        let extra = out.add(38).cast::<u16>().read_unaligned();
        out.write_bytes(0, 156);
        for i in 0..31 {
            *out.add(i) = w[i * 2];
        }
        std::ptr::copy_nonoverlapping(w.as_ptr().add(64), out.add(32), 38);
        std::ptr::copy_nonoverlapping(w.as_ptr().add(166), out.add(102), 54);
        out.add(36).cast::<u16>().write_unaligned(156);
        out.add(38).cast::<u16>().write_unaligned(extra);
    }
    1
}
pub unsafe extern "win64" fn enum_display_settings_ex_a(
    device: *const std::ffi::c_char,
    mode: u32,
    out: *mut u8,
    _flags: u32,
) -> i32 {
    unsafe { enum_display_settings_a(device, mode, out) }
}
pub extern "win64" fn system_metrics(index: i32) -> i32 {
    let (w, h) = screen();
    match index {
        0 | 16 | 61 => w, // SM_CXSCREEN, SM_CXFULLSCREEN, SM_CXMAXIMIZED
        1 | 17 | 62 => h, // SM_CYSCREEN, SM_CYFULLSCREEN, SM_CYMAXIMIZED
        59 => w + 16,     // SM_CXMAXTRACK
        60 => h + 16,     // SM_CYMAXTRACK
        11..=14 => 32,    // SM_CXICON, SM_CYICON, SM_CXCURSOR, SM_CYCURSOR
        49 | 50 => 16,    // SM_CXSMICON, SM_CYSMICON
        28 | 34 => 112,   // SM_CXMIN, SM_CXMINTRACK
        29 | 35 => 27,    // SM_CYMIN, SM_CYMINTRACK
        36 | 37 | 68 | 69 => 4, // SM_CXDOUBLECLK, SM_CYDOUBLECLK, SM_CXDRAG, SM_CYDRAG
        38 | 39 => 75,    // SM_CXICONSPACING, SM_CYICONSPACING
        47 | 57 => 160,   // SM_CXMINSPACING, SM_CXMINIMIZED
        48 | 58 => 24,    // SM_CYMINSPACING, SM_CYMINIMIZED
        71 | 72 => 15,    // SM_CXMENUCHECK, SM_CYMENUCHECK
        83 | 84 => 1,     // SM_CXFOCUSBORDER, SM_CYFOCUSBORDER
        76 => virtual_rect()[0],
        77 => virtual_rect()[1],
        78 => {
            let r = virtual_rect();
            r[2] - r[0]
        }
        79 => {
            let r = virtual_rect();
            r[3] - r[1]
        } // virtual screen origin
        80 => all().len() as i32,        // SM_CMONITORS
        81 | 82 | 75 | 19 => 1,          // same format, IMM, wheel, mouse present
        43 => 5,                         // SM_CMOUSEBUTTONS
        18 | 22..=27 | 40..=42 | 44 | 56 | 63..=67 | 70 | 73 | 74 | 85..=91 | 93..=95 | 0x1000 | 0x2000 | 0x2001 => 0,
        2..=10 | 15 | 20 | 21 | 30..=33 | 45 | 46 | 51..=55 | 92 => 0, // caption/borders/frame/padding: no non-client area
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
        0x29 => {
            // SPI_GETNONCLIENTMETRICS: NONCLIENTMETRICSA (340/344 bytes) or NONCLIENTMETRICSW (500/504 bytes).
            let cb = unsafe { (out as *const u32).read_unaligned() }.max(param) as usize;
            if !(340..=504).contains(&cb) {
                error::set(87);
                return 0;
            }
            let wide = cb >= 500;
            unsafe {
                out.write_bytes(0, cb);
                (out as *mut u32).write_unaligned(cb as u32);
                for (off, val) in [(4, 1i32), (8, 17), (12, 17), (16, 22), (20, 22)] {
                    out.add(off).cast::<i32>().write_unaligned(val);
                }
                let fill_logfont = |base: usize| {
                    out.add(base).cast::<i32>().write_unaligned(-12); // lfHeight
                    out.add(base + 16).cast::<i32>().write_unaligned(400); // lfWeight = FW_NORMAL
                    *out.add(base + 23) = 1; // DEFAULT_CHARSET
                    self::put(out.add(base + 28), "Segoe UI", wide);
                };
                if wide {
                    fill_logfont(24);
                    out.add(116).cast::<i32>().write_unaligned(17);
                    out.add(120).cast::<i32>().write_unaligned(17);
                    fill_logfont(124);
                    out.add(216).cast::<i32>().write_unaligned(19);
                    out.add(220).cast::<i32>().write_unaligned(19);
                    fill_logfont(224);
                    fill_logfont(316);
                    fill_logfont(408);
                } else {
                    fill_logfont(24);
                    out.add(84).cast::<i32>().write_unaligned(17);
                    out.add(88).cast::<i32>().write_unaligned(17);
                    fill_logfont(92);
                    out.add(152).cast::<i32>().write_unaligned(19);
                    out.add(156).cast::<i32>().write_unaligned(19);
                    fill_logfont(160);
                    fill_logfont(220);
                    fill_logfont(280);
                }
            }
        }
        0x30 => {
            // SPI_GETWORKAREA: the whole screen.
            let (w, h) = screen();
            for (i, v) in [0, 0, w, h].into_iter().enumerate() {
                unsafe { (out as *mut i32).add(i).write_unaligned(v) };
            }
        }
        0x42 => {
            // SPI_GETHIGHCONTRAST: HIGHCONTRASTW/A (16 bytes on x64: cbSize(4), dwFlags(4), lpszDefaultScheme(8))
            let cb = unsafe { (out as *const u32).read_unaligned() }.max(param) as usize;
            unsafe {
                let len = cb.max(16);
                out.write_bytes(0, len);
                (out as *mut u32).write_unaligned(cb as u32);
                (out.add(4) as *mut u32).write_unaligned(2); // HCF_AVAILABLE (high contrast off)
            }
        }
        0x43 => {} // SPI_SETHIGHCONTRAST
        0x48 => {
            // SPI_GETANIMATION: ANIMATIONINFO (8 bytes: cbSize(4), iMinAnimate(4))
            unsafe {
                (out as *mut u32).write_unaligned(8);
                (out.add(4) as *mut i32).write_unaligned(0);
            }
        }
        0x04 => {
            // SPI_GETMOUSE: 3 ints
            for (i, v) in [6i32, 10, 1].into_iter().enumerate() {
                unsafe { (out as *mut i32).add(i).write_unaligned(v) };
            }
        }
        0x44 | 0x5E | 0x46 | 0x1A | 0x72 | 0x1000 | 0x1002 | 0x1004 | 0x1006 | 0x1008 | 0x100A
        | 0x100C | 0x100E | 0x1012 | 0x1014 | 0x1016 | 0x1018 | 0x101A | 0x101C | 0x101E
        | 0x1020 | 0x1022 | 0x1024 | 0x1026 | 0x103E | 0x1040 | 0x1042 | 0x104A
        | 0x2000 | 0x2002 | 0x2018 | 0x201A | 0x201E | 0x2020 | 0x2024 => put(0),
        0x17 | 0x4A | 0x1048 | 0x104E | 0x2006 | 0x200E | 0x2010 | 0x2012 => put(1), // font smoothing / ClearType / caret width / focus border
        0x200A => put(2),    // SPI_GETFONTSMOOTHINGTYPE = FE_FONTSMOOTHINGCLEARTYPE
        0x200C => put(1400), // SPI_GETFONTSMOOTHINGCONTRAST
        0x2008 => put(1200), // SPI_GETMOUSECLICKLOCKTIME
        0x2014 => put(5),    // SPI_GETMESSAGEDURATION
        0x2022 => put(5000), // SPI_GETCARETTIMEOUT
        0x16 => put(31),     // SPI_GETKEYBOARDSPEED
        0x70 => put(10),     // SPI_GETMOUSESPEED
        0x66 | 0x6A => put(400), // hover time / menu show delay (Windows default)
        0x68 | 0x6C | 0x2004 => put(3), // wheel scroll lines/chars / foreground flash count
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
