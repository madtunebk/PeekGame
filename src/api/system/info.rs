//! Process, machine and thread information (ported from peek's kernel32_ext.inc,
//! processor_topology.inc and threads.inc). Values come from the real machine.
use crate::{
    api::{system::error, text},
    host::paths,
    images,
};
use std::{
    ffi::c_char,
    sync::atomic::{AtomicU32, Ordering},
};
unsafe extern "C" {
    fn sched_getaffinity(pid: i32, size: usize, set: *mut u64) -> i32;
    fn sched_yield() -> i32;
    fn gethostname(name: *mut u8, length: usize) -> i32;
    fn sysconf(name: i32) -> i64;
}
const SC_PAGESIZE: i32 = 30;

static ERROR_MODE: AtomicU32 = AtomicU32::new(0);
pub extern "win64" fn set_error_mode(mode: u32) -> u32 {
    ERROR_MODE.swap(mode & 0x8003, Ordering::Relaxed)
}
pub extern "win64" fn get_error_mode() -> u32 {
    ERROR_MODE.load(Ordering::Relaxed)
}

/// CPUs this process may run on (the first 64), as an affinity mask.
fn affinity() -> u64 {
    let mut set = [0u64; 16];
    if unsafe { sched_getaffinity(0, size_of_val(&set), set.as_mut_ptr()) } == 0 && set[0] != 0 {
        return set[0];
    }
    1
}
/// SYSTEM_INFO (x64, 48 bytes): AMD64, page size, address range, the CPUs this
/// process may use, 64 KiB allocation granularity, family/model/stepping (cpuid).
pub unsafe extern "win64" fn system_info(out: *mut u8) {
    let mask = affinity();
    let leaf = std::arch::x86_64::__cpuid(1).eax;
    let mut family = ((leaf >> 8) & 15) as u16;
    let mut model = ((leaf >> 4) & 15) as u16;
    if family == 15 {
        family += ((leaf >> 20) & 255) as u16;
    }
    if family == 6 || family >= 15 {
        model |= (((leaf >> 16) & 15) << 4) as u16;
    }
    let revision = model << 8 | (leaf & 15) as u16;
    let mut info = [0u8; 48];
    info[0..2].copy_from_slice(&9u16.to_le_bytes());
    info[4..8].copy_from_slice(&(unsafe { sysconf(SC_PAGESIZE) } as u32).to_le_bytes());
    info[8..16].copy_from_slice(&0x10000u64.to_le_bytes());
    info[16..24].copy_from_slice(&0x7fff_fffe_ffffu64.to_le_bytes());
    info[24..32].copy_from_slice(&mask.to_le_bytes());
    info[32..36].copy_from_slice(&mask.count_ones().to_le_bytes());
    info[36..40].copy_from_slice(&8664u32.to_le_bytes());
    info[40..44].copy_from_slice(&0x10000u32.to_le_bytes());
    info[44..46].copy_from_slice(&family.to_le_bytes());
    info[46..48].copy_from_slice(&revision.to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(info.as_ptr(), out, 48) };
}

fn meminfo(file: &str, key: &str) -> u64 {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.strip_prefix(key)?
                    .strip_prefix(':')
                    .map(str::trim)
                    .map(str::to_owned)
            })
        })
        .and_then(|v| v.trim_end_matches(" kB").trim().parse::<u64>().ok())
        .map_or(0, |kb| kb * 1024)
}
/// MEMORYSTATUSEX (64 bytes) from /proc/meminfo and the process size.
pub unsafe extern "win64" fn global_memory_status_ex(out: *mut u32) -> i32 {
    if out.is_null() || unsafe { *out } != 64 {
        error::set(87);
        return 0;
    }
    let total = meminfo("/proc/meminfo", "MemTotal");
    let available = meminfo("/proc/meminfo", "MemAvailable").min(total);
    let commit_limit = meminfo("/proc/meminfo", "CommitLimit").max(total);
    let committed = meminfo("/proc/meminfo", "Committed_AS").min(commit_limit);
    let virtual_total = 1u64 << 47;
    let used_virtual = meminfo("/proc/self/status", "VmSize");
    let load = ((total - available) * 100).checked_div(total).unwrap_or(0) as u32;
    unsafe {
        out.add(1).write(load);
        let q = out.add(2).cast::<u64>();
        for (i, v) in [
            total,
            available,
            commit_limit,
            commit_limit - committed,
            virtual_total,
            virtual_total.saturating_sub(used_virtual),
            0,
        ]
        .into_iter()
        .enumerate()
        {
            q.add(i).write_unaligned(v);
        }
    }
    1
}

fn topology(cpu: u32, field: &str) -> Option<i32> {
    std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{cpu}/topology/{field}"))
        .ok()?
        .trim()
        .parse()
        .ok()
}
fn processor_record(relation: u32, mask: u64) -> [u8; 48] {
    let mut r = [0u8; 48];
    r[0..4].copy_from_slice(&relation.to_le_bytes());
    r[4..8].copy_from_slice(&48u32.to_le_bytes());
    if relation == 0 && mask.count_ones() > 1 {
        r[8] = 1; // LTP_PC_SMT
    }
    r[30..32].copy_from_slice(&1u16.to_le_bytes());
    r[32..40].copy_from_slice(&mask.to_le_bytes());
    r
}
/// GetLogicalProcessorInformationEx for cores (0), NUMA (1, single node),
/// packages (3) and groups (4), from sysfs topology of the CPUs this process may use.
pub unsafe extern "win64" fn logical_processor_information_ex(
    relation: u32,
    out: *mut u8,
    length: *mut u32,
) -> i32 {
    if length.is_null() || ![0, 1, 3, 4, 0xffff].contains(&relation) {
        error::set(87);
        return 0;
    }
    let active = affinity();
    let mut cores: Vec<((i32, i32), u64)> = Vec::new();
    let mut packages: Vec<(i32, u64)> = Vec::new();
    for cpu in (0..64).filter(|c| active & (1 << c) != 0) {
        let (Some(package), Some(core)) = (
            topology(cpu, "physical_package_id"),
            topology(cpu, "core_id"),
        ) else {
            error::set(50);
            return 0;
        };
        match cores.iter_mut().find(|(k, _)| *k == (package, core)) {
            Some((_, m)) => *m |= 1 << cpu,
            None => cores.push(((package, core), 1 << cpu)),
        }
        match packages.iter_mut().find(|(k, _)| *k == package) {
            Some((_, m)) => *m |= 1 << cpu,
            None => packages.push((package, 1 << cpu)),
        }
    }
    let single_node =
        std::fs::read_to_string("/sys/devices/system/node/online").is_ok_and(|v| v.trim() == "0");
    let all = relation == 0xffff;
    let mut data = Vec::new();
    if relation == 0 || all {
        cores
            .iter()
            .for_each(|(_, m)| data.extend(processor_record(0, *m)));
    }
    if relation == 3 || all {
        packages
            .iter()
            .for_each(|(_, m)| data.extend(processor_record(3, *m)));
    }
    if (relation == 1 || all) && single_node {
        let mut r = [0u8; 48];
        r[0..4].copy_from_slice(&1u32.to_le_bytes());
        r[4..8].copy_from_slice(&48u32.to_le_bytes());
        r[32..40].copy_from_slice(&active.to_le_bytes());
        data.extend(r);
    }
    if relation == 4 || all {
        let mut r = [0u8; 80];
        r[0..4].copy_from_slice(&4u32.to_le_bytes());
        r[4..8].copy_from_slice(&80u32.to_le_bytes());
        r[8..10].copy_from_slice(&1u16.to_le_bytes());
        r[10..12].copy_from_slice(&1u16.to_le_bytes());
        r[32] = 64;
        r[33] = active.count_ones() as u8;
        r[72..80].copy_from_slice(&active.to_le_bytes());
        data.extend(r);
    }
    if data.is_empty() {
        error::set(50);
        return 0;
    }
    if out.is_null() || unsafe { *length } < data.len() as u32 {
        unsafe { *length = data.len() as u32 };
        error::set(122);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), out, data.len());
        *length = data.len() as u32;
    }
    1
}

/// SetThreadPriority: accepted without changing the host scheduler, which an
/// unprivileged process cannot raise anyway (peek clamps to the current nice value).
pub extern "win64" fn set_thread_priority(_thread: usize, priority: i32) -> i32 {
    if ![-15, -2, -1, 0, 1, 2, 15].contains(&priority) {
        error::set(87);
        return 0;
    }
    1
}
/// SetThreadAffinityMask: the previous mask (all usable CPUs); the host keeps
/// scheduling freely.
pub extern "win64" fn set_thread_affinity_mask(_thread: usize, mask: u64) -> u64 {
    let all = affinity();
    if mask & all == 0 {
        error::set(87);
        return 0;
    }
    all
}
pub extern "win64" fn switch_to_thread() -> i32 {
    unsafe { sched_yield() };
    1
}
pub extern "win64" fn sleep_ex(ms: u32, alertable: i32) -> u32 {
    crate::api::threading::iocp::release_worker();
    if alertable != 0 {
        let result = crate::api::threading::objects::wait_alertable(ms, || None);
        return if result == crate::api::threading::apc::WAIT_IO_COMPLETION {
            result
        } else {
            0
        };
    }
    if ms == u32::MAX {
        loop {
            std::thread::park();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    0
}
/// SetThreadDescription: the Linux thread name (15 bytes kept) of the calling
/// thread; other threads' names are left unchanged.
pub unsafe extern "win64" fn set_thread_description(thread: usize, description: *const u16) -> i32 {
    if thread == usize::MAX - 1 {
        if let Some(name) = unsafe { text::string(description) } {
            let mut name: Vec<u8> = name
                .into_bytes()
                .into_iter()
                .filter(|b| *b != 0)
                .take(15)
                .collect();
            name.push(0);
            let _ = std::fs::write("/proc/thread-self/comm", &name[..name.len() - 1]);
        }
    }
    0
}

/// GetTempPathW: TMP, TEMP, TMPDIR or /tmp, with the trailing backslash.
pub unsafe extern "win64" fn temp_path_w(capacity: u32, out: *mut u16) -> u32 {
    let dir = ["TMP", "TEMP", "TMPDIR"]
        .iter()
        .filter_map(|n| std::env::var(n).ok())
        .find(|v| !v.is_empty())
        .unwrap_or_else(|| "/tmp".into());
    let mut path = paths::windows(std::path::Path::new(&dir));
    if !path.ends_with('\\') {
        path.push('\\');
    }
    unsafe { text::copy_out(&text::wide(&path), out, capacity) }
}
/// GetComputerNameW: the host name's first label, uppercased, at most 15 characters.
pub unsafe extern "win64" fn computer_name_w(out: *mut u16, size: *mut u32) -> i32 {
    if size.is_null() {
        error::set(87);
        return 0;
    }
    let mut host = [0u8; 256];
    if unsafe { gethostname(host.as_mut_ptr(), host.len() - 1) } != 0 {
        error::set(1);
        return 0;
    }
    let host = String::from_utf8_lossy(&host[..host.iter().position(|b| *b == 0).unwrap_or(0)])
        .into_owned();
    let name: String = host
        .split('.')
        .next()
        .unwrap_or("")
        .to_uppercase()
        .chars()
        .take(15)
        .collect();
    let name = text::wide(&name);
    if out.is_null() || unsafe { *size } as usize <= name.len() {
        unsafe { *size = name.len() as u32 + 1 };
        error::set(111);
        return 0;
    }
    unsafe {
        text::copy_out(&name, out, *size);
        *size = name.len() as u32;
    }
    1
}
/// Windows directories do not exist on the host; programs get the usual paths,
/// which then fail to open like any missing file.
const SYSTEM_DIRECTORY: &str = "C:\\Windows\\system32";
const WINDOWS_DIRECTORY: &str = "C:\\Windows";
pub unsafe extern "win64" fn system_directory_w(out: *mut u16, capacity: u32) -> u32 {
    unsafe { text::copy_out(&text::wide(SYSTEM_DIRECTORY), out, capacity) }
}
pub unsafe extern "win64" fn system_directory_a(out: *mut u8, capacity: u32) -> u32 {
    unsafe { text::copy_out_narrow(SYSTEM_DIRECTORY.as_bytes(), out, capacity) }
}
pub unsafe extern "win64" fn windows_directory_w(out: *mut u16, capacity: u32) -> u32 {
    unsafe { text::copy_out(&text::wide(WINDOWS_DIRECTORY), out, capacity) }
}
pub unsafe extern "win64" fn current_directory_a(capacity: u32, out: *mut u8) -> u32 {
    match std::env::current_dir() {
        Ok(path) => unsafe {
            text::copy_out_narrow(paths::windows(&path).as_bytes(), out, capacity)
        },
        Err(e) => {
            error::set(paths::error(&e));
            0
        }
    }
}
pub unsafe extern "win64" fn output_debug_string_w(message: *const u16) {
    if let Some(message) = unsafe { text::string(message) } {
        eprintln!("OutputDebugStringW: {message}");
    }
}

/// The host path of a module handle: NULL or the EXE, or a loaded PE module.
fn module_path(module: usize) -> Option<std::path::PathBuf> {
    let exe = images::exe()?;
    if module == 0 || module == exe.base {
        return Some(exe.path.clone());
    }
    images::by_handle(module).map(|m| m.path.clone())
}
/// GetModuleFileName contract: truncated copies are NUL-terminated and return the
/// capacity with ERROR_INSUFFICIENT_BUFFER.
unsafe fn copy_truncated<T: Copy + Default>(text: &[T], out: *mut T, capacity: u32) -> u32 {
    if capacity == 0 {
        error::set(122);
        return 0;
    }
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let n = text.len().min(capacity as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, n);
        *out.add(n) = T::default();
    }
    if text.len() >= capacity as usize {
        error::set(122);
        capacity
    } else {
        text.len() as u32
    }
}
pub unsafe extern "win64" fn module_file_name_w(
    module: usize,
    out: *mut u16,
    capacity: u32,
) -> u32 {
    match module_path(module) {
        Some(path) => unsafe { copy_truncated(&text::wide(&paths::windows(&path)), out, capacity) },
        None => {
            error::set(126);
            0
        }
    }
}
pub unsafe extern "win64" fn module_file_name_a(
    module: usize,
    out: *mut c_char,
    capacity: u32,
) -> u32 {
    match module_path(module) {
        Some(path) => unsafe {
            copy_truncated(paths::windows(&path).as_bytes(), out.cast::<u8>(), capacity)
        },
        None => {
            error::set(126);
            0
        }
    }
}
thread_local! {static STACK_GUARANTEE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };}
/// SetThreadStackGuarantee: records the size (stack overflows are dispatched while
/// 16 KiB remain, which covers what programs ask for) and returns the previous one.
pub unsafe extern "win64" fn set_thread_stack_guarantee(size: *mut u32) -> i32 {
    if size.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    let previous = STACK_GUARANTEE.with(|g| g.replace(unsafe { *size }));
    unsafe { *size = previous };
    1
}
thread_local! {static THREAD_ERROR_MODE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };}
/// SetThreadErrorMode: recorded per thread (there are no error dialogs to hide).
pub unsafe extern "win64" fn set_thread_error_mode(mode: u32, old: *mut u32) -> i32 {
    let previous = THREAD_ERROR_MODE.with(|m| m.replace(mode));
    if !old.is_null() {
        unsafe { *old = previous };
    }
    1
}
pub extern "win64" fn get_thread_error_mode() -> u32 {
    THREAD_ERROR_MODE.with(|m| m.get())
}
