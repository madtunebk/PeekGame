//! Process, machine and thread information (ported from peek's kernel32_ext.inc,
//! processor_topology.inc and threads.inc). Values come from the real machine.
use crate::{
    api::{system::error, text},
    host::paths,
    images,
};
use std::{
    collections::HashMap,
    ffi::c_char,
    sync::{atomic::{AtomicU32, Ordering}, Mutex, OnceLock},
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
/// GlobalMemoryStatus: the MEMORYSTATUS (56 bytes on x64: length, load, six
/// SIZE_T counts) subset of GlobalMemoryStatusEx; it sets dwLength itself.
pub unsafe extern "win64" fn global_memory_status(out: *mut u8) {
    if out.is_null() {
        return;
    }
    let mut ex = [0u32; 16];
    ex[0] = 64;
    unsafe { global_memory_status_ex(ex.as_mut_ptr()) };
    unsafe {
        out.cast::<u32>().write_unaligned(56);
        out.add(4).cast::<u32>().write_unaligned(ex[1]);
        std::ptr::copy_nonoverlapping(ex.as_ptr().add(2).cast::<u8>(), out.add(8), 48);
    }
}
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

/// GetLogicalProcessorInformation: returns processor cores and caches.
pub unsafe extern "win64" fn logical_processor_information(
    out: *mut u8,
    length: *mut u32,
) -> i32 {
    if length.is_null() {
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
    let mut data = Vec::new();
    // RelationProcessorCore (0)
    for (_, mask) in &cores {
        let mut r = [0u8; 32];
        r[0..8].copy_from_slice(&mask.to_le_bytes());
        r[8..12].copy_from_slice(&0u32.to_le_bytes()); // RelationProcessorCore
        r[16] = if mask.count_ones() > 1 { 1 } else { 0 }; // Flags: LTP_PC_SMT (1)
        data.extend(r);
    }
    // RelationCache (2): L1 Data cache (32 KB), L2 cache (1 MB) per core
    for (_, mask) in &cores {
        let mut r1 = [0u8; 32];
        r1[0..8].copy_from_slice(&mask.to_le_bytes());
        r1[8..12].copy_from_slice(&2u32.to_le_bytes()); // RelationCache
        r1[16] = 1; // Level 1
        r1[17] = 8; // Associativity
        r1[18..20].copy_from_slice(&64u16.to_le_bytes()); // LineSize
        r1[20..24].copy_from_slice(&(32 * 1024u32).to_le_bytes()); // Size: 32 KB
        r1[24..28].copy_from_slice(&2u32.to_le_bytes()); // CacheData
        data.extend(r1);

        let mut r2 = [0u8; 32];
        r2[0..8].copy_from_slice(&mask.to_le_bytes());
        r2[8..12].copy_from_slice(&2u32.to_le_bytes()); // RelationCache
        r2[16] = 2; // Level 2
        r2[17] = 8; // Associativity
        r2[18..20].copy_from_slice(&64u16.to_le_bytes()); // LineSize
        r2[20..24].copy_from_slice(&(1024 * 1024u32).to_le_bytes()); // Size: 1 MB
        r2[24..28].copy_from_slice(&0u32.to_le_bytes()); // CacheUnified
        data.extend(r2);
    }
    // L3 cache (32 MB) per package
    for (_, mask) in &packages {
        let mut r3 = [0u8; 32];
        r3[0..8].copy_from_slice(&mask.to_le_bytes());
        r3[8..12].copy_from_slice(&2u32.to_le_bytes()); // RelationCache
        r3[16] = 3; // Level 3
        r3[17] = 16; // Associativity
        r3[18..20].copy_from_slice(&64u16.to_le_bytes()); // LineSize
        r3[20..24].copy_from_slice(&(32 * 1024 * 1024u32).to_le_bytes()); // Size: 32 MB
        r3[24..28].copy_from_slice(&0u32.to_le_bytes()); // CacheUnified
        data.extend(r3);
    }
    // RelationProcessorPackage (3)
    for (_, mask) in &packages {
        let mut r = [0u8; 32];
        r[0..8].copy_from_slice(&mask.to_le_bytes());
        r[8..12].copy_from_slice(&3u32.to_le_bytes()); // RelationProcessorPackage
        data.extend(r);
    }
    if out.is_null() || unsafe { *length } < data.len() as u32 {
        unsafe { *length = data.len() as u32 };
        error::set(122); // ERROR_INSUFFICIENT_BUFFER
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
pub extern "win64" fn set_thread_priority_boost(_thread: usize, _disable: i32) -> i32 {
    1
}
pub unsafe extern "win64" fn set_thread_group_affinity(
    _thread: usize,
    group_affinity: *const u8,
    previous: *mut u8,
) -> i32 {
    if group_affinity.is_null() {
        error::set(87);
        return 0;
    }
    let all = affinity();
    let mask = unsafe { group_affinity.cast::<u64>().read_unaligned() };
    let group = unsafe { group_affinity.add(8).cast::<u16>().read_unaligned() };
    if group != 0 || (mask & all) == 0 {
        error::set(87);
        return 0;
    }
    if !previous.is_null() {
        unsafe {
            previous.write_bytes(0, 16);
            previous.cast::<u64>().write_unaligned(all);
        }
    }
    1
}
pub unsafe extern "win64" fn numa_highest_node_number(out: *mut u32) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { out.write_unaligned(0) };
    1
}
pub unsafe extern "win64" fn numa_processor_node(processor: u8, node_number: *mut u8) -> i32 {
    if node_number.is_null() || processor >= 64 {
        if !node_number.is_null() {
            unsafe { *node_number = 0xff };
        }
        error::set(87);
        return 0;
    }
    unsafe { *node_number = 0 };
    1
}
pub unsafe extern "win64" fn numa_processor_node_ex(
    processor: *const u8,
    node_number: *mut u16,
) -> i32 {
    if processor.is_null() || node_number.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { node_number.write_unaligned(0) };
    1
}
pub unsafe extern "win64" fn numa_node_processor_mask(
    node: u8,
    processor_mask: *mut u64,
) -> i32 {
    if processor_mask.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { processor_mask.write_unaligned(if node == 0 { affinity() } else { 0 }) };
    1
}
pub unsafe extern "win64" fn numa_node_processor_mask_ex(
    node: u16,
    processor_mask: *mut u8,
) -> i32 {
    if processor_mask.is_null() {
        error::set(87);
        return 0;
    }
    unsafe {
        processor_mask.write_bytes(0, 16);
        if node == 0 {
            processor_mask.cast::<u64>().write_unaligned(affinity());
        }
    }
    1
}
pub unsafe extern "win64" fn get_thread_ideal_processor_ex(
    _thread: usize,
    ideal: *mut u8,
) -> i32 {
    if ideal.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { ideal.write_bytes(0, 4) };
    1
}
pub unsafe extern "win64" fn current_processor_number_ex(proc_num: *mut u8) {
    if !proc_num.is_null() {
        let cpu = crate::api::threading::threads::current_processor_number() as u8;
        unsafe {
            proc_num.write_bytes(0, 4);
            *proc_num.add(2) = cpu;
        }
    }
}
pub extern "win64" fn get_active_processor_count(group_number: u16) -> u32 {
    if group_number == 0 || group_number == 0xffff {
        affinity().count_ones() as u32
    } else {
        error::set(87);
        0
    }
}
pub extern "win64" fn get_active_processor_group_count() -> u16 {
    1
}
pub extern "win64" fn get_maximum_processor_count(group_number: u16) -> u32 {
    if group_number == 0 || group_number == 0xffff {
        64
    } else {
        error::set(87);
        0
    }
}
pub extern "win64" fn get_maximum_processor_group_count() -> u16 {
    1
}
pub unsafe extern "win64" fn query_thread_cycle_time(_thread: usize, cycles: *mut u64) -> i32 {
    if cycles.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { cycles.write_unaligned(core::arch::x86_64::_rdtsc()) };
    1
}
pub unsafe extern "win64" fn query_process_cycle_time(_process: usize, cycles: *mut u64) -> i32 {
    if cycles.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { cycles.write_unaligned(core::arch::x86_64::_rdtsc()) };
    1
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
static THREAD_DESCRIPTIONS: OnceLock<Mutex<HashMap<u32, String>>> = OnceLock::new();

fn thread_descriptions() -> &'static Mutex<HashMap<u32, String>> {
    THREAD_DESCRIPTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub unsafe extern "win64" fn set_thread_description(thread: usize, description: *const u16) -> i32 {
    let Some(id) = crate::api::threading::threads::id_of(thread) else {
        return 0x8007_0006u32 as i32; // HRESULT_FROM_WIN32(ERROR_INVALID_HANDLE)
    };
    let Some(name) = (unsafe { text::string(description) }) else {
        return 0x8007_0057u32 as i32; // HRESULT_FROM_WIN32(ERROR_INVALID_PARAMETER)
    };
    thread_descriptions().lock().unwrap().insert(id, name.clone());
    if id == crate::api::system::startup::thread_id() {
            let mut name: Vec<u8> = name
                .into_bytes()
                .into_iter()
                .filter(|b| *b != 0)
                .take(15)
                .collect();
            name.push(0);
            let _ = std::fs::write("/proc/thread-self/comm", &name[..name.len() - 1]);
    }
    0
}

/// The caller owns the UTF-16 buffer and releases it with LocalFree.
pub unsafe extern "win64" fn get_thread_description(thread: usize, out: *mut *mut u16) -> i32 {
    if out.is_null() {
        return 0x8000_4003u32 as i32; // E_POINTER
    }
    unsafe { out.write(std::ptr::null_mut()) };
    let Some(id) = crate::api::threading::threads::id_of(thread) else {
        return 0x8007_0006u32 as i32;
    };
    let name = thread_descriptions().lock().unwrap().get(&id).cloned().unwrap_or_default();
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let ptr = crate::api::memory::local::allocate(0, wide.len() * 2) as *mut u16;
    if ptr.is_null() {
        return 0x8007_000eu32 as i32; // HRESULT_FROM_WIN32(ERROR_OUTOFMEMORY)
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        out.write(ptr);
    }
    0 // S_OK
}

/// Windows exposes memory priority and power throttling here. Neither policy
/// is represented by the current Linux thread backend, so fail explicitly.
pub unsafe extern "win64" fn set_thread_information(
    thread: usize,
    class: u32,
    information: *const u8,
    size: u32,
) -> i32 {
    if crate::api::threading::threads::id_of(thread).is_none() {
        super::error::set(6); // ERROR_INVALID_HANDLE
        return 0;
    }
    let expected_size = match class {
        0 => 4,  // ThreadMemoryPriority: MEMORY_PRIORITY_INFORMATION
        3 => 12, // ThreadPowerThrottling: THREAD_POWER_THROTTLING_STATE
        _ => {
            super::error::set(87); // ERROR_INVALID_PARAMETER
            return 0;
        }
    };
    if information.is_null() || size != expected_size {
        super::error::set(87);
        return 0;
    }
    1
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

pub unsafe extern "win64" fn computer_name_a(out: *mut u8, size: *mut u32) -> i32 {
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
    let bytes = name.as_bytes();
    if out.is_null() || unsafe { *size } as usize <= bytes.len() {
        unsafe { *size = bytes.len() as u32 + 1 };
        error::set(111);
        return 0;
    }
    unsafe {
        text::copy_out_narrow(bytes, out, *size);
        *size = bytes.len() as u32;
    }
    1
}

pub unsafe extern "win64" fn computer_name_ex_w(
    format: u32,
    out: *mut u16,
    size: *mut u32,
) -> i32 {
    if size.is_null() {
        error::set(87);
        return 0;
    }
    let mut host = [0u8; 256];
    if unsafe { gethostname(host.as_mut_ptr(), host.len() - 1) } != 0 {
        error::set(1);
        return 0;
    }
    let host_full = String::from_utf8_lossy(&host[..host.iter().position(|b| *b == 0).unwrap_or(0)])
        .into_owned();
    let label = host_full.split('.').next().unwrap_or("").to_string();

    let name = match format {
        0 | 4 => label.to_uppercase().chars().take(15).collect::<String>(),
        1 | 5 => label,
        2 | 6 => {
            if let Some((_, domain)) = host_full.split_once('.') {
                domain.to_string()
            } else {
                String::new()
            }
        }
        3 | 7 => host_full,
        _ => {
            error::set(87);
            return 0;
        }
    };

    let wide = text::wide(&name);
    let needed = wide.len() as u32 + 1;
    if out.is_null() || unsafe { *size } < needed {
        unsafe { *size = needed };
        error::set(234); // ERROR_MORE_DATA
        return 0;
    }

    unsafe {
        text::copy_out(&wide, out, *size);
        *size = wide.len() as u32;
    }
    1
}

pub unsafe extern "win64" fn computer_name_ex_a(
    format: u32,
    out: *mut u8,
    size: *mut u32,
) -> i32 {
    if size.is_null() {
        error::set(87);
        return 0;
    }
    let mut host = [0u8; 256];
    if unsafe { gethostname(host.as_mut_ptr(), host.len() - 1) } != 0 {
        error::set(1);
        return 0;
    }
    let host_full = String::from_utf8_lossy(&host[..host.iter().position(|b| *b == 0).unwrap_or(0)])
        .into_owned();
    let label = host_full.split('.').next().unwrap_or("").to_string();

    let name = match format {
        0 | 4 => label.to_uppercase().chars().take(15).collect::<String>(),
        1 | 5 => label,
        2 | 6 => {
            if let Some((_, domain)) = host_full.split_once('.') {
                domain.to_string()
            } else {
                String::new()
            }
        }
        3 | 7 => host_full,
        _ => {
            error::set(87);
            return 0;
        }
    };

    let bytes = name.as_bytes();
    let needed = bytes.len() as u32 + 1;
    if out.is_null() || unsafe { *size } < needed {
        unsafe { *size = needed };
        error::set(234); // ERROR_MORE_DATA
        return 0;
    }

    unsafe {
        text::copy_out_narrow(bytes, out, *size);
        *size = bytes.len() as u32;
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
pub unsafe extern "win64" fn windows_directory_a(out: *mut u8, capacity: u32) -> u32 {
    unsafe { text::copy_out_narrow(WINDOWS_DIRECTORY.as_bytes(), out, capacity) }
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
    if let Some(m) = images::by_handle(module) {
        return Some(m.path.clone());
    }
    if crate::api::steam::loaded() && module == crate::api::steam::base_address() {
        return Some(std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamclient64.dll"));
    }
    if let Some(name) = crate::api::system::dynamic::builtin_name(module) {
        if crate::api::steam::recognizes(&name) {
            return Some(std::path::PathBuf::from(r"C:\Program Files (x86)\Steam\steamclient64.dll"));
        }
    }
    None
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
    let p = module_path(module);
    static VERBOSE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *VERBOSE.get_or_init(|| std::env::var_os("PEEKGAME_VERBOSE").is_some()) {
        eprintln!("[info] module_file_name_w module={module:#x} path={p:?}");
    }
    match p {
        Some(path) => {
            let win_path = if path.to_string_lossy().starts_with("C:") || path.to_string_lossy().starts_with(r"C:\") {
                path.to_string_lossy().into_owned()
            } else {
                paths::windows(&path)
            };
            unsafe { copy_truncated(&text::wide(&win_path), out, capacity) }
        }
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
        Some(path) => {
            let win_path = if path.to_string_lossy().starts_with("C:") || path.to_string_lossy().starts_with(r"C:\") {
                path.to_string_lossy().into_owned()
            } else {
                paths::windows(&path)
            };
            unsafe {
                copy_truncated(win_path.as_bytes(), out.cast::<u8>(), capacity)
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_and_maximum_processor_count() {
        let active = get_active_processor_count(0xffff);
        assert!(active >= 1, "active processors must be at least 1");
        assert_eq!(get_active_processor_count(0), active);
        assert_eq!(get_active_processor_count(1), 0); // Invalid group
        assert_eq!(crate::api::system::error::get_last_error(), 87);

        assert_eq!(get_active_processor_group_count(), 1);
        assert_eq!(get_maximum_processor_group_count(), 1);
        assert_eq!(get_maximum_processor_count(0xffff), 64);
        assert_eq!(get_maximum_processor_count(0), 64);
        assert_eq!(get_maximum_processor_count(1), 0);
    }
}
