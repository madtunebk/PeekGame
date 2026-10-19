//! Toolhelp snapshots (ported from peek's toolhelp.inc): TH32CS_SNAPPROCESS (2) lists
//! every process from /proc, TH32CS_SNAPTHREAD (4) the threads of this process
//! (callers filter by owner process). This process is named after its EXE; others by
//! their /proc comm. Snapshot handles close with CloseHandle.
use crate::{api::system::error, images};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};
struct Process {
    pid: u32,
    parent: u32,
    threads: u32,
    name: String,
}
struct SnapshotModule {
    base: usize,
    size: u32,
    name: String,
    path: String,
}
struct Snapshot {
    processes: Vec<Process>,
    process_next: usize,
    threads: Vec<u32>,
    thread_next: usize,
    modules: Vec<SnapshotModule>,
    module_next: usize,
}
static SNAPSHOTS: Mutex<Option<HashMap<usize, Snapshot>>> = Mutex::new(None);
static NEXT: AtomicUsize = AtomicUsize::new(0x6800_0000);

pub fn close(handle: usize) -> bool {
    SNAPSHOTS
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|s| s.remove(&handle))
        .is_some()
}
fn read_process(pid: u32) -> Option<Process> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (head, rest) = stat.rsplit_once(')')?;
    // After "pid (comm)": state ppid ... num_threads is the 18th field.
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let parent = fields.get(1)?.parse().ok()?;
    let threads = fields.get(17)?.parse().ok()?;
    let name = if pid == std::process::id() {
        images::exe().map(|m| m.name.clone()).unwrap_or_default()
    } else {
        head.split_once('(')
            .map(|(_, n)| n.to_owned())
            .unwrap_or_default()
    };
    Some(Process {
        pid,
        parent,
        threads,
        name,
    })
}
pub extern "win64" fn create_snapshot(flags: u32, pid: u32) -> usize {
    if flags & !(2 | 4 | 8 | 0x10 | 0x8000_0000) != 0 {
        eprintln!(
            "CreateToolhelp32Snapshot({flags:#x}): unsupported snapshot flags"
        );
        error::set(50);
        return usize::MAX;
    }
    let numbers = |dir: &str| -> Vec<u32> {
        std::fs::read_dir(dir)
            .map(|d| {
                d.flatten()
                    .filter_map(|e| e.file_name().to_str()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    let snapshot = Snapshot {
        processes: if flags & 2 != 0 {
            numbers("/proc")
                .into_iter()
                .filter_map(read_process)
                .collect()
        } else {
            Vec::new()
        },
        process_next: 0,
        threads: if flags & 4 != 0 {
            numbers("/proc/self/task")
        } else {
            Vec::new()
        },
        thread_next: 0,
        modules: if flags & (8 | 0x10) != 0 {
            if pid == 0 || pid == std::process::id() {
                images::all()
                    .into_iter()
                    .map(|m| SnapshotModule {
                        base: m.base,
                        size: m.size as u32,
                        name: m.name.clone(),
                        path: crate::host::paths::windows(&m.path),
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        },
        module_next: 0,
    };
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    SNAPSHOTS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(handle, snapshot);
    handle
}
/// PROCESSENTRY32 (304 bytes) / PROCESSENTRY32W (568): th32ProcessID @8,
/// cntThreads @28, th32ParentProcessID @32, pcPriClassBase @36, szExeFile @44.
unsafe fn process(handle: usize, entry: *mut u8, first: bool, wide: bool) -> i32 {
    let mut snapshots = SNAPSHOTS.lock().unwrap();
    let Some(s) = snapshots.as_mut().and_then(|s| s.get_mut(&handle)) else {
        error::set(6);
        return 0;
    };
    let size = if wide { 568 } else { 304 };
    if entry.is_null() || unsafe { (entry as *const u32).read_unaligned() } < size {
        error::set(24);
        return 0;
    }
    if first {
        s.process_next = 0;
    }
    let Some(p) = s.processes.get(s.process_next) else {
        error::set(18);
        return 0;
    };
    s.process_next += 1;
    unsafe {
        entry.add(4).write_bytes(0, size as usize - 4);
        for (at, v) in [(8, p.pid), (28, p.threads), (32, p.parent), (36, 8)] {
            (entry.add(at) as *mut u32).write_unaligned(v);
        }
        if wide {
            for (i, u) in p.name.encode_utf16().take(259).enumerate() {
                (entry.add(44 + i * 2) as *mut u16).write_unaligned(u);
            }
        } else {
            let bytes = &p.name.as_bytes()[..p.name.len().min(259)];
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), entry.add(44), bytes.len());
        }
    }
    1
}
pub unsafe extern "win64" fn process32_first(h: usize, e: *mut u8) -> i32 {
    unsafe { process(h, e, true, false) }
}
pub unsafe extern "win64" fn process32_next(h: usize, e: *mut u8) -> i32 {
    unsafe { process(h, e, false, false) }
}
pub unsafe extern "win64" fn process32_first_w(h: usize, e: *mut u8) -> i32 {
    unsafe { process(h, e, true, true) }
}
pub unsafe extern "win64" fn process32_next_w(h: usize, e: *mut u8) -> i32 {
    unsafe { process(h, e, false, true) }
}
/// THREADENTRY32 (28 bytes): th32ThreadID @8, th32OwnerProcessID @12, tpBasePri @16.
unsafe fn thread(handle: usize, entry: *mut u8, first: bool) -> i32 {
    let mut snapshots = SNAPSHOTS.lock().unwrap();
    let Some(s) = snapshots.as_mut().and_then(|s| s.get_mut(&handle)) else {
        error::set(6);
        return 0;
    };
    if entry.is_null() || unsafe { (entry as *const u32).read_unaligned() } < 28 {
        error::set(24);
        return 0;
    }
    if first {
        s.thread_next = 0;
    }
    let Some(&tid) = s.threads.get(s.thread_next) else {
        error::set(18);
        return 0;
    };
    s.thread_next += 1;
    unsafe {
        entry.add(4).write_bytes(0, 24);
        for (at, v) in [(8, tid), (12, std::process::id()), (16, 8)] {
            (entry.add(at) as *mut u32).write_unaligned(v);
        }
    }
    1
}
pub unsafe extern "win64" fn thread32_first(h: usize, e: *mut u8) -> i32 {
    unsafe { thread(h, e, true) }
}
pub unsafe extern "win64" fn thread32_next(h: usize, e: *mut u8) -> i32 {
    unsafe { thread(h, e, false) }
}

/// MODULEENTRY32 (568 bytes) / MODULEENTRY32W (1080 bytes)
unsafe fn module(handle: usize, entry: *mut u8, first: bool, wide: bool) -> i32 {
    let mut snapshots = SNAPSHOTS.lock().unwrap();
    let Some(s) = snapshots.as_mut().and_then(|s| s.get_mut(&handle)) else {
        error::set(6);
        return 0;
    };
    let size = if wide { 1080 } else { 568 };
    if entry.is_null() || unsafe { (entry as *const u32).read_unaligned() } < size {
        error::set(24);
        return 0;
    }
    if first {
        s.module_next = 0;
    }
    let Some(m) = s.modules.get(s.module_next) else {
        error::set(18);
        return 0;
    };
    s.module_next += 1;
    unsafe {
        entry.add(4).write_bytes(0, size as usize - 4);
        (entry.add(4) as *mut u32).write_unaligned(1);
        (entry.add(8) as *mut u32).write_unaligned(std::process::id());
        (entry.add(12) as *mut u32).write_unaligned(0xffff);
        (entry.add(16) as *mut u32).write_unaligned(0xffff);
        (entry.add(24) as *mut usize).write_unaligned(m.base);
        (entry.add(32) as *mut u32).write_unaligned(m.size);
        (entry.add(40) as *mut usize).write_unaligned(m.base);
        if wide {
            for (i, u) in m.name.encode_utf16().take(255).enumerate() {
                (entry.add(48 + i * 2) as *mut u16).write_unaligned(u);
            }
            for (i, u) in m.path.encode_utf16().take(259).enumerate() {
                (entry.add(560 + i * 2) as *mut u16).write_unaligned(u);
            }
        } else {
            let name_bytes = &m.name.as_bytes()[..m.name.len().min(255)];
            std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), entry.add(48), name_bytes.len());
            let path_bytes = &m.path.as_bytes()[..m.path.len().min(259)];
            std::ptr::copy_nonoverlapping(path_bytes.as_ptr(), entry.add(304), path_bytes.len());
        }
    }
    1
}

pub unsafe extern "win64" fn module32_first(h: usize, e: *mut u8) -> i32 {
    unsafe { module(h, e, true, false) }
}
pub unsafe extern "win64" fn module32_next(h: usize, e: *mut u8) -> i32 {
    unsafe { module(h, e, false, false) }
}
pub unsafe extern "win64" fn module32_first_w(h: usize, e: *mut u8) -> i32 {
    unsafe { module(h, e, true, true) }
}
pub unsafe extern "win64" fn module32_next_w(h: usize, e: *mut u8) -> i32 {
    unsafe { module(h, e, false, true) }
}

