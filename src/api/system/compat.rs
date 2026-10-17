//! Smaller kernel32/advapi32/gdi32/user32 entries Windows libraries import: ANSI
//! forms of existing W functions, process affinity, handle information.
use super::error;
use crate::api::{filesystem::files, system::info, text, threading::objects};
use std::ffi::{c_char, CStr};
fn wide(p: *const c_char) -> Option<Vec<u16>> {
    (!p.is_null()).then(|| {
        unsafe { CStr::from_ptr(p) }
            .to_string_lossy()
            .encode_utf16()
            .chain([0])
            .collect()
    })
}
fn ptr(v: &Option<Vec<u16>>) -> *const u16 {
    v.as_ref().map_or(std::ptr::null(), |v| v.as_ptr())
}
pub unsafe extern "win64" fn create_semaphore_a(
    security: usize,
    initial: i32,
    maximum: i32,
    name: *const c_char,
) -> usize {
    let name = wide(name);
    unsafe { objects::create_semaphore_w(security, initial, maximum, ptr(&name)) }
}
/// GetTempPathA: the length without the NUL, or the size needed.
pub unsafe extern "win64" fn temp_path_a(capacity: u32, out: *mut c_char) -> u32 {
    let mut buffer = [0u16; 1024];
    let n = unsafe { info::temp_path_w(1024, buffer.as_mut_ptr()) } as usize;
    let text = String::from_utf16_lossy(&buffer[..n.min(1024)]);
    unsafe { text::copy_out_narrow(text.as_bytes(), out.cast(), capacity) };
    if (capacity as usize) <= text.len() {
        text.len() as u32 + 1
    } else {
        text.len() as u32
    }
}
pub unsafe extern "win64" fn temp_file_name_a(
    directory: *const c_char,
    prefix: *const c_char,
    unique: u32,
    out: *mut c_char,
) -> u32 {
    let (d, p) = (wide(directory), wide(prefix));
    let mut buffer = [0u16; 260];
    let r = unsafe { files::temp_file_name_w(ptr(&d), ptr(&p), unique, buffer.as_mut_ptr()) };
    if r != 0 && !out.is_null() {
        let n = buffer.iter().position(|&c| c == 0).unwrap_or(259);
        let text = String::from_utf16_lossy(&buffer[..n]);
        unsafe { text::copy_out_narrow(text.as_bytes(), out.cast(), 260) };
    }
    r
}
/// The process may run on every CPU the host gives it.
pub unsafe extern "win64" fn process_affinity_mask(
    _process: usize,
    process_mask: *mut usize,
    system_mask: *mut usize,
) -> i32 {
    let cpus = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(64);
    let mask = if cpus == 64 {
        usize::MAX
    } else {
        (1usize << cpus) - 1
    };
    unsafe {
        if !process_mask.is_null() {
            *process_mask = mask;
        }
        if !system_mask.is_null() {
            *system_mask = mask;
        }
    }
    1
}
pub extern "win64" fn set_process_affinity_mask(_process: usize, mask: usize) -> i32 {
    if mask == 0 {
        error::set(87);
        return 0;
    }
    1
}
pub unsafe extern "win64" fn handle_information(_handle: usize, flags: *mut u32) -> i32 {
    if !flags.is_null() {
        unsafe { *flags = 0 };
    }
    1
}
/// On Windows NT, SetHandleCount does not change the file handle limit.
/// It reports the requested count; file handles remain managed by the OS.
pub extern "win64" fn set_handle_count(requested: u32) -> u32 {
    requested
}
pub extern "win64" fn set_handle_information(_handle: usize, _mask: u32, _flags: u32) -> i32 {
    1
}
pub(crate) const STEAM_PROCESS_HANDLE: usize = 0x57EA_0001;

/// OpenProcess: supports current process and the running Steam client process.
pub extern "win64" fn open_process(_access: u32, _inherit: i32, id: u32) -> usize {
    if id == super::startup::process_id() {
        usize::MAX
    } else if id != 0 && id == crate::api::steam::steam_pid() {
        STEAM_PROCESS_HANDLE
    } else {
        error::set(87);
        0
    }
}
/// DeviceIoControl: no device drivers.
pub extern "win64" fn device_io_control(
    _device: usize,
    _code: u32,
    _in: usize,
    _in_size: u32,
    _out: usize,
    _out_size: u32,
    _returned: usize,
    _overlapped: usize,
) -> i32 {
    error::set(1); // ERROR_INVALID_FUNCTION
    0
}
/// RegNotifyChangeKeyValue: accepted; changes are not signalled (keys change only
/// through this process).
pub extern "win64" fn reg_notify_change_key_value(
    _key: usize,
    _subtree: i32,
    _filter: u32,
    _event: usize,
    _asynchronous: i32,
) -> i32 {
    0
}
/// CreateCompatibleDC/DeleteDC: memory DC tokens (nothing is drawn through GDI).
pub extern "win64" fn create_compatible_dc(_dc: usize) -> usize {
    crate::api::window::misc::get_dc(0)
}
pub extern "win64" fn delete_dc(dc: usize) -> i32 {
    crate::api::window::misc::release_dc(0, dc)
}
/// ChangeDisplaySettingsEx: the current mode stays (DISP_CHANGE_SUCCESSFUL for a
/// reset or the current size, DISP_CHANGE_BADMODE otherwise).
/// ChangeDisplaySettingsEx: no mode switching. NULL (restore) and the device's
/// current size succeed; any other size is DISP_CHANGE_BADMODE (-2). The size is
/// dmPelsWidth/dmPelsHeight: DEVMODEW @172/176, DEVMODEA @108/112 (32-byte names).
fn change_display_settings(device: Option<String>, mode: *const u8, size_offset: usize) -> i32 {
    if mode.is_null() {
        return 0;
    }
    let (w, h) = unsafe {
        (
            mode.add(size_offset).cast::<u32>().read_unaligned(),
            mode.add(size_offset + 4).cast::<u32>().read_unaligned(),
        )
    };
    let Some(current) = crate::api::window::display::device_size(device.as_deref()) else {
        return -2;
    };
    if w == 0 || (w as i32, h as i32) == current {
        0
    } else {
        -2
    }
}
pub unsafe extern "win64" fn change_display_settings_ex_w(
    device: *const u16,
    mode: *const u8,
    _window: usize,
    _flags: u32,
    _parameter: usize,
) -> i32 {
    let device = (!device.is_null()).then(|| String::from_utf16_lossy(unsafe { units(device) }));
    change_display_settings(device, mode, 172)
}
pub unsafe extern "win64" fn change_display_settings_ex_a(
    device: *const c_char,
    mode: *const u8,
    _window: usize,
    _flags: u32,
    _parameter: usize,
) -> i32 {
    let device = (!device.is_null()).then(|| unsafe { CStr::from_ptr(device) }.to_string_lossy().into_owned());
    change_display_settings(device, mode, 108)
}
// lstr*: the old string helpers (NULL counts as empty).
unsafe fn units<'a>(p: *const u16) -> &'a [u16] {
    unsafe { text::units(p) }
}
pub unsafe extern "win64" fn lstrlen_w(s: *const u16) -> i32 {
    unsafe { units(s) }.len() as i32
}
pub unsafe extern "win64" fn lstrlen_a(s: *const c_char) -> i32 {
    if s.is_null() {
        0
    } else {
        unsafe { CStr::from_ptr(s) }.to_bytes().len() as i32
    }
}
fn order(o: std::cmp::Ordering) -> i32 {
    o as i32
}
pub unsafe extern "win64" fn lstrcmp_w(a: *const u16, b: *const u16) -> i32 {
    order(
        String::from_utf16_lossy(unsafe { units(a) })
            .cmp(&String::from_utf16_lossy(unsafe { units(b) })),
    )
}
pub unsafe extern "win64" fn lstrcmpi_w(a: *const u16, b: *const u16) -> i32 {
    order(
        String::from_utf16_lossy(unsafe { units(a) })
            .to_lowercase()
            .cmp(&String::from_utf16_lossy(unsafe { units(b) }).to_lowercase()),
    )
}
unsafe fn bytes<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        &[]
    } else {
        unsafe { CStr::from_ptr(p) }.to_bytes()
    }
}
pub unsafe extern "win64" fn lstrcmp_a(a: *const c_char, b: *const c_char) -> i32 {
    order(unsafe { bytes(a) }.cmp(unsafe { bytes(b) }))
}
pub unsafe extern "win64" fn lstrcmpi_a(a: *const c_char, b: *const c_char) -> i32 {
    order(
        unsafe { bytes(a) }
            .to_ascii_lowercase()
            .cmp(&unsafe { bytes(b) }.to_ascii_lowercase()),
    )
}
pub unsafe extern "win64" fn lstrcpy_w(out: *mut u16, s: *const u16) -> *mut u16 {
    let s = unsafe { units(s) };
    unsafe {
        std::ptr::copy(s.as_ptr(), out, s.len());
        *out.add(s.len()) = 0;
    }
    out
}
pub unsafe extern "win64" fn lstrcpyn_w(out: *mut u16, s: *const u16, max: i32) -> *mut u16 {
    if max <= 0 {
        return out;
    }
    let s = unsafe { units(s) };
    let n = s.len().min(max as usize - 1);
    unsafe {
        std::ptr::copy(s.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    out
}
pub unsafe extern "win64" fn lstrcat_w(out: *mut u16, s: *const u16) -> *mut u16 {
    let at = unsafe { units(out) }.len();
    unsafe { lstrcpy_w(out.add(at), s) };
    out
}
pub unsafe extern "win64" fn lstrcpy_a(out: *mut c_char, s: *const c_char) -> *mut c_char {
    let s = unsafe { bytes(s) };
    unsafe {
        std::ptr::copy(s.as_ptr().cast(), out, s.len());
        *out.add(s.len()) = 0;
    }
    out
}
pub extern "win64" fn get_profile_int_a(
    _app: *const c_char,
    _key: *const c_char,
    default: i32,
) -> u32 {
    default.max(0) as u32
}
pub extern "win64" fn mul_div(number: i32, numerator: i32, denominator: i32) -> i32 {
    let d = denominator as i64;
    if d == 0 {
        return -1;
    }
    let prod = (number as i64) * (numerator as i64);
    let half = d.abs() / 2;
    let rounded = if (prod >= 0) == (d >= 0) {
        prod + half
    } else {
        prod - half
    };
    let q = rounded / d;
    i32::try_from(q).unwrap_or(-1)
}
