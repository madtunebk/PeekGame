//! The rest of USER32 a game touches: TranslateMessage, device contexts, the
//! clipboard, message boxes and dialogs, IME contexts, DWM attributes, DPI,
//! desktop objects, device notifications and rectangle helpers.
use super::{
    queue,
    state::{self, windows},
    strings,
    types::*,
};
use crate::api::system::error;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
/// TranslateMessage: WM_CHAR (WM_SYSCHAR) for a key press that types a character.
pub(crate) unsafe extern "win64" fn translate_message(message: *const Message) -> i32 {
    if message.is_null() {
        return fail(87, 0);
    }
    let m = unsafe { *message };
    if !matches!(
        m.message,
        WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP
    ) {
        return 0;
    }
    if (m.message == WM_KEYDOWN || m.message == WM_SYSKEYDOWN) && m.private != 0 {
        let char_message = if m.message == WM_KEYDOWN {
            WM_CHAR
        } else {
            WM_SYSCHAR
        };
        let mut units = [0u16; 2];
        let chars = char::from_u32(m.private)
            .map(|c| c.encode_utf16(&mut units).len())
            .unwrap_or(0);
        for &unit in &units[..chars] {
            queue::post_message_w(m.window, char_message, unit as usize, m.lparam);
        }
    }
    1
}

// ---- device contexts and GDI --------------------------------------------------------
static DCS: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);
static NEXT_DC: AtomicUsize = AtomicUsize::new(0x0010_0000);
/// A device context: a token naming its window (or the screen). PeekGame draws
/// nothing through GDI; games render with Direct3D.
pub(crate) extern "win64" fn get_dc(hwnd: usize) -> usize {
    if hwnd != 0 && hwnd != DESKTOP && !state::exists(hwnd) {
        return fail(ERROR_WINDOW, 0);
    }
    let dc = NEXT_DC.fetch_add(4, Ordering::Relaxed);
    DCS.lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(dc, if hwnd == 0 { DESKTOP } else { hwnd });
    dc
}
pub(crate) extern "win64" fn get_dc_ex(hwnd: usize, _clip: usize, _flags: u32) -> usize {
    get_dc(hwnd)
}
pub(crate) extern "win64" fn get_window_dc(hwnd: usize) -> usize {
    get_dc(hwnd)
}
pub(crate) extern "win64" fn release_dc(_hwnd: usize, dc: usize) -> i32 {
    DCS.lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .remove(&dc)
        .is_some() as i32
}
pub(crate) extern "win64" fn window_from_dc(dc: usize) -> usize {
    DCS.lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .get(&dc)
        .copied()
        .unwrap_or(0)
}
/// GetDeviceCaps for the screen: sizes, 32 bits per pixel, 96 DPI, 60 Hz.
pub(crate) extern "win64" fn get_device_caps(_dc: usize, index: i32) -> i32 {
    let (w, h) = crate::host::x11::primary_size().unwrap_or((1920, 1080));
    match index {
        0 => 0x0100,        // DRIVERVERSION
        2 => 1,             // TECHNOLOGY: DT_RASDISPLAY
        4 => w * 254 / 960, // HORZSIZE (mm at 96 DPI)
        6 => h * 254 / 960, // VERTSIZE
        8 | 118 => w,       // HORZRES, DESKTOPHORZRES
        10 | 117 => h,      // VERTRES, DESKTOPVERTRES
        12 => 32,           // BITSPIXEL
        14 => 1,            // PLANES
        24 => -1,           // NUMCOLORS
        88 | 90 => 96,      // LOGPIXELSX/Y
        116 => 60,          // VREFRESH
        _ => 0,
    }
}
pub(crate) unsafe extern "win64" fn get_text_charset_info(
    _dc: usize,
    sig: *mut u32,
    _flags: u32,
) -> i32 {
    if !sig.is_null() {
        unsafe {
            sig.write_bytes(0, 6);
            sig.write_unaligned(1);
            sig.add(4).write_unaligned(1);
        }
    }
    0 // ANSI_CHARSET
}
type FontEnumProcA = unsafe extern "win64" fn(*const u8, *const u8, u32, isize) -> i32;
pub(crate) unsafe extern "win64" fn enum_font_families_a(
    _dc: usize,
    family: *const i8,
    proc: Option<FontEnumProcA>,
    lparam: isize,
) -> i32 {
    let Some(proc) = proc else {
        return 1;
    };
    let mut elf = [0u8; 348]; // ENUMLOGFONTEXA (LOGFONTA 60 + FullName 64 + Style 32 + Script 32 + ...)
    let mut tm = [0u8; 76];   // NEWTEXTMETRICA
    let name = if family.is_null() {
        "Segoe UI".to_owned()
    } else {
        unsafe { strings::narrow(family) }
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Segoe UI".to_owned())
    };
    let bytes = name.as_bytes();
    let n = bytes.len().min(31);
    unsafe {
        elf.as_mut_ptr().cast::<i32>().write_unaligned(16); // lfHeight
        elf.as_mut_ptr().add(16).cast::<i32>().write_unaligned(400); // lfWeight
        elf[23] = 0; // ANSI_CHARSET
        elf[28..28 + n].copy_from_slice(&bytes[..n]);
        elf[60..60 + n].copy_from_slice(&bytes[..n]);
        elf[124..124 + 7].copy_from_slice(b"Regular");
        tm.as_mut_ptr().cast::<i32>().write_unaligned(16); // tmHeight
        tm.as_mut_ptr().add(4).cast::<i32>().write_unaligned(13); // tmAscent
        tm.as_mut_ptr().add(8).cast::<i32>().write_unaligned(3); // tmDescent
        tm.as_mut_ptr().add(20).cast::<i32>().write_unaligned(8); // tmAveCharWidth
        tm.as_mut_ptr().add(24).cast::<i32>().write_unaligned(16); // tmMaxCharWidth
        tm.as_mut_ptr().add(28).cast::<i32>().write_unaligned(400); // tmWeight
        tm[44] = 0x20; // tmFirstChar
        tm[45] = 0x7e; // tmLastChar
        tm[46] = 0x20; // tmDefaultChar
        tm[47] = 0x20; // tmBreakChar
        proc(elf.as_ptr(), tm.as_ptr(), 4, lparam) // 4 = TRUETYPE_FONTTYPE
    }
}
pub(crate) unsafe extern "win64" fn get_text_metrics_a(_dc: usize, out: *mut u8) -> i32 {
    if out.is_null() {
        return 0;
    }
    unsafe {
        out.write_bytes(0, 56);
        out.cast::<i32>().write_unaligned(16); // tmHeight
        out.add(4).cast::<i32>().write_unaligned(13); // tmAscent
        out.add(8).cast::<i32>().write_unaligned(3); // tmDescent
        out.add(12).cast::<i32>().write_unaligned(3); // tmInternalLeading
        out.add(20).cast::<i32>().write_unaligned(7); // tmAveCharWidth
        out.add(24).cast::<i32>().write_unaligned(14); // tmMaxCharWidth
        out.add(28).cast::<i32>().write_unaligned(400); // tmWeight
        out.add(36).cast::<i32>().write_unaligned(96); // tmDigitizedAspectX
        out.add(40).cast::<i32>().write_unaligned(96); // tmDigitizedAspectY
        *out.add(44) = 0x20; // tmFirstChar
        *out.add(45) = 0x7e; // tmLastChar
        *out.add(46) = 0x20; // tmDefaultChar
        *out.add(47) = 0x20; // tmBreakChar
        *out.add(51) = 0x04; // TMPF_TRUETYPE
    }
    1
}

// ---- clipboard (this process) ---------------------------------------------------------
struct Clipboard {
    open: bool,
    data: HashMap<u32, usize>,
}
static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);
fn clipboard<T>(f: impl FnOnce(&mut Clipboard) -> T) -> T {
    f(CLIPBOARD.lock().unwrap().get_or_insert_with(|| Clipboard {
        open: false,
        data: HashMap::new(),
    }))
}
/// The clipboard holds data within this process (handles from GlobalAlloc, as
/// programs pass them); the X selection is not connected.
pub(crate) extern "win64" fn open_clipboard(_owner: usize) -> i32 {
    clipboard(|c| {
        if c.open {
            fail(5, 0)
        } else {
            c.open = true;
            1
        }
    })
}
pub(crate) extern "win64" fn close_clipboard() -> i32 {
    clipboard(|c| std::mem::replace(&mut c.open, false) as i32)
}
pub(crate) extern "win64" fn empty_clipboard() -> i32 {
    clipboard(|c| {
        c.data.clear();
        1
    })
}
pub(crate) extern "win64" fn set_clipboard_data(format: u32, data: usize) -> usize {
    clipboard(|c| {
        c.data.insert(format, data);
        data
    })
}
pub(crate) extern "win64" fn get_clipboard_data(format: u32) -> usize {
    clipboard(|c| c.data.get(&format).copied().unwrap_or(0))
}
pub(crate) extern "win64" fn is_clipboard_format_available(format: u32) -> i32 {
    clipboard(|c| c.data.contains_key(&format) as i32)
}
pub(crate) unsafe extern "win64" fn register_clipboard_format_w(name: *const u16) -> u32 {
    unsafe { register_window_message_w(name) }
}
pub(crate) unsafe extern "win64" fn register_clipboard_format_a(name: *const i8) -> u32 {
    unsafe { register_window_message_a(name) }
}
pub(crate) unsafe extern "win64" fn register_window_message_w(name: *const u16) -> u32 {
    match unsafe { strings::wide(name) } {
        Ok(n) => queue::register_message(&n),
        Err(e) => fail(e, 0),
    }
}
pub(crate) unsafe extern "win64" fn register_window_message_a(name: *const i8) -> u32 {
    match unsafe { strings::narrow(name) } {
        Ok(n) => queue::register_message(&n),
        Err(e) => fail(e, 0),
    }
}

// ---- message boxes and dialogs ------------------------------------------------------------
/// MessageBox: a real modal dialog through zenity, returning the button the user
/// pressed (closing the dialog is Cancel, or the last button). The text is also
/// logged. Without zenity or a display the call fails: 0 with ERROR_NOT_SUPPORTED.
fn message_box(caption: &str, text: &str, kind: u32) -> i32 {
    const IDOK: i32 = 1;
    const IDCANCEL: i32 = 2;
    const IDABORT: i32 = 3;
    const IDRETRY: i32 = 4;
    const IDIGNORE: i32 = 5;
    const IDYES: i32 = 6;
    const IDNO: i32 = 7;
    const IDTRYAGAIN: i32 = 10;
    const IDCONTINUE: i32 = 11;
    eprintln!("MessageBox [{caption}]: {text}");
    // (first button, middle button, last button); a closed dialog answers with the last.
    let buttons: &[(&str, i32)] = match kind & 0xf {
        0 => &[("OK", IDOK)],
        1 => &[("OK", IDOK), ("Cancel", IDCANCEL)],
        2 => &[("Abort", IDABORT), ("Retry", IDRETRY), ("Ignore", IDIGNORE)],
        3 => &[("Yes", IDYES), ("No", IDNO), ("Cancel", IDCANCEL)],
        4 => &[("Yes", IDYES), ("No", IDNO)],
        5 => &[("Retry", IDRETRY), ("Cancel", IDCANCEL)],
        6 => &[("Continue", IDCONTINUE), ("Try Again", IDTRYAGAIN), ("Cancel", IDCANCEL)],
        _ => return fail(87, 0),
    };
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return fail(50, 0);
    }
    let mut command = std::process::Command::new("zenity");
    command.arg(format!("--title={caption}")).arg(format!("--text={text}")).arg("--no-markup");
    if buttons.len() == 1 {
        command.arg(match kind & 0xf0 {
            0x10 => "--error",
            0x30 => "--warning",
            _ => "--info",
        });
    } else {
        let last = buttons[buttons.len() - 1].0;
        command.arg("--question").arg(format!("--ok-label={}", buttons[0].0)).arg(format!("--cancel-label={last}"));
        if buttons.len() == 3 {
            command.arg(format!("--extra-button={}", buttons[1].0));
        }
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(_) => return fail(50, 0),
    };
    // GTK reports a display it cannot open on stderr and exits 1, like "Cancel".
    if String::from_utf8_lossy(&output.stderr).contains("cannot open display") {
        return fail(50, 0);
    }
    if buttons.len() == 1 {
        return IDOK;
    }
    let pressed = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.code() == Some(0) {
        buttons[0].1
    } else if buttons.len() == 3 && pressed == buttons[1].0 {
        buttons[1].1
    } else if output.status.code() == Some(1) {
        buttons[buttons.len() - 1].1
    } else {
        fail(50, 0) // zenity could not show the dialog (no display)
    }
}
pub(crate) unsafe extern "win64" fn message_box_w(
    _owner: usize,
    text: *const u16,
    caption: *const u16,
    kind: u32,
) -> i32 {
    let text = unsafe { strings::wide(text) }.unwrap_or_default();
    let caption = unsafe { strings::wide(caption) }.unwrap_or_default();
    message_box(&caption, &text, kind)
}
pub(crate) unsafe extern "win64" fn message_box_a(
    _owner: usize,
    text: *const i8,
    caption: *const i8,
    kind: u32,
) -> i32 {
    let text = unsafe { strings::narrow(text) }.unwrap_or_default();
    let caption = unsafe { strings::narrow(caption) }.unwrap_or_default();
    message_box(&caption, &text, kind)
}
/// Dialog templates (resources) are not run: DialogBoxParam fails with -1.
pub(crate) extern "win64" fn dialog_box_param_w(
    _instance: usize,
    template: usize,
    _owner: usize,
    _procedure: usize,
    _parameter: isize,
) -> isize {
    eprintln!(
        "DialogBoxParam: dialog resource {template:#x} is not shown (dialogs are not supported)"
    );
    fail(50, -1)
}
pub(crate) extern "win64" fn end_dialog(_dialog: usize, _result: isize) -> i32 {
    1
}
pub(crate) extern "win64" fn set_dlg_item_text(_dialog: usize, _item: i32, _text: usize) -> i32 {
    fail(ERROR_WINDOW, 0)
}
pub(crate) extern "win64" fn send_dlg_item_message_w(
    _dialog: usize,
    _item: i32,
    _message: u32,
    _wparam: usize,
    _lparam: isize,
) -> isize {
    0
}

// ---- IME (imm32): no input method; composition is empty -----------------------------------
pub(crate) extern "win64" fn imm_get_context(hwnd: usize) -> usize {
    if state::exists(hwnd) {
        0x0009_0000 | (hwnd & 0xffff)
    } else {
        0
    }
}
pub(crate) extern "win64" fn imm_release_context(_hwnd: usize, _context: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_associate_context(hwnd: usize, _context: usize) -> usize {
    imm_get_context(hwnd)
}
pub(crate) extern "win64" fn imm_associate_context_ex(
    _hwnd: usize,
    _context: usize,
    _flags: u32,
) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_get_composition_string_w(
    _context: usize,
    _index: u32,
    _out: usize,
    _size: u32,
) -> i32 {
    0
}
pub(crate) extern "win64" fn imm_set_composition_string_w(
    _context: usize,
    _index: u32,
    _a: usize,
    _b: u32,
    _c: usize,
    _d: u32,
) -> i32 {
    0
}
pub(crate) extern "win64" fn imm_notify_ime(
    _context: usize,
    _action: u32,
    _index: u32,
    _value: u32,
) -> i32 {
    1
}
pub(crate) unsafe extern "win64" fn imm_get_conversion_status(
    _context: usize,
    conversion: *mut u32,
    sentence: *mut u32,
) -> i32 {
    unsafe {
        if !conversion.is_null() {
            *conversion = 0;
        }
        if !sentence.is_null() {
            *sentence = 0;
        }
    }
    1
}
pub(crate) extern "win64" fn imm_set_conversion_status(
    _context: usize,
    _conversion: u32,
    _sentence: u32,
) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_set_open_status(_context: usize, _open: i32) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_get_open_status(_context: usize) -> i32 {
    0
}
pub(crate) extern "win64" fn imm_set_candidate_window(_context: usize, _form: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_set_composition_window(_context: usize, _form: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_create_context() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    0x0009_0000 | (NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) & 0xffff)
}
pub(crate) extern "win64" fn imm_destroy_context(_context: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_get_property(_hkl: usize, index: u32) -> u32 {
    match index {
        4 => 0x0008_0000, // IGP_PROPERTY: IME_PROP_UNICODE
        _ => 0,
    }
}
pub(crate) extern "win64" fn imm_is_ime(_hkl: usize) -> i32 {
    0
}
pub(crate) extern "win64" fn imm_get_ime_file_name_w(_hkl: usize, _name: *mut u16, _size: u32) -> u32 {
    0
}
pub(crate) extern "win64" fn imm_get_ime_file_name_a(_hkl: usize, _name: *mut u8, _size: u32) -> u32 {
    0
}

// ---- DWM ------------------------------------------------------------------------------------
/// DwmGetWindowAttribute: DWMWA_EXTENDED_FRAME_BOUNDS (9) is the window rectangle;
/// DWMWA_CLOAKED (14) is 0; others E_INVALIDARG.
pub(crate) unsafe extern "win64" fn dwm_get_window_attribute(
    hwnd: usize,
    attribute: u32,
    out: *mut u8,
    size: u32,
) -> i32 {
    const E_INVALIDARG: i32 = 0x8007_0057u32 as i32;
    if out.is_null() {
        return E_INVALIDARG;
    }
    match attribute {
        9 if size >= 16 => match super::windows::screen_rect(hwnd) {
            Some(r) => {
                unsafe { out.cast::<Rect>().write_unaligned(r) };
                0
            }
            None => 0x8007_0578u32 as i32, // HRESULT_FROM_WIN32(ERROR_INVALID_WINDOW_HANDLE)
        },
        14 if size >= 4 => {
            unsafe { out.cast::<u32>().write_unaligned(0) };
            0
        }
        _ => E_INVALIDARG,
    }
}
/// DwmSetWindowAttribute: dark mode, corners, backdrops are the compositor's; S_OK.
pub(crate) extern "win64" fn dwm_set_window_attribute(
    _hwnd: usize,
    _attribute: u32,
    _value: usize,
    _size: u32,
) -> i32 {
    0
}
pub(crate) unsafe extern "win64" fn dwm_is_composition_enabled(enabled: *mut i32) -> i32 {
    if !enabled.is_null() {
        unsafe { *enabled = 1 };
    }
    0
}
pub(crate) extern "win64" fn dwm_flush() -> i32 {
    0
}
pub(crate) unsafe extern "win64" fn dwm_enable_blur_behind_window(hwnd: usize, _blur: *const std::ffi::c_void) -> i32 {
    if !state::exists(hwnd) {
        return 0x8007_0578u32 as i32; // HRESULT_FROM_WIN32(ERROR_INVALID_WINDOW_HANDLE)
    }
    0
}

// ---- DPI: physical pixels at 96 DPI ---------------------------------------------------------
pub(crate) extern "win64" fn get_dpi_for_window(hwnd: usize) -> u32 {
    if state::exists(hwnd) || hwnd == DESKTOP {
        96
    } else {
        fail(87, 0)
    }
}
pub(crate) extern "win64" fn get_dpi_for_system() -> u32 {
    96
}
pub(crate) extern "win64" fn set_process_dpi_aware() -> i32 {
    1
}
pub(crate) extern "win64" fn set_process_dpi_awareness_context(_context: isize) -> i32 {
    1
}
pub(crate) extern "win64" fn set_thread_dpi_awareness_context(_context: isize) -> isize {
    -4 // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
}
pub(crate) extern "win64" fn get_system_metrics_for_dpi(index: i32, _dpi: u32) -> i32 {
    super::display::system_metrics(index)
}
pub(crate) extern "win64" fn enable_non_client_dpi_scaling(_hwnd: usize) -> i32 {
    1
}
/// shcore SetProcessDpiAwareness: S_OK.
pub(crate) extern "win64" fn set_process_dpi_awareness(_value: u32) -> i32 {
    0
}
pub(crate) unsafe extern "win64" fn get_dpi_for_monitor(
    _monitor: usize,
    _kind: u32,
    x: *mut u32,
    y: *mut u32,
) -> i32 {
    unsafe {
        if !x.is_null() {
            *x = 96;
        }
        if !y.is_null() {
            *y = 96;
        }
    }
    0
}
pub(crate) unsafe extern "win64" fn get_process_dpi_awareness(
    _process: usize,
    out: *mut u32,
) -> i32 {
    if !out.is_null() {
        unsafe { *out = 2 };
    }
    0
}

// ---- pointer input: mouse messages stay classic -------------------------------------------
pub(crate) extern "win64" fn enable_mouse_in_pointer(_enable: i32) -> i32 {
    fail(50, 0)
}
pub(crate) extern "win64" fn is_mouse_in_pointer_enabled() -> i32 {
    0
}
pub(crate) extern "win64" fn get_pointer_info(_id: u32, _out: usize) -> i32 {
    fail(87, 0)
}
pub(crate) extern "win64" fn register_touch_window(_hwnd: usize, _flags: u32) -> i32 {
    1
}
pub(crate) extern "win64" fn unregister_touch_window(_hwnd: usize) -> i32 {
    1
}

// ---- desktop objects, processes, notifications ---------------------------------------------
pub(crate) extern "win64" fn get_thread_desktop(_thread: u32) -> usize {
    0x000a_0010
}
pub(crate) unsafe extern "win64" fn create_desktop_w(
    _desktop: *const u16,
    _device: *const u16,
    _devmode: *const std::ffi::c_void,
    _flags: u32,
    _access: u32,
    _sa: *const std::ffi::c_void,
) -> usize {
    0x000a_0010
}
pub(crate) unsafe extern "win64" fn create_desktop_a(
    _desktop: *const std::ffi::c_char,
    _device: *const std::ffi::c_char,
    _devmode: *const std::ffi::c_void,
    _flags: u32,
    _access: u32,
    _sa: *const std::ffi::c_void,
) -> usize {
    0x000a_0010
}
pub(crate) unsafe extern "win64" fn open_desktop_w(
    _desktop: *const u16,
    _flags: u32,
    _inherit: i32,
    _access: u32,
) -> usize {
    0x000a_0010
}
pub(crate) unsafe extern "win64" fn open_desktop_a(
    _desktop: *const std::ffi::c_char,
    _flags: u32,
    _inherit: i32,
    _access: u32,
) -> usize {
    0x000a_0010
}
pub(crate) extern "win64" fn set_thread_desktop(_desktop: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn close_desktop(_desktop: usize) -> i32 {
    1
}
/// GetProcessWindowStation: the interactive window station, WinSta0.
const WINDOW_STATION: usize = 0x000a_0020;
pub(crate) extern "win64" fn get_process_window_station() -> usize {
    WINDOW_STATION
}
pub(crate) unsafe extern "win64" fn create_window_station_w(
    _winsta: *const u16,
    _flags: u32,
    _access: u32,
    _sa: *const std::ffi::c_void,
) -> usize {
    WINDOW_STATION
}
pub(crate) unsafe extern "win64" fn create_window_station_a(
    _winsta: *const std::ffi::c_char,
    _flags: u32,
    _access: u32,
    _sa: *const std::ffi::c_void,
) -> usize {
    WINDOW_STATION
}
pub(crate) unsafe extern "win64" fn open_window_station_w(
    _winsta: *const u16,
    _inherit: i32,
    _access: u32,
) -> usize {
    WINDOW_STATION
}
pub(crate) unsafe extern "win64" fn open_window_station_a(
    _winsta: *const std::ffi::c_char,
    _inherit: i32,
    _access: u32,
) -> usize {
    WINDOW_STATION
}
pub(crate) extern "win64" fn set_process_window_station(_winsta: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn close_window_station(_winsta: usize) -> i32 {
    1
}
fn object_name(object: usize) -> &'static str {
    if object == WINDOW_STATION {
        "WinSta0"
    } else {
        "Default"
    }
}
/// GetUserObjectInformationA: UOI_FLAGS (1: visible window station), UOI_NAME (2).
pub(crate) unsafe extern "win64" fn get_user_object_information_a(
    object: usize,
    index: i32,
    out: *mut u8,
    size: u32,
    needed: *mut u32,
) -> i32 {
    let value: Vec<u8> = match index {
        1 => [0u32, 0, 1].iter().flat_map(|v| v.to_le_bytes()).collect(),
        2 => format!("{}\0", object_name(object)).into_bytes(),
        _ => return fail(87, 0),
    };
    if !needed.is_null() {
        unsafe { *needed = value.len() as u32 };
    }
    if out.is_null() || (size as usize) < value.len() {
        return fail(122, 0);
    }
    unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), out, value.len()) };
    1
}
pub(crate) unsafe extern "win64" fn get_user_object_information_w(
    object: usize,
    index: i32,
    out: *mut u8,
    size: u32,
    needed: *mut u32,
) -> i32 {
    if index != 2 {
        return unsafe { get_user_object_information_a(object, index, out, size, needed) };
    }
    let value: Vec<u8> = format!("{}\0", object_name(object))
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    if !needed.is_null() {
        unsafe { *needed = value.len() as u32 };
    }
    if out.is_null() || (size as usize) < value.len() {
        return fail(122, 0);
    }
    unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), out, value.len()) };
    1
}
pub(crate) extern "win64" fn wait_for_input_idle(_process: usize, _ms: u32) -> u32 {
    0
}
pub(crate) extern "win64" fn disable_process_windows_ghosting() {}
/// RegisterDeviceNotification: accepted; device arrival/removal is not reported.
pub(crate) extern "win64" fn register_device_notification_w(
    _recipient: usize,
    _filter: usize,
    _flags: u32,
) -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0x000b_0000);
    NEXT.fetch_add(4, Ordering::Relaxed)
}
pub(crate) extern "win64" fn unregister_device_notification(_handle: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn register_suspend_resume_notification(
    _recipient: usize,
    _flags: u32,
) -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0x000b_8000);
    NEXT.fetch_add(4, Ordering::Relaxed)
}
pub(crate) extern "win64" fn unregister_suspend_resume_notification(_handle: usize) -> i32 {
    1
}
/// Icons: handles that are never drawn (the compositor shows the program's icon
/// from its desktop entry, if any).
/// LoadIcon: the system icons (no module, IDI_APPLICATION..IDI_SHIELD) and icons
/// named in a program module are icon tokens; any other system name is
/// ERROR_RESOURCE_NAME_NOT_FOUND. Program icon resources are not checked or decoded.
pub(crate) extern "win64" fn load_icon(instance: usize, name: usize) -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0x000c_0000);
    if name == 0 || (instance == 0 && !(32512..=32518).contains(&name)) {
        return fail(1814, 0);
    }
    NEXT.fetch_add(4, Ordering::Relaxed)
}
/// LoadImageW: system cursors (IMAGE_CURSOR 2 with OIC/OCR ids and no module) are
/// cursors; other images are icon-like handles.
pub(crate) extern "win64" fn load_image_w(
    instance: usize,
    name: *const u16,
    kind: u32,
    _cx: i32,
    _cy: i32,
    _flags: u32,
) -> usize {
    if kind == 2 {
        return super::input::load_cursor_w(instance, name);
    }
    if kind == 0 {
        return 0x000d_0000; // a bitmap handle nothing draws
    }
    load_icon(instance, name as usize)
}
pub(crate) extern "win64" fn create_icon_indirect(_info: usize) -> usize {
    load_icon(0, 32512)
}
pub(crate) extern "win64" fn set_parent(_child: usize, _parent: usize) -> usize {
    DESKTOP
}
pub(crate) extern "win64" fn is_child(_parent: usize, _child: usize) -> i32 {
    0
}
pub(crate) extern "win64" fn vk_key_scan_ex_w(ch: u16, _hkl: usize) -> i16 {
    match ch as u8 {
        b'a'..=b'z' => (ch as u8 - b'a' + b'A') as i16,
        b'A'..=b'Z' => ((ch as u8) as i16) | 0x0100,
        b'0'..=b'9' => (ch as u8) as i16,
        b' ' => 0x20,
        b'\r' | b'\n' => 0x0d,
        b'\t' => 0x09,
        0x1b => 0x1b,
        _ => -1,
    }
}
pub(crate) unsafe extern "win64" fn get_window_display_affinity(
    _hwnd: usize,
    out: *mut u32,
) -> i32 {
    if !out.is_null() {
        unsafe { *out = 0 };
    }
    1
}
pub(crate) unsafe extern "win64" fn change_display_settings_w(
    mode: *const u8,
    flags: u32,
) -> i32 {
    unsafe {
        crate::api::system::compat::change_display_settings_ex_w(
            std::ptr::null(),
            mode,
            0,
            flags,
            0,
        )
    }
}
pub(crate) extern "win64" fn is_dialog_message_w(_hwnd: usize, _msg: usize) -> i32 {
    0
}
#[allow(clippy::too_many_arguments)]
pub(crate) extern "win64" fn draw_icon_ex(
    _dc: usize,
    _x: i32,
    _y: i32,
    _icon: usize,
    _w: i32,
    _h: i32,
    _step: u32,
    _br: usize,
    _flags: u32,
) -> i32 {
    1
}
pub(crate) extern "win64" fn get_class_long_ptr_w(_hwnd: usize, _index: i32) -> usize {
    0
}
pub(crate) extern "win64" fn fill_rect(_dc: usize, _rect: usize, _brush: usize) -> i32 {
    1
}
pub(crate) extern "win64" fn set_layered_window_attributes(
    _hwnd: usize,
    _key: u32,
    _alpha: u8,
    _flags: u32,
) -> i32 {
    1
}
pub(crate) extern "win64" fn register_hot_key(
    _hwnd: usize,
    _id: i32,
    _mod: u32,
    _vk: u32,
) -> i32 {
    1
}
pub(crate) extern "win64" fn unregister_hot_key(_hwnd: usize, _id: i32) -> i32 {
    1
}
pub(crate) extern "win64" fn get_dlg_item(_dlg: usize, _id: i32) -> usize {
    0
}
pub(crate) unsafe extern "win64" fn get_dlg_item_text_w(
    _dlg: usize,
    _id: i32,
    out: *mut u16,
    cch: i32,
) -> u32 {
    if !out.is_null() && cch > 0 {
        unsafe { *out = 0 };
    }
    0
}
#[allow(clippy::too_many_arguments)]
pub(crate) extern "win64" fn draw_state_w(
    _dc: usize,
    _br: usize,
    _cb: usize,
    _lp: usize,
    _wp: usize,
    _x: i32,
    _y: i32,
    _cx: i32,
    _cy: i32,
    _flags: u32,
) -> i32 {
    1
}
pub(crate) extern "win64" fn draw_text_w(
    _dc: usize,
    _str: *const u16,
    _count: i32,
    _rect: usize,
    _fmt: u32,
) -> i32 {
    16
}
pub(crate) extern "win64" fn create_popup_menu() -> usize {
    0x000e_8000
}
pub(crate) extern "win64" fn insert_menu_item_w(
    _menu: usize,
    _item: u32,
    _by_pos: i32,
    _info: usize,
) -> i32 {
    1
}
#[allow(clippy::too_many_arguments)]
pub(crate) extern "win64" fn track_popup_menu(
    _menu: usize,
    _flags: u32,
    _x: i32,
    _y: i32,
    _res: i32,
    _hwnd: usize,
    _rect: usize,
) -> i32 {
    0
}
pub(crate) extern "win64" fn delete_menu(_menu: usize, _pos: u32, _flags: u32) -> i32 {
    1
}
pub(crate) extern "win64" fn imm_get_description_w(
    _hkl: usize,
    _buf: *mut u16,
    _cch: u32,
) -> u32 {
    0
}
pub(crate) unsafe extern "win64" fn dwm_get_composition_timing_info(
    _hwnd: usize,
    info: *mut u8,
) -> i32 {
    if info.is_null() {
        return -2147024809;
    }
    let size = unsafe { (info as *const u32).read_unaligned() } as usize;
    if size > 4 && size <= 512 {
        unsafe { info.add(4).write_bytes(0, size - 4) };
        if size >= 12 {
            unsafe {
                (info.add(4) as *mut u32).write_unaligned(60);
                (info.add(8) as *mut u32).write_unaligned(1);
            }
        }
    }
    0
}
pub(crate) extern "win64" fn set_window_theme(
    _hwnd: usize,
    _sub_app: *const u16,
    _sub_id: *const u16,
) -> i32 {
    0
}

// ---- rectangles -----------------------------------------------------------------------------
pub(crate) unsafe extern "win64" fn copy_rect(out: *mut Rect, source: *const Rect) -> i32 {
    if out.is_null() || source.is_null() {
        return 0;
    }
    unsafe { *out = *source };
    1
}
pub(crate) unsafe extern "win64" fn set_rect(
    out: *mut Rect,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    unsafe {
        *out = Rect {
            left,
            top,
            right,
            bottom,
        }
    };
    1
}
pub(crate) unsafe extern "win64" fn set_rect_empty(out: *mut Rect) -> i32 {
    unsafe { set_rect(out, 0, 0, 0, 0) }
}
pub(crate) unsafe extern "win64" fn offset_rect(rect: *mut Rect, dx: i32, dy: i32) -> i32 {
    if rect.is_null() {
        return 0;
    }
    let r = unsafe { &mut *rect };
    r.left += dx;
    r.right += dx;
    r.top += dy;
    r.bottom += dy;
    1
}
pub(crate) unsafe extern "win64" fn inflate_rect(rect: *mut Rect, dx: i32, dy: i32) -> i32 {
    if rect.is_null() {
        return 0;
    }
    let r = unsafe { &mut *rect };
    r.left -= dx;
    r.right += dx;
    r.top -= dy;
    r.bottom += dy;
    1
}
/// PtInRect takes the POINT by value (8 bytes in one register).
pub(crate) unsafe extern "win64" fn pt_in_rect(rect: *const Rect, point: u64) -> i32 {
    if rect.is_null() {
        return 0;
    }
    let (x, y) = (point as u32 as i32, (point >> 32) as u32 as i32);
    let r = unsafe { *rect };
    (x >= r.left && x < r.right && y >= r.top && y < r.bottom) as i32
}
pub(crate) unsafe extern "win64" fn is_rect_empty(rect: *const Rect) -> i32 {
    if rect.is_null() {
        return 1;
    }
    let r = unsafe { *rect };
    (r.right <= r.left || r.bottom <= r.top) as i32
}
pub(crate) unsafe extern "win64" fn intersect_rect(
    out: *mut Rect,
    a: *const Rect,
    b: *const Rect,
) -> i32 {
    if out.is_null() || a.is_null() || b.is_null() {
        return 0;
    }
    let (a, b) = unsafe { (*a, *b) };
    let r = Rect {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    };
    let empty = r.right <= r.left || r.bottom <= r.top;
    unsafe { *out = if empty { Rect::default() } else { r } };
    (!empty) as i32
}
pub(crate) unsafe extern "win64" fn union_rect(
    out: *mut Rect,
    a: *const Rect,
    b: *const Rect,
) -> i32 {
    if out.is_null() || a.is_null() || b.is_null() {
        return 0;
    }
    let (a, b) = unsafe { (*a, *b) };
    unsafe {
        *out = Rect {
            left: a.left.min(b.left),
            top: a.top.min(b.top),
            right: a.right.max(b.right),
            bottom: a.bottom.max(b.bottom),
        }
    };
    1
}
pub(crate) unsafe extern "win64" fn equal_rect(a: *const Rect, b: *const Rect) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    (unsafe { *a } == unsafe { *b }) as i32
}
/// WindowFromPoint: the program window under a screen point (POINT by value).
pub(crate) extern "win64" fn window_from_point(point: u64) -> usize {
    let (x, y) = (point as u32 as i32, (point >> 32) as u32 as i32);
    let list: Vec<usize> = windows()
        .iter()
        .filter(|(_, w)| w.visible && w.native != 0)
        .map(|(&h, _)| h)
        .collect();
    list.into_iter()
        .rev()
        .find(|&h| {
            super::windows::screen_rect(h)
                .is_some_and(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom)
        })
        .unwrap_or(0)
}
/// Display configuration (QueryDisplayConfig): not available; programs fall back
/// to EnumDisplaySettings.
pub(crate) extern "win64" fn get_display_config_buffer_sizes(
    _flags: u32,
    _paths: *mut u32,
    _modes: *mut u32,
) -> i32 {
    50 // ERROR_NOT_SUPPORTED
}
pub(crate) extern "win64" fn query_display_config(
    _flags: u32,
    _a: usize,
    _b: usize,
    _c: usize,
    _d: usize,
    _e: usize,
) -> i32 {
    50
}
pub(crate) extern "win64" fn display_config_get_device_info(_packet: usize) -> i32 {
    50
}

/// All current runtime windows use physical pixels at 96 DPI: both directions
/// are identity transforms, with the Windows HWND/POINT validity checks.
pub(crate) extern "win64" fn per_monitor_point(
    hwnd: usize,
    point: *mut super::types::Point,
) -> i32 {
    if point.is_null() {
        return fail(87, 0);
    }
    if !state::exists(hwnd) {
        return fail(1400, 0);
    }
    1
}
/// Standard Windows 10/11 default system colors (0x00bbggrr) for COLOR_* indices 0..=30.
pub(crate) extern "win64" fn get_sys_color(index: i32) -> u32 {
    match index {
        0 => 0x00c8c8c8,  // COLOR_SCROLLBAR
        1 => 0x00000000,  // COLOR_BACKGROUND
        2 => 0x00d1b499,  // COLOR_ACTIVECAPTION
        3 => 0x00dbcdbf,  // COLOR_INACTIVECAPTION
        4 => 0x00f0f0f0,  // COLOR_MENU
        5 => 0x00ffffff,  // COLOR_WINDOW
        6 => 0x00646464,  // COLOR_WINDOWFRAME
        7 => 0x00000000,  // COLOR_MENUTEXT
        8 => 0x00000000,  // COLOR_WINDOWTEXT
        9 => 0x00000000,  // COLOR_CAPTIONTEXT
        10 => 0x00b4b4b4, // COLOR_ACTIVEBORDER
        11 => 0x00fcf7f4, // COLOR_INACTIVEBORDER
        12 => 0x00abaaab, // COLOR_APPWORKSPACE
        13 => 0x00d77800, // COLOR_HIGHLIGHT
        14 => 0x00ffffff, // COLOR_HIGHLIGHTTEXT
        15 => 0x00f0f0f0, // COLOR_BTNFACE
        16 => 0x00a0a0a0, // COLOR_BTNSHADOW
        17 => 0x006d6d6d, // COLOR_GRAYTEXT
        18 => 0x00000000, // COLOR_BTNTEXT
        19 => 0x00000000, // COLOR_INACTIVECAPTIONTEXT
        20 => 0x00ffffff, // COLOR_BTNHIGHLIGHT
        21 => 0x00696969, // COLOR_3DDKSHADOW
        22 => 0x00e3e3e3, // COLOR_3DLIGHT
        23 => 0x00000000, // COLOR_INFOTEXT
        24 => 0x00e1ffff, // COLOR_INFOBK
        26 => 0x00cc6600, // COLOR_HOTLIGHT
        27 => 0x00ead1b9, // COLOR_GRADIENTACTIVECAPTION
        28 => 0x00f2e4d7, // COLOR_GRADIENTINACTIVECAPTION
        29 => 0x00d77800, // COLOR_MENUHILIGHT
        30 => 0x00f0f0f0, // COLOR_MENUBAR
        _ => 0,
    }
}
pub(crate) extern "win64" fn get_sys_color_brush(index: i32) -> usize {
    if (0..=30).contains(&index) {
        0x000e_0000 + index as usize
    } else {
        0
    }
}

pub unsafe extern "win64" fn wsprintf_w_impl(
    buffer: *mut u16,
    format: *const u16,
    args: *const u64,
) -> i32 {
    if buffer.is_null() {
        return 0;
    }
    if format.is_null() {
        unsafe { *buffer = 0 };
        return 0;
    }
    unsafe { crate::api::crt::ucrt::common_vswprintf(0, buffer, usize::MAX, format, 0, args) }
}

pub unsafe extern "win64" fn wsprintf_a_impl(
    buffer: *mut std::ffi::c_char,
    format: *const std::ffi::c_char,
    args: *const u64,
) -> i32 {
    if buffer.is_null() {
        return 0;
    }
    if format.is_null() {
        unsafe { *buffer = 0 };
        return 0;
    }
    unsafe { crate::api::crt::ucrt::common_vsprintf(0, buffer, usize::MAX, format, 0, args) }
}

std::arch::global_asm!(
    ".text",
    ".globl peekgame_user32_wsprintf_w",
    "peekgame_user32_wsprintf_w:",
    "mov [rsp+24], r8",
    "mov [rsp+32], r9",
    "lea r8, [rsp+24]",
    "jmp {wide}",
    ".globl peekgame_user32_wsprintf_a",
    "peekgame_user32_wsprintf_a:",
    "mov [rsp+24], r8",
    "mov [rsp+32], r9",
    "lea r8, [rsp+24]",
    "jmp {narrow}",
    wide = sym wsprintf_w_impl,
    narrow = sym wsprintf_a_impl,
);

unsafe extern "win64" {
    #[link_name = "peekgame_user32_wsprintf_w"]
    pub fn wsprintf_w(buffer: *mut u16, format: *const u16) -> i32;
    #[link_name = "peekgame_user32_wsprintf_a"]
    pub fn wsprintf_a(buffer: *mut std::ffi::c_char, format: *const std::ffi::c_char) -> i32;
}

pub unsafe extern "win64" fn wvsprintf_w(
    buffer: *mut u16,
    format: *const u16,
    arglist: *const u64,
) -> i32 {
    unsafe { wsprintf_w_impl(buffer, format, arglist) }
}

pub unsafe extern "win64" fn wvsprintf_a(
    buffer: *mut std::ffi::c_char,
    format: *const std::ffi::c_char,
    arglist: *const u64,
) -> i32 {
    unsafe { wsprintf_a_impl(buffer, format, arglist) }
}

#[repr(C)]
pub struct InitCommonControlsExStruct {
    pub size: u32,
    pub icc: u32,
}

pub unsafe extern "win64" fn init_common_controls_ex(
    init: *const InitCommonControlsExStruct,
) -> i32 {
    if init.is_null() {
        return 0;
    }
    if unsafe { (*init).size } < std::mem::size_of::<InitCommonControlsExStruct>() as u32 {
        return 0;
    }
    1
}

pub extern "win64" fn init_common_controls() {}

