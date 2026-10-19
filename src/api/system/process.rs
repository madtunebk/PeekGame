use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};

pub struct ChildProcess {
    pub child: Mutex<std::process::Child>,
    #[allow(dead_code)]
    pub pid: u32,
    pub exit_code: AtomicU32,
}

static CHILD_PROCESSES: OnceLock<Mutex<HashMap<usize, Arc<ChildProcess>>>> = OnceLock::new();
static THREAD_TO_PROCESS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
static NEXT_PROC_HANDLE: AtomicUsize = AtomicUsize::new(0x7100_0000);

fn child_processes() -> &'static Mutex<HashMap<usize, Arc<ChildProcess>>> {
    CHILD_PROCESSES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn thread_handles() -> &'static Mutex<HashMap<usize, usize>> {
    THREAD_TO_PROCESS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn is_child_process(handle: usize) -> bool {
    child_processes().lock().unwrap().contains_key(&handle)
}

pub fn duplicate_child_handle(handle: usize) -> Option<usize> {
    let proc = child_processes().lock().unwrap().get(&handle).cloned();
    if let Some(proc) = proc {
        let new_h = NEXT_PROC_HANDLE.fetch_add(16, Ordering::Relaxed);
        child_processes().lock().unwrap().insert(new_h, proc);
        return Some(new_h);
    }
    let proc_h = thread_handles().lock().unwrap().get(&handle).copied();
    if let Some(proc_h) = proc_h {
        let new_h = NEXT_PROC_HANDLE.fetch_add(16, Ordering::Relaxed);
        thread_handles().lock().unwrap().insert(new_h, proc_h);
        return Some(new_h);
    }
    None
}

pub fn close_child_handle(handle: usize) -> bool {
    if child_processes().lock().unwrap().remove(&handle).is_some() {
        return true;
    }
    if thread_handles().lock().unwrap().remove(&handle).is_some() {
        return true;
    }
    false
}

pub fn is_child_thread(handle: usize) -> bool {
    thread_handles().lock().unwrap().contains_key(&handle)
}

pub fn wait_child_process(handle: usize, milliseconds: u32) -> Option<u32> {
    let proc = child_processes().lock().unwrap().get(&handle).cloned()?;
    let start = std::time::Instant::now();
    loop {
        {
            let mut child = proc.child.lock().unwrap();
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().unwrap_or(0) as u32;
                    proc.exit_code.store(code, Ordering::SeqCst);
                    return Some(0); // WAIT_OBJECT_0
                }
                Ok(None) => {}
                Err(_) => return Some(u32::MAX), // WAIT_FAILED
            }
        }
        if milliseconds == 0
            || (milliseconds != u32::MAX && start.elapsed().as_millis() >= milliseconds as u128)
        {
            return Some(258); // WAIT_TIMEOUT
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

static EXITING: AtomicBool = AtomicBool::new(false);
/// Real loader shutdown state, queried by DXVK during ordinary device cleanup.
pub(crate) extern "win64" fn dll_shutdown_in_progress() -> u8 {
    EXITING.load(Ordering::Acquire) as u8
}
// DLL_PROCESS_DETACH notifications require the future PE loader.
// For now this implements host process termination, including all host threads.
pub(crate) extern "win64" fn exit_process(code: u32) -> ! {
    eprintln!("ExitProcess called with code {code}");
    let rsp_val: usize;
    unsafe { std::arch::asm!("mov {}, rsp", out(reg) rsp_val); }
    let ptr = rsp_val as *const usize;
    for i in 0..48 {
        let val = unsafe { ptr.add(i).read() };
        if let Some(m) = crate::images::try_by_address(val) {
            eprintln!("  stack[{i}] = {val:#x} ({}+{:#x})", m.name, val - m.base);        }
    }
    EXITING.store(true, Ordering::Release);
    crate::api::crt::startup::cexit();
    std::process::exit((code & 0xff) as i32)
}

/// Only the current process is exposed by OpenProcess in this backend.
/// _exit deliberately skips CRT cleanup, DLL detach and Rust destructors.
pub(crate) extern "win64" fn terminate_process(handle: usize, code: u32) -> i32 {
    if let Some(proc) = child_processes().lock().unwrap().get(&handle).cloned() {
        let mut child = proc.child.lock().unwrap();
        let _ = child.kill();
        proc.exit_code.store(code, Ordering::SeqCst);
        return 1;
    }
    eprintln!("TerminateProcess called with code {code}");
    let rsp_val: usize;
    unsafe { std::arch::asm!("mov {}, rsp", out(reg) rsp_val); }
    let ptr = rsp_val as *const usize;
    for i in 0..48 {
        let val = unsafe { ptr.add(i).read() };
        let desc = crate::images::try_by_address(val)
            .map(|m| format!("({}+{:#x})", m.name, val - m.base))
            .unwrap_or_default();
        eprintln!("  stack[{i}] = {val:#x} {desc}");
    }
    if handle != usize::MAX && handle != 1 && handle != std::process::id() as usize {
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
    if let Some(proc) = child_processes().lock().unwrap().get(&handle).cloned() {
        let mut child = proc.child.lock().unwrap();
        if let Ok(Some(status)) = child.try_wait() {
            let exit = status.code().unwrap_or(0) as u32;
            proc.exit_code.store(exit, Ordering::SeqCst);
            unsafe { *code = exit };
        } else {
            unsafe { *code = proc.exit_code.load(Ordering::SeqCst) };
        }
        return 1;
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

/// Wide variant. The job name is ignored (jobs are process-local tokens), so
/// this is identical to the ANSI variant apart from the unused name encoding.
pub unsafe extern "win64" fn create_job_object_w(
    _security: usize,
    _name: *const u16,
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

#[repr(C)]
pub struct ProcessInformation {
    pub h_process: usize,
    pub h_thread: usize,
    pub dw_process_id: u32,
    pub dw_thread_id: u32,
}

fn spawn_child_process(
    app: Option<String>,
    cmd: Option<String>,
    dir: Option<String>,
    information: usize,
) -> i32 {
    let mut target_exe = None;
    let mut args = Vec::new();

    if let Some(app) = app {
        target_exe = Some(app);
        if let Some(cmd) = cmd {
            let u16_cmd: Vec<u16> = cmd.encode_utf16().collect();
            let tokens = crate::host::arguments::split(&u16_cmd);
            for t in tokens.into_iter().skip(1) {
                args.push(String::from_utf16_lossy(&t));
            }
        }
    } else if let Some(cmd) = cmd {
        let u16_cmd: Vec<u16> = cmd.encode_utf16().collect();
        let tokens = crate::host::arguments::split(&u16_cmd);
        if let Some(first) = tokens.first() {
            target_exe = Some(String::from_utf16_lossy(first));
            for t in tokens.into_iter().skip(1) {
                args.push(String::from_utf16_lossy(&t));
            }
        }
    }

    let Some(target) = target_exe else {
        super::error::set(87);
        return 0;
    };

    let host_exe = match crate::host::paths::host(&target) {
        Ok(p) => p,
        Err(e) => {
            super::error::set(e);
            return 0;
        }
    };

    let is_palworld_cef = host_exe
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("EpicWebHelper.exe"))
        && host_exe.to_string_lossy().contains("/Palworld/");
    if is_palworld_cef && std::env::var("PEEKGAME_BLOCK_PALWORLD_CEF").as_deref() == Ok("1") {
        eprintln!("CreateProcess: Palworld CEF helper blocked by PEEKGAME_BLOCK_PALWORLD_CEF=1");
        super::error::set(5); // ERROR_ACCESS_DENIED
        return 0;
    }

    // Steam may replace the compatibility-tool binary while a game is running.
    // current_exe() then ends in " (deleted)" and cannot be spawned by pathname;
    // procfs still refers to the running executable inode.
    #[cfg(target_os = "linux")]
    let self_exe = PathBuf::from("/proc/self/exe");
    #[cfg(not(target_os = "linux"))]
    let self_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("./dist/peekgame"));

    let mut command = std::process::Command::new(self_exe);
    command.arg(&host_exe);
    command.arg("--run");
    for a in &args {
        command.arg(a);
    }

    if let Some(d) = dir {
        if let Ok(dir_path) = crate::host::paths::host(&d) {
            command.current_dir(dir_path);
        }
    } else if let Some(parent) = host_exe.parent() {
        command.current_dir(parent);
    }

    eprintln!(
        "CreateProcess: spawning {:?} with args {:?}",
        host_exe, args
    );

    let mappings_env = crate::api::memory::mapping::export_inherited();
    if !mappings_env.is_empty() {
        command.env("PEEKGAME_INHERITED_MAPPINGS", mappings_env);
    }
    let pipes_env = crate::api::filesystem::pipes::export_inherited();
    if is_palworld_cef {
        let mojo = args.iter().find_map(|arg| {
            arg.strip_prefix("--mojo-platform-channel-handle=")?
                .parse::<usize>().ok()
        });
        let present = mojo.is_some_and(|handle| {
            pipes_env.split(',').any(|entry| entry.starts_with(&format!("{handle:x}:")))
        });
        eprintln!(
            "CEF IPC: mojo={mojo:?}, exported={present}, inherited_pipes={}",
            pipes_env.split(',').filter(|entry| !entry.is_empty()).count()
        );
    }
    if !pipes_env.is_empty() {
        command.env("PEEKGAME_INHERITED_PIPES", pipes_env);
    }

    match command.spawn() {
        Ok(child) => {
            let pid = child.id();
            let h_process = NEXT_PROC_HANDLE.fetch_add(16, Ordering::Relaxed);
            let h_thread = NEXT_PROC_HANDLE.fetch_add(16, Ordering::Relaxed);
            let proc_info = Arc::new(ChildProcess {
                child: Mutex::new(child),
                pid,
                exit_code: AtomicU32::new(259), // STILL_ACTIVE
            });
            child_processes().lock().unwrap().insert(h_process, proc_info);
            thread_handles().lock().unwrap().insert(h_thread, h_process);

            if information != 0 {
                unsafe {
                    let out = &mut *(information as *mut ProcessInformation);
                    out.h_process = h_process;
                    out.h_thread = h_thread;
                    out.dw_process_id = pid;
                    out.dw_thread_id = pid;
                }
            }
            1
        }
        Err(e) => {
            eprintln!("CreateProcess spawn failed: {e}");
            super::error::set(crate::host::paths::error(&e));
            0
        }
    }
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
    directory: *const u16,
    _startup: usize,
    information: usize,
) -> i32 {
    spawn_child_process(
        unsafe { crate::api::text::string(application) },
        unsafe { crate::api::text::string(command) },
        unsafe { crate::api::text::string(directory) },
        information,
    )
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
    directory: *const std::ffi::c_char,
    _startup: usize,
    information: usize,
) -> i32 {
    let narrow = |p: *const std::ffi::c_char| {
        (!p.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
    };
    spawn_child_process(narrow(application), narrow(command), narrow(directory), information)
}

/// GetProcessId: the only process is this one (the current-process pseudo handle
/// -1, or a handle that duplicates it). Other handles are not processes here.
pub extern "win64" fn get_process_id(handle: usize) -> u32 {
    if let Some(proc) = child_processes().lock().unwrap().get(&handle) {
        return proc.pid;
    }
    if handle == usize::MAX
        || handle == 1
        || handle == crate::api::system::compat::STEAM_PROCESS_HANDLE
    {
        crate::api::system::startup::process_id()
    } else {
        crate::api::system::error::set(6);
        0
    }
}

pub extern "win64" fn flush_instruction_cache(_process: usize, _base: usize, _size: usize) -> i32 {
    1
}
