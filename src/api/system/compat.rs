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
/// CreateCompatibleDC/CreateDC/DeleteDC: memory/device DC tokens (nothing is drawn through GDI).
pub extern "win64" fn create_compatible_dc(_dc: usize) -> usize {
    crate::api::window::misc::get_dc(0)
}
pub extern "win64" fn create_dc(
    _driver: usize,
    _device: usize,
    _port: usize,
    _mode: usize,
) -> usize {
    crate::api::window::misc::get_dc(0)
}
pub extern "win64" fn delete_dc(dc: usize) -> i32 {
    crate::api::window::misc::release_dc(0, dc)
}
pub extern "win64" fn get_icm_profile(_dc: usize, _size: usize, _filename: usize) -> i32 {
    0
}
pub extern "win64" fn wts_register_session_notification(_hwnd: usize, _flags: u32) -> i32 {
    1
}
pub extern "win64" fn wts_register_session_notification_ex(
    _server: usize,
    _hwnd: usize,
    _flags: u32,
) -> i32 {
    1
}
pub extern "win64" fn wts_unregister_session_notification(_hwnd: usize) -> i32 {
    1
}
pub extern "win64" fn wts_unregister_session_notification_ex(
    _server: usize,
    _hwnd: usize,
) -> i32 {
    1
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

static WORKGROUP_NAME_W: [u16; 10] = [
    b'W' as u16,
    b'O' as u16,
    b'R' as u16,
    b'K' as u16,
    b'G' as u16,
    b'R' as u16,
    b'O' as u16,
    b'U' as u16,
    b'P' as u16,
    0,
];

pub unsafe extern "win64" fn net_get_join_information(
    _server: *const u16,
    name_buffer: *mut *const u16,
    buffer_type: *mut u32,
) -> u32 {
    if !name_buffer.is_null() {
        unsafe { *name_buffer = WORKGROUP_NAME_W.as_ptr() };
    }
    if !buffer_type.is_null() {
        // NetSetupWorkgroupName = 2
        unsafe { *buffer_type = 2 };
    }
    0 // NERR_Success
}

pub extern "win64" fn net_api_buffer_free(_buffer: *mut u8) -> u32 {
    0 // NERR_Success
}

pub unsafe extern "win64" fn net_user_get_info(
    _server: *const u16,
    _user: *const u16,
    _level: u32,
    bufptr: *mut usize,
) -> u32 {
    if !bufptr.is_null() {
        unsafe { *bufptr = 0 };
    }
    2221 // NERR_UserNotFound
}

static WTS_EMPTY_U32: u32 = 0;

pub unsafe extern "win64" fn wts_query_session_information_w(
    _server: usize,
    _session_id: u32,
    _info_class: u32,
    buffer: *mut *const u32,
    bytes_returned: *mut u32,
) -> i32 {
    if !buffer.is_null() {
        unsafe { *buffer = &WTS_EMPTY_U32 };
    }
    if !bytes_returned.is_null() {
        unsafe { *bytes_returned = 4 };
    }
    1
}

pub extern "win64" fn wts_free_memory(_memory: *mut u8) {}

static EMPTY_ENV_BLOCK_W: [u16; 2] = [0, 0];

pub unsafe extern "win64" fn create_environment_block(
    env: *mut *const u16,
    _token: usize,
    _inherit: i32,
) -> i32 {
    if !env.is_null() {
        unsafe { *env = EMPTY_ENV_BLOCK_W.as_ptr() };
    }
    1
}

pub extern "win64" fn destroy_environment_block(_env: *mut u8) -> i32 {
    1
}

pub unsafe extern "win64" fn get_user_name_ex_w(
    _format: u32,
    buffer: *mut u16,
    size: *mut u32,
) -> u8 {
    let name: [u16; 7] = [
        b'p' as u16,
        b'l' as u16,
        b'a' as u16,
        b'y' as u16,
        b'e' as u16,
        b'r' as u16,
        0,
    ];
    if size.is_null() {
        return 0;
    }
    let cap = unsafe { *size } as usize;
    if buffer.is_null() || cap < name.len() {
        unsafe { *size = name.len() as u32 };
        error::set(234); // ERROR_MORE_DATA
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(name.as_ptr(), buffer, name.len());
        *size = (name.len() - 1) as u32;
    }
    1
}

pub unsafe extern "win64" fn dsound_create(
    _guid: usize,
    pp_ds: *mut usize,
    _unk: usize,
) -> i32 {
    if !pp_ds.is_null() {
        unsafe { *pp_ds = 0 };
    }
    0x88780078u32 as i32 // DSERR_NODRIVER
}

pub extern "win64" fn dsound_enum(_cb: usize, _ctx: usize) -> i32 {
    0 // DS_OK
}

pub extern "win64" fn sh_change_notify_register(
    _hwnd: usize,
    _sources: i32,
    _events: i32,
    _msg: u32,
    _entries: i32,
    _pshcne: usize,
) -> u32 {
    1
}

pub extern "win64" fn sh_change_notify_deregister(_id: u32) -> i32 {
    1
}

pub extern "win64" fn is_user_an_admin() -> i32 {
    0
}

pub extern "win64" fn sh_alloc_shared(_data: usize, _size: u32, _pid: u32) -> usize {
    0
}

pub unsafe extern "win64" fn qi_search(
    _that: usize,
    _pqit: usize,
    _riid: usize,
    ppv: *mut usize,
) -> i32 {
    if !ppv.is_null() {
        unsafe { *ppv = 0 };
    }
    0x80004002u32 as i32 // E_NOINTERFACE
}

pub unsafe extern "win64" fn path_find_file_name_w(path: *const u16) -> *const u16 {
    if path.is_null() {
        return std::ptr::null();
    }
    let mut cur = path;
    let mut last = path;
    unsafe {
        while *cur != 0 {
            let ch = *cur;
            cur = cur.add(1);
            if (ch == b'\\' as u16 || ch == b'/' as u16 || ch == b':' as u16) && *cur != 0 {
                last = cur;
            }
        }
    }
    last
}

pub unsafe extern "win64" fn path_find_extension_w(path: *const u16) -> *const u16 {
    if path.is_null() {
        return std::ptr::null();
    }
    let mut cur = path;
    let mut dot = std::ptr::null();
    unsafe {
        while *cur != 0 {
            let ch = *cur;
            if ch == b'.' as u16 {
                dot = cur;
            } else if ch == b'\\' as u16 || ch == b'/' as u16 || ch == b' ' as u16 {
                dot = std::ptr::null();
            }
            cur = cur.add(1);
        }
        if dot.is_null() { cur } else { dot }
    }
}

pub unsafe extern "win64" fn path_file_exists_w(path: *const u16) -> i32 {
    if path.is_null() {
        return 0;
    }
    i32::from(unsafe { crate::api::filesystem::paths::attributes_w(path) } != u32::MAX)
}

pub unsafe extern "win64" fn path_append_w(path: *mut u16, more: *const u16) -> i32 {
    if path.is_null() || more.is_null() {
        return 0;
    }
    unsafe {
        let mut p_len = text::units(path).len();
        let m_units = text::units(more);
        let mut m_slice = m_units;
        while let Some(&c) = m_slice.first() {
            if c == b'\\' as u16 || c == b'/' as u16 {
                m_slice = &m_slice[1..];
            } else {
                break;
            }
        }
        if p_len > 0 && *path.add(p_len - 1) != b'\\' as u16 && *path.add(p_len - 1) != b'/' as u16
        {
            if p_len + 1 < 260 {
                *path.add(p_len) = b'\\' as u16;
                p_len += 1;
            }
        }
        let copy_len = m_slice.len().min(259usize.saturating_sub(p_len));
        std::ptr::copy_nonoverlapping(m_slice.as_ptr(), path.add(p_len), copy_len);
        *path.add(p_len + copy_len) = 0;
    }
    1
}

pub unsafe extern "win64" fn path_remove_extension_w(path: *mut u16) {
    if path.is_null() {
        return;
    }
    unsafe {
        let ext = path_find_extension_w(path) as *mut u16;
        if !ext.is_null() && *ext == b'.' as u16 {
            *ext = 0;
        }
    }
}

pub extern "win64" fn path_match_spec_w(_file: *const u16, _spec: *const u16) -> i32 {
    0
}

pub extern "win64" fn assoc_query_string_w(
    _flags: u32,
    _str: u32,
    _assoc: *const u16,
    _extra: *const u16,
    _out: *mut u16,
    _cch: *mut u32,
) -> i32 {
    0x80004005u32 as i32
}

pub extern "win64" fn draw_theme_background(
    _htheme: usize,
    _hdc: usize,
    _part: i32,
    _state: i32,
    _rect: usize,
    _clip: usize,
) -> i32 {
    0
}

pub extern "win64" fn open_theme_data(_hwnd: usize, _class_list: *const u16) -> usize {
    0
}

pub extern "win64" fn close_theme_data(_htheme: usize) -> i32 {
    0
}

pub extern "win64" fn get_theme_part_size(
    _htheme: usize,
    _hdc: usize,
    _part: i32,
    _state: i32,
    _prc: usize,
    _esize: u32,
    _psz: usize,
) -> i32 {
    0x80004005u32 as i32
}

pub extern "win64" fn get_theme_background_content_rect(
    _htheme: usize,
    _hdc: usize,
    _part: i32,
    _state: i32,
    _bound: usize,
    _content: usize,
) -> i32 {
    0
}

pub extern "win64" fn connect_to_printer_dlg(_hwnd: usize, _flags: u32) -> usize {
    0
}

pub unsafe extern "win64" fn queue_user_work_item(
    func: Option<extern "win64" fn(usize) -> u32>,
    context: usize,
    _flags: u32,
) -> i32 {
    if func.is_none() {
        return 0;
    }
    let h = unsafe {
        crate::api::threading::threads::create(
            std::ptr::null(),
            0,
            func,
            context,
            0,
            std::ptr::null_mut(),
        )
    };
    if h != 0 {
        let _ = crate::api::threading::threads::close(h);
        1
    } else {
        0
    }
}

pub extern "win64" fn create_fiber(_stack_size: usize, _start: usize, param: usize) -> usize {
    param.max(1)
}

pub extern "win64" fn delete_fiber(_fiber: usize) {}

pub extern "win64" fn switch_to_fiber(_fiber: usize) {}

pub extern "win64" fn is_thread_a_fiber() -> i32 {
    0
}

pub unsafe extern "win64" fn create_event_ex_a(
    sec: usize,
    name: *const c_char,
    flags: u32,
    access: u32,
) -> usize {
    let w = wide(name);
    unsafe { objects::create_event_ex_w(sec, ptr(&w), flags, access) }
}

pub unsafe extern "win64" fn query_full_process_image_name_w(
    _proc: usize,
    _flags: u32,
    exe_name: *mut u16,
    size: *mut u32,
) -> i32 {
    if exe_name.is_null() || size.is_null() {
        return 0;
    }
    let cap = unsafe { *size };
    let n = unsafe { info::module_file_name_w(0, exe_name, cap) };
    if n == 0 || n >= cap {
        return 0;
    }
    unsafe { *size = n };
    1
}

pub unsafe extern "win64" fn get_process_handle_count(_proc: usize, count: *mut u32) -> i32 {
    if !count.is_null() {
        unsafe { *count = 64 };
    }
    1
}

pub extern "win64" fn reopen_file(
    _orig: usize,
    _access: u32,
    _share: u32,
    _flags: u32,
) -> usize {
    error::set(50); // ERROR_NOT_SUPPORTED
    usize::MAX
}

pub unsafe extern "win64" fn move_file_ex_a(
    existing: *const c_char,
    new: *const c_char,
    flags: u32,
) -> i32 {
    let (e, n) = (wide(existing), wide(new));
    unsafe { files::move_ex_w(ptr(&e), ptr(&n), flags) }
}

pub extern "win64" fn console_noop_true(_a: usize, _b: usize, _c: usize, _d: usize, _e: usize) -> i32 {
    1
}

pub unsafe extern "win64" fn write_console_a(
    _handle: usize,
    _buf: *const u8,
    len: u32,
    written: *mut u32,
    _reserved: usize,
) -> i32 {
    if !written.is_null() {
        unsafe { *written = len };
    }
    1
}

pub unsafe extern "win64" fn read_console_a(
    _handle: usize,
    _buf: *mut u8,
    _len: u32,
    read: *mut u32,
    _ctrl: usize,
) -> i32 {
    if !read.is_null() {
        unsafe { *read = 0 };
    }
    1
}

pub unsafe extern "win64" fn get_number_of_console_input_events(
    _handle: usize,
    events: *mut u32,
) -> i32 {
    if !events.is_null() {
        unsafe { *events = 0 };
    }
    1
}

pub extern "win64" fn get_named_pipe_handle_state_w(
    _pipe: usize,
    _state: usize,
    _cur_inst: usize,
    _max_count: usize,
    _collect_timeout: usize,
    _user_name: usize,
    _max_user_size: u32,
) -> i32 {
    error::set(50);
    0
}

pub extern "win64" fn read_directory_changes_w(
    _dir: usize,
    _buf: usize,
    _len: u32,
    _watch_subtree: i32,
    _filter: u32,
    _bytes_ret: usize,
    _overlapped: usize,
    _completion: usize,
) -> i32 {
    error::set(50);
    0
}

pub extern "win64" fn nt_create_named_pipe_file(
    _file_handle: usize,
    _desired_access: u32,
    _obj_attr: usize,
    _io_status: usize,
    _share_access: u32,
    _create_disp: u32,
    _create_opts: u32,
    _pipe_type: u32,
    _read_mode: u32,
    _completion_mode: u32,
    _max_inst: u32,
    _inbound_quota: u32,
    _outbound_quota: u32,
    _default_timeout: usize,
) -> i32 {
    0xC0000034u32 as i32 // STATUS_OBJECT_NAME_NOT_FOUND
}

pub extern "win64" fn qos_false(_a: usize, _b: usize, _c: usize, _d: usize, _e: usize, _f: usize) -> i32 {
    0
}

pub extern "win64" fn cm_locate_devnode_w(
    pdn_dev_inst: *mut u32,
    _p_device_id: *const u16,
    _ul_flags: u32,
) -> u32 {
    if !pdn_dev_inst.is_null() {
        unsafe { *pdn_dev_inst = 0 };
    }
    0x0000000D // CR_NO_SUCH_DEVNODE
}

pub extern "win64" fn cm_get_devnode_property_w(
    _dn_dev_inst: u32,
    _prop_key: usize,
    _prop_type: usize,
    _prop_buf: usize,
    _prop_buf_size: usize,
    _ul_flags: u32,
) -> u32 {
    0x00000025 // CR_NO_SUCH_VALUE
}

pub extern "win64" fn cm_map_cr_to_win32_err(_cr: u32, default_err: u32) -> u32 {
    default_err
}

pub unsafe extern "win64" fn cm_get_device_id_list_size_w(
    pul_len: *mut u32,
    _psz_filter: *const u16,
    _ul_flags: u32,
) -> u32 {
    if !pul_len.is_null() {
        unsafe { *pul_len = 2 };
    }
    0 // CR_SUCCESS
}

pub unsafe extern "win64" fn cm_get_device_id_list_w(
    _psz_filter: *const u16,
    buffer: *mut u16,
    buffer_len: u32,
    _ul_flags: u32,
) -> u32 {
    if !buffer.is_null() && buffer_len >= 2 {
        unsafe {
            *buffer = 0;
            *buffer.add(1) = 0;
        }
    }
    0 // CR_SUCCESS
}

pub extern "win64" fn setup_di_open_dev_reg_key(
    _dev_info_set: usize,
    _dev_info_data: usize,
    _scope: u32,
    _hw_profile: u32,
    _key_type: u32,
    _sam_desired: u32,
) -> usize {
    error::set(2);
    usize::MAX // INVALID_HANDLE_VALUE
}

pub extern "win64" fn setup_get_inf_driver_store_location_w(
    _file_name: *const u16,
    _alt_platform: usize,
    _locale: *const u16,
    _return_buf: *mut u16,
    _return_buf_size: u32,
    _required_size: *mut u32,
) -> i32 {
    error::set(2);
    0
}

pub extern "win64" fn icmp_create_file() -> usize {
    usize::MAX
}

pub extern "win64" fn icmp_close_handle(_h: usize) -> i32 {
    1
}

pub extern "win64" fn icmp_send_echo(
    _h: usize,
    _dest: u32,
    _req_data: usize,
    _req_size: u16,
    _req_opts: usize,
    _rep_buf: usize,
    _rep_size: u32,
    _timeout: u32,
) -> u32 {
    error::set(11010); // IP_REQ_TIMED_OUT
    0
}

pub extern "win64" fn wsa_duplicate_socket_w(_s: usize, _pid: u32, _info: usize) -> i32 {
    -1
}

pub extern "win64" fn wsa_address_to_string_w(
    _addr: usize,
    _addr_len: u32,
    _proto: usize,
    _str_buf: *mut u16,
    _str_len: *mut u32,
) -> i32 {
    -1
}

pub extern "win64" fn wsa_string_to_address_w(
    _str_buf: *const u16,
    _family: i32,
    _proto: usize,
    _addr: usize,
    _addr_len: *mut i32,
) -> i32 {
    -1
}

pub unsafe extern "win64" fn crypt_acquire_context(
    ph_prov: *mut usize,
    _container: usize,
    _provider: usize,
    _prov_type: u32,
    _flags: u32,
) -> i32 {
    if !ph_prov.is_null() {
        unsafe { *ph_prov = 0xCAFE_0001 };
    }
    1
}

pub extern "win64" fn crypt_release_context(_h_prov: usize, _flags: u32) -> i32 {
    1
}

pub unsafe extern "win64" fn crypt_create_hash(
    _h_prov: usize,
    _algid: u32,
    _h_key: usize,
    _flags: u32,
    ph_hash: *mut usize,
) -> i32 {
    if !ph_hash.is_null() {
        unsafe { *ph_hash = 0xCAFE_0002 };
    }
    1
}

pub extern "win64" fn crypt_hash_data(
    _h_hash: usize,
    _data: *const u8,
    _len: u32,
    _flags: u32,
) -> i32 {
    1
}

pub unsafe extern "win64" fn crypt_get_hash_param(
    _h_hash: usize,
    param: u32,
    data: *mut u8,
    len: *mut u32,
    _flags: u32,
) -> i32 {
    if len.is_null() {
        return 0;
    }
    if param == 4 {
        // HP_HASHSIZE
        if !data.is_null() && unsafe { *len } >= 4 {
            unsafe { data.cast::<u32>().write_unaligned(20) };
        }
        unsafe { *len = 4 };
        return 1;
    }
    let cap = unsafe { *len } as usize;
    if !data.is_null() && cap > 0 {
        unsafe { data.write_bytes(0, cap.min(64)) };
    }
    unsafe { *len = 20 };
    1
}

pub extern "win64" fn crypt_destroy_handle(_h: usize) -> i32 {
    1
}

pub extern "win64" fn crypt_stub_false(_a: usize, _b: usize, _c: usize, _d: usize, _e: usize, _f: usize, _g: usize) -> i32 {
    error::set(1168); // ERROR_NOT_FOUND
    0
}

pub extern "win64" fn win_verify_trust(_hwnd: usize, _action_id: usize, _data: usize) -> i32 {
    0 // ERROR_SUCCESS
}

pub extern "win64" fn complete_auth_token(_context: usize, _token: usize) -> i32 {
    0 // SEC_E_OK
}

pub extern "win64" fn get_current_thread_stack_limits(low: *mut usize, high: *mut usize) {
    crate::host::teb::current_stack_limits(low, high);
}



