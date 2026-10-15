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
pub extern "win64" fn set_handle_information(_handle: usize, _mask: u32, _flags: u32) -> i32 {
    1
}
/// OpenProcess: only this process exists (its pseudo handle).
pub extern "win64" fn open_process(_access: u32, _inherit: i32, id: u32) -> usize {
    if id == super::startup::process_id() {
        usize::MAX
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
pub unsafe extern "win64" fn change_display_settings_ex(
    _device: usize,
    mode: *const u8,
    _window: usize,
    _flags: u32,
    _parameter: usize,
) -> i32 {
    if mode.is_null() {
        return 0;
    }
    // DEVMODEW: dmPelsWidth @172, dmPelsHeight @176 (fields dmFields @72 bit 0x80000/0x100000).
    let (w, h) = unsafe {
        (
            mode.add(172).cast::<u32>().read_unaligned(),
            mode.add(176).cast::<u32>().read_unaligned(),
        )
    };
    let current = crate::host::x11::screen_size().unwrap_or((0, 0));
    if (w as i32, h as i32) == current || w == 0 {
        0
    } else {
        -2
    }
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
/// I/O completion ports are not implemented yet: creation fails with
/// ERROR_NOT_SUPPORTED, so libraries that need them fail their initialization
/// cleanly (see TODO.md).
pub extern "win64" fn create_io_completion_port(_file: usize, _port: usize, _key: usize, _threads: u32) -> usize {
    static WARNED: std::sync::Once = std::sync::Once::new();
    WARNED.call_once(|| eprintln!("CreateIoCompletionPort: I/O completion ports are not supported yet"));
    error::set(50);
    0
}
