use std::sync::atomic::{AtomicBool, Ordering};
static EXITING: AtomicBool = AtomicBool::new(false);
/// Real loader shutdown state, queried by DXVK during ordinary device cleanup.
pub(crate) extern "win64" fn dll_shutdown_in_progress() -> u8 {
    EXITING.load(Ordering::Acquire) as u8
}
// DLL_PROCESS_DETACH notifications require the future PE loader.
// For now this implements host process termination, including all host threads.
pub(crate) extern "win64" fn exit_process(code: u32) -> ! {
    EXITING.store(true, Ordering::Release);
    crate::api::crt::startup::cexit();
    std::process::exit((code & 0xff) as i32)
}

/// Only the current process is exposed by OpenProcess in this backend.
/// _exit deliberately skips CRT cleanup, DLL detach and Rust destructors.
pub(crate) extern "win64" fn terminate_process(handle: usize, code: u32) -> i32 {
    if handle != usize::MAX {
        super::error::set(6);
        return 0;
    }
    unsafe extern "C" {
        fn _exit(status: i32) -> !;
    }
    unsafe { _exit((code & 0xff) as i32) }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct IoCounters {
    pub read_operation_count: u64,
    pub write_operation_count: u64,
    pub other_operation_count: u64,
    pub read_transfer_count: u64,
    pub write_transfer_count: u64,
    pub other_transfer_count: u64,
}

pub unsafe extern "win64" fn get_process_io_counters(
    handle: usize,
    counters: *mut IoCounters,
) -> i32 {
    if handle != !0 && handle != 1 && handle != std::process::id() as usize {
        super::error::set(6);
        return 0;
    }
    if counters.is_null() {
        super::error::set(87);
        return 0;
    }
    let mut io = IoCounters::default();
    if let Ok(text) = std::fs::read_to_string("/proc/self/io") {
        for line in text.lines() {
            if let Some((k, v)) = line.split_once(':') {
                let v = v.trim().parse::<u64>().unwrap_or(0);
                match k.trim() {
                    "syscr" => io.read_operation_count = v,
                    "syscw" => io.write_operation_count = v,
                    "read_bytes" => io.read_transfer_count = v,
                    "write_bytes" => io.write_transfer_count = v,
                    _ => {}
                }
            }
        }
    }
    unsafe { *counters = io };
    1
}

pub unsafe extern "win64" fn get_process_times(
    handle: usize,
    creation: *mut u64,
    exit: *mut u64,
    kernel: *mut u64,
    user: *mut u64,
) -> i32 {
    if handle != !0 && handle != 1 && handle != std::process::id() as usize {
        super::error::set(6);
        return 0;
    }
    if creation.is_null() || exit.is_null() || kernel.is_null() || user.is_null() {
        super::error::set(87);
        return 0;
    }
    #[repr(C)]
    struct Usage {
        user: [i64; 2],
        kernel: [i64; 2],
        counters: [i64; 14],
    }
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut Usage) -> i32;
    }
    let mut usage = Usage {
        user: [0; 2],
        kernel: [0; 2],
        counters: [0; 14],
    };
    let (mut k_time, mut u_time) = (0u64, 0u64);
    if unsafe { getrusage(0, &mut usage) } == 0 {
        k_time = (usage.kernel[0] as u64) * 10_000_000 + (usage.kernel[1] as u64) * 10;
        u_time = (usage.user[0] as u64) * 10_000_000 + (usage.user[1] as u64) * 10;
    }
    const EPOCH: u128 = 11_644_473_600;
    const UNITS: u128 = 10_000_000;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let create_time = ((now.as_secs() as u128 + EPOCH) * UNITS) as u64;
    unsafe {
        creation.write_unaligned(create_time);
        exit.write_unaligned(0);
        kernel.write_unaligned(k_time);
        user.write_unaligned(u_time);
    }
    1
}

pub unsafe extern "win64" fn get_exit_code_process(handle: usize, code: *mut u32) -> i32 {
    if code.is_null() {
        super::error::set(87);
        return 0;
    }
    if handle != !0
        && handle != 1
        && handle != std::process::id() as usize
        && handle != crate::api::system::compat::STEAM_PROCESS_HANDLE
    {
        super::error::set(6);
        return 0;
    }
    unsafe { code.write_unaligned(259) }; // STILL_ACTIVE
    1
}

/// The loader runs x64 PE code in an x64 process, so its exposed processes
/// cannot be WOW64 processes.
pub unsafe extern "win64" fn is_wow64_process(handle: usize, out: *mut i32) -> i32 {
    if out.is_null() {
        super::error::set(87); // ERROR_INVALID_PARAMETER
        return 0;
    }
    if handle != !0
        && handle != 1
        && handle != std::process::id() as usize
        && handle != crate::api::system::compat::STEAM_PROCESS_HANDLE
    {
        super::error::set(6); // ERROR_INVALID_HANDLE
        return 0;
    }
    unsafe { out.write_unaligned(0) };
    1
}

/// IsWow64Process2: an x64 process on an x64 host (process machine UNKNOWN = not WOW64).
pub unsafe extern "win64" fn is_wow64_process2(
    handle: usize,
    process_machine: *mut u16,
    native_machine: *mut u16,
) -> i32 {
    let mut wow = 0i32;
    if unsafe { is_wow64_process(handle, &mut wow) } == 0 {
        return 0;
    }
    if process_machine.is_null() {
        super::error::set(87);
        return 0;
    }
    unsafe {
        process_machine.write_unaligned(0); // IMAGE_FILE_MACHINE_UNKNOWN
        if !native_machine.is_null() {
            native_machine.write_unaligned(0x8664); // IMAGE_FILE_MACHINE_AMD64
        }
    }
    1
}

/// GetProcessMitigationPolicy: no mitigation is enforced on the host, so every
/// policy structure reads back as all flags clear.
pub unsafe extern "win64" fn get_process_mitigation_policy(
    _process: usize,
    policy: i32,
    buffer: *mut u8,
    length: usize,
) -> i32 {
    // ProcessDEPPolicy (0) is DWORD flags + BOOLEAN Permanent; the others are DWORD
    // flag sets (ProcessMitigationOptionsMask (5) is 8 or 16 bytes).
    let needed = match policy {
        0 => 8,
        5 => 8,
        1..=4 | 6..=16 => 4,
        _ => {
            super::error::set(87);
            return 0;
        }
    };
    if buffer.is_null() || length < needed {
        super::error::set(87);
        return 0;
    }
    unsafe { std::ptr::write_bytes(buffer, 0, length) };
    1
}

/// SetProcessMitigationPolicy: accepted; the host does not enforce Windows mitigations.
pub unsafe extern "win64" fn set_process_mitigation_policy(
    _policy: i32,
    buffer: *const u8,
    length: usize,
) -> i32 {
    if buffer.is_null() || length == 0 {
        super::error::set(87);
        return 0;
    }
    1
}

pub unsafe extern "win64" fn create_job_object_a(
    _security: usize,
    _name: *const std::ffi::c_char,
) -> usize {
    unsafe { crate::api::threading::events::create_a(std::ptr::null(), 1, 0, std::ptr::null()) }
}

pub extern "win64" fn set_information_job_object(
    job: usize,
    _class: i32,
    _info: *const u8,
    _length: u32,
) -> i32 {
    if job == 0 {
        super::error::set(6);
        return 0;
    }
    1
}

pub extern "win64" fn assign_process_to_job_object(job: usize, process: usize) -> i32 {
    if job == 0 || process == 0 {
        super::error::set(6);
        return 0;
    }
    1
}

// ---- process creation ---------------------------------------------------------

/// PROC_THREAD_ATTRIBUTE_LIST, laid out by us (the type is opaque): capacity and
/// count (u32 each), then (attribute, value pointer, size) entries.
const ATTRIBUTE_HEADER: usize = 8;
const ATTRIBUTE_ENTRY: usize = 24;
/// InitializeProcThreadAttributeList: with a NULL list it reports the size needed
/// (FALSE, ERROR_INSUFFICIENT_BUFFER), as callers expect before allocating.
pub unsafe extern "win64" fn initialize_proc_thread_attribute_list(
    list: *mut u8,
    count: u32,
    flags: u32,
    size: *mut usize,
) -> i32 {
    if size.is_null() || flags != 0 {
        crate::api::system::error::set(87);
        return 0;
    }
    let needed = ATTRIBUTE_HEADER + count as usize * ATTRIBUTE_ENTRY;
    let given = unsafe { *size };
    unsafe { *size = needed };
    if list.is_null() || given < needed {
        crate::api::system::error::set(122); // ERROR_INSUFFICIENT_BUFFER
        return 0;
    }
    unsafe {
        list.cast::<u32>().write_unaligned(count);
        list.add(4).cast::<u32>().write_unaligned(0);
    }
    1
}
/// UpdateProcThreadAttribute: records the attribute (it only matters to the process
/// creation that would consume it).
pub unsafe extern "win64" fn update_proc_thread_attribute(
    list: *mut u8,
    flags: u32,
    attribute: usize,
    value: usize,
    value_size: usize,
    previous: usize,
    return_size: usize,
) -> i32 {
    if list.is_null() || flags != 0 || previous != 0 || return_size != 0 {
        crate::api::system::error::set(87);
        return 0;
    }
    let (capacity, used) = unsafe { (list.cast::<u32>().read_unaligned(), list.add(4).cast::<u32>().read_unaligned()) };
    if used >= capacity {
        crate::api::system::error::set(122);
        return 0;
    }
    unsafe {
        let entry = list.add(ATTRIBUTE_HEADER + used as usize * ATTRIBUTE_ENTRY).cast::<usize>();
        entry.write_unaligned(attribute);
        entry.add(1).write_unaligned(value);
        entry.add(2).write_unaligned(value_size);
        list.add(4).cast::<u32>().write_unaligned(used + 1);
    }
    1
}
pub extern "win64" fn delete_proc_thread_attribute_list(_list: *mut u8) {}

/// CreateProcess: Windows child processes cannot be started yet, so the call fails
/// with ERROR_NOT_SUPPORTED (a crash handler or launcher is simply not started),
/// and the requested command line is logged.
fn create_process(application: Option<String>, command: Option<String>) -> i32 {
    eprintln!(
        "CreateProcess: {} {}: child processes are not supported",
        application.as_deref().unwrap_or("-"),
        command.as_deref().unwrap_or("")
    );
    crate::api::system::error::set(50);
    0
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn create_process_w(
    application: *const u16,
    command: *const u16,
    _process_attributes: usize,
    _thread_attributes: usize,
    _inherit: i32,
    _flags: u32,
    _environment: usize,
    _directory: *const u16,
    _startup: usize,
    _information: usize,
) -> i32 {
    unsafe { create_process(crate::api::text::string(application), crate::api::text::string(command)) }
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn create_process_a(
    application: *const std::ffi::c_char,
    command: *const std::ffi::c_char,
    _process_attributes: usize,
    _thread_attributes: usize,
    _inherit: i32,
    _flags: u32,
    _environment: usize,
    _directory: *const std::ffi::c_char,
    _startup: usize,
    _information: usize,
) -> i32 {
    let narrow = |p: *const std::ffi::c_char| {
        (!p.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
    };
    create_process(narrow(application), narrow(command))
}

/// GetProcessId: the only process is this one (the current-process pseudo handle
/// -1, or a handle that duplicates it). Other handles are not processes here.
pub extern "win64" fn get_process_id(handle: usize) -> u32 {
    if handle == usize::MAX || handle == crate::api::system::compat::STEAM_PROCESS_HANDLE {
        crate::api::system::startup::process_id()
    } else {
        crate::api::system::error::set(6);
        0
    }
}
