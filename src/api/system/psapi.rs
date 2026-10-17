//! psapi.dll and kernel32's K32* aliases (ported from peek's psapi.inc): modules of
//! this process and memory figures from /proc. Only the current process (-1) can be
//! queried.
use crate::{
    api::system::{error, info},
    images,
};
const CURRENT_PROCESS: usize = usize::MAX;

fn status_kib(key: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.strip_prefix(key)?
                    .strip_prefix(':')?
                    .trim()
                    .split(' ')
                    .next()?
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}
fn meminfo_kib(key: &str) -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|t| {
            t.lines().find_map(|l| {
                l.strip_prefix(key)?
                    .strip_prefix(':')?
                    .trim()
                    .split(' ')
                    .next()?
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}
/// EnumProcessModules(Ex): sizes in bytes; every module is 64-bit here.
pub unsafe extern "win64" fn enum_process_modules(
    process: usize,
    out: *mut usize,
    size: u32,
    needed: *mut u32,
) -> i32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    if needed.is_null() {
        error::set(998);
        return 0;
    }
    let mut modules = images::all()
        .iter()
        .map(|module| module.base)
        .collect::<Vec<_>>();
    modules.extend(super::dynamic::loaded_builtin_handles());
    let room = if out.is_null() { 0 } else { size as usize / 8 };
    for (i, m) in modules.iter().take(room).enumerate() {
        unsafe { *out.add(i) = *m };
    }
    unsafe { *needed = (modules.len() * 8) as u32 };
    1
}
pub unsafe extern "win64" fn enum_process_modules_ex(
    process: usize,
    out: *mut usize,
    size: u32,
    needed: *mut u32,
    filter: u32,
) -> i32 {
    if filter > 3 {
        error::set(87);
        return 0;
    }
    unsafe { enum_process_modules(process, out, size, needed) }
}
/// MODULEINFO: lpBaseOfDll, SizeOfImage, EntryPoint.
pub unsafe extern "win64" fn module_information(
    process: usize,
    module: usize,
    out: *mut u8,
    size: u32,
) -> i32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    if out.is_null() || size < 24 {
        error::set(122);
        return 0;
    }
    let Some(m) = images::by_handle(module) else {
        error::set(6);
        return 0;
    };
    let entry = if m.image.entry_rva == 0 {
        0
    } else {
        m.base + m.image.entry_rva as usize
    };
    unsafe {
        (out as *mut u64).write_unaligned(m.base as u64);
        (out.add(8) as *mut u64).write_unaligned(m.image.image_size as u64);
        (out.add(16) as *mut u64).write_unaligned(entry as u64);
    }
    1
}
pub unsafe extern "win64" fn module_file_name_ex_w(
    process: usize,
    module: usize,
    out: *mut u16,
    size: u32,
) -> u32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    unsafe { info::module_file_name_w(module, out, size) }
}
pub unsafe extern "win64" fn module_file_name_ex_a(
    process: usize,
    module: usize,
    out: *mut std::ffi::c_char,
    size: u32,
) -> u32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    unsafe { info::module_file_name_a(module, out, size) }
}
/// GetModuleBaseName: the file name part of GetModuleFileName.
fn base_name(module: usize) -> Option<String> {
    let exe = images::exe()?;
    let m = if module == 0 {
        exe
    } else {
        images::by_handle(module)?
    };
    Some(m.name.clone())
}
pub unsafe extern "win64" fn module_base_name_w(
    process: usize,
    module: usize,
    out: *mut u16,
    size: u32,
) -> u32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    let Some(name) = base_name(module) else {
        error::set(6);
        return 0;
    };
    if out.is_null() || size == 0 {
        error::set(122);
        return 0;
    }
    let wide: Vec<u16> = name.encode_utf16().take(size as usize - 1).collect();
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), out, wide.len());
        *out.add(wide.len()) = 0;
    }
    wide.len() as u32
}
pub unsafe extern "win64" fn module_base_name_a(
    process: usize,
    module: usize,
    out: *mut u8,
    size: u32,
) -> u32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    let Some(name) = base_name(module) else {
        error::set(6);
        return 0;
    };
    if out.is_null() || size == 0 {
        error::set(122);
        return 0;
    }
    let n = name.len().min(size as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(name.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    n as u32
}
/// PROCESS_MEMORY_COUNTERS (72) or _EX (80): working set VmRSS, peak VmHWM, private
/// usage VmData; Linux has no paged/nonpaged quota pools.
pub unsafe extern "win64" fn process_memory_info(process: usize, out: *mut u8, size: u32) -> i32 {
    if process != CURRENT_PROCESS {
        error::set(6);
        return 0;
    }
    if out.is_null() || (size != 72 && size < 80) {
        error::set(24);
        return 0;
    }
    let cb = if size >= 80 { 80u32 } else { 72 };
    let (peak, rss, data) = (
        status_kib("VmHWM") * 1024,
        status_kib("VmRSS") * 1024,
        status_kib("VmData") * 1024,
    );
    unsafe {
        out.write_bytes(0, cb as usize);
        (out as *mut u32).write_unaligned(cb);
        (out.add(8) as *mut u64).write_unaligned(peak);
        (out.add(16) as *mut u64).write_unaligned(rss);
        (out.add(56) as *mut u64).write_unaligned(data);
        (out.add(64) as *mut u64).write_unaligned(data);
        if cb == 80 {
            (out.add(72) as *mut u64).write_unaligned(data);
        }
    }
    1
}
/// PERFORMANCE_INFORMATION (104 bytes), sizes in pages.
pub unsafe extern "win64" fn performance_info(out: *mut u8, size: u32) -> i32 {
    if out.is_null() || size < 104 {
        error::set(24);
        return 0;
    }
    let page = 4096u64;
    let v = [
        meminfo_kib("Committed_AS"),
        meminfo_kib("CommitLimit"),
        meminfo_kib("Committed_AS"),
        meminfo_kib("MemTotal"),
        meminfo_kib("MemAvailable"),
        meminfo_kib("Cached"),
        meminfo_kib("Slab"),
        meminfo_kib("SReclaimable"),
        meminfo_kib("SUnreclaim"),
    ];
    unsafe {
        out.write_bytes(0, 104);
        (out as *mut u32).write_unaligned(104);
        for (i, kib) in v.iter().enumerate() {
            (out.add(8 + i * 8) as *mut u64).write_unaligned(kib * 1024 / page);
        }
        (out.add(80) as *mut u64).write_unaligned(page);
    }
    1
}
