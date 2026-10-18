use std::{
    ffi::{c_char, CStr},
    io::Write,
    sync::atomic::{AtomicU32, Ordering},
};
// The prototype's debug sink is Linux stderr, separate from Windows standard handles.
// Like the Windows void API, a failed debug sink cannot report a write result.
pub(crate) unsafe extern "win64" fn output_debug_string_a(text: *const c_char) {
    if text.is_null() {
        return;
    }
    let bytes = unsafe { CStr::from_ptr(text) }.to_bytes();
    let _ = std::io::stderr().lock().write_all(bytes);
}

static SYM_OPTIONS: AtomicU32 = AtomicU32::new(0x0000_0016); // SYMOPT_CASE_INSENSITIVE | SYMOPT_UNDNAME | SYMOPT_DEFERRED_LOADS

pub(crate) extern "win64" fn sym_get_options() -> u32 {
    SYM_OPTIONS.load(Ordering::Relaxed)
}

pub(crate) extern "win64" fn sym_set_options(options: u32) -> u32 {
    SYM_OPTIONS.store(options, Ordering::Relaxed);
    options
}

pub(crate) extern "win64" fn sym_initialize(
    _process: usize,
    _search_path: *const c_char,
    _invade: i32,
) -> i32 {
    1
}

pub(crate) extern "win64" fn sym_initialize_w(
    _process: usize,
    _search_path: *const u16,
    _invade: i32,
) -> i32 {
    1
}

pub(crate) extern "win64" fn sym_cleanup(_process: usize) -> i32 {
    1
}

pub(crate) extern "win64" fn sym_refresh_module_list(_process: usize) -> i32 {
    1
}

pub(crate) extern "win64" fn stack_walk64(
    _machine_type: u32,
    _process: usize,
    _thread: usize,
    _stack_frame: usize,
    _context_record: usize,
    _read_memory: usize,
    _function_table_access: usize,
    _get_module_base: usize,
    _translate_address: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_function_table_access64(_process: usize, _addr_base: u64) -> usize {
    0
}

pub(crate) extern "win64" fn sym_get_module_base64(_process: usize, _addr: u64) -> u64 {
    0
}

pub(crate) extern "win64" fn sym_from_addr(
    _process: usize,
    _address: u64,
    _displacement: *mut u64,
    _symbol: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_from_addr_w(
    _process: usize,
    _address: u64,
    _displacement: *mut u64,
    _symbol: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_get_line_from_addr64(
    _process: usize,
    _addr: u64,
    _displacement: *mut u32,
    _line: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_get_line_from_addr_w64(
    _process: usize,
    _addr: u64,
    _displacement: *mut u32,
    _line: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_get_module_info64(
    _process: usize,
    _addr: u64,
    _module_info: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_get_module_info_w64(
    _process: usize,
    _addr: u64,
    _module_info: usize,
) -> i32 {
    0
}

pub(crate) extern "win64" fn sym_load_module_ex(
    _process: usize,
    _file: usize,
    _image_name: *const c_char,
    _module_name: *const c_char,
    _base_of_dll: u64,
    _dll_size: u32,
    _data: usize,
    _flags: u32,
) -> u64 {
    0
}

pub(crate) extern "win64" fn sym_load_module_ex_w(
    _process: usize,
    _file: usize,
    _image_name: *const u16,
    _module_name: *const u16,
    _base_of_dll: u64,
    _dll_size: u32,
    _data: usize,
    _flags: u32,
) -> u64 {
    0
}

pub(crate) extern "win64" fn sym_set_home_directory(
    _process: usize,
    dir: *const c_char,
) -> *const c_char {
    dir
}

pub(crate) extern "win64" fn sym_set_home_directory_w(
    _process: usize,
    dir: *const u16,
) -> *const u16 {
    dir
}

pub(crate) extern "win64" fn debug_break() {
    unsafe {
        crate::seh::raise_exception_entry(0x8000_0003, 0, 0, std::ptr::null());
    }
}

pub(crate) extern "win64" fn minidump_write_dump(
    _process: usize,
    _pid: u32,
    _file: usize,
    _dump_type: u32,
    _exception_param: usize,
    _user_stream_param: usize,
    _callback_param: usize,
) -> i32 {
    eprintln!("MiniDumpWriteDump called");
    1
}

pub(crate) unsafe extern "win64" fn image_nt_header(base: *const u8) -> *const u8 {
    if base.is_null() {
        return std::ptr::null();
    }
    let magic = unsafe { (base as *const u16).read_unaligned() };
    if magic != 0x5a4d {
        return std::ptr::null();
    }
    let e_lfanew = unsafe { (base.add(0x3c) as *const i32).read_unaligned() };
    if e_lfanew <= 0 {
        return std::ptr::null();
    }
    let nt = unsafe { base.add(e_lfanew as usize) };
    let nt_magic = unsafe { (nt as *const u32).read_unaligned() };
    if nt_magic != 0x0000_4550 {
        return std::ptr::null();
    }
    nt
}

