//! VirtualAlloc/VirtualFree/VirtualProtect/VirtualQuery (ported from peek's
//! virtual.inc and virtual_query.inc). Reservations and commits are anonymous
//! mappings; reservations start on the 64 KiB allocation granularity. VirtualQuery
//! reads /proc/self/maps.
use crate::{api::system::error, images};
use std::{ffi::c_void, sync::Mutex};
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
    fn mprotect(a: *mut c_void, n: usize, p: i32) -> i32;
    fn madvise(a: *mut c_void, n: usize, advice: i32) -> i32;
}
const MAP_PRIVATE_ANONYMOUS: i32 = 0x22;
const MAP_FIXED_NOREPLACE: i32 = 0x100000;
const MADV_DONTNEED: i32 = 4;
const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const MEM_DECOMMIT: u32 = 0x4000;
const MEM_RELEASE: u32 = 0x8000;
const MEM_FREE: u32 = 0x10000;
const MEM_PRIVATE: u32 = 0x20000;
const MEM_IMAGE: u32 = 0x1000000;
const GRANULARITY: usize = 0x10000;
const PAGE: usize = 4096;
const ERROR_INVALID_ADDRESS: u32 = 487;
/// Reservations: (base, size).
static ALLOCATIONS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

fn linux_protection(p: u32) -> Option<i32> {
    Some(match p {
        0x01 => 0,
        0x02 => 1,
        0x04 => 3,
        0x10 => 4,
        0x20 => 5,
        0x40 => 7,
        _ => return None,
    })
}
fn windows_protection(perms: &[u8]) -> u32 {
    let (r, w, x) = (perms[0] == b'r', perms[1] == b'w', perms[2] == b'x');
    match (x, w, r) {
        (true, true, _) => 0x40,
        (true, false, true) => 0x20,
        (true, false, false) => 0x10,
        (false, true, _) => 0x04,
        (false, false, true) => 0x02,
        _ => 0x01,
    }
}

pub extern "win64" fn alloc(address: usize, n: usize, kind: u32, protection: u32) -> usize {
    let Some(prot) = linux_protection(protection) else {
        error::set(87);
        return 0;
    };
    if n == 0
        || n > usize::MAX - GRANULARITY
        || kind & !(MEM_COMMIT | MEM_RESERVE) != 0
        || kind == 0
        || address.checked_add(n).is_none()
    {
        error::set(87);
        return 0;
    }
    let mut allocations = ALLOCATIONS.lock().unwrap();
    if kind & MEM_RESERVE != 0 || address == 0 {
        let initial = if kind & MEM_COMMIT != 0 { prot } else { 0 };
        let base = if address != 0 {
            let start = address & !(GRANULARITY - 1);
            let size = (address + n - start).next_multiple_of(PAGE);
            let p = unsafe {
                mmap(
                    start as *mut c_void,
                    size,
                    initial,
                    MAP_PRIVATE_ANONYMOUS | MAP_FIXED_NOREPLACE,
                    -1,
                    0,
                )
            };
            if p as isize == -1 || p as usize != start {
                if p as isize != -1 {
                    unsafe { munmap(p, size) };
                }
                error::set(ERROR_INVALID_ADDRESS); // the range is in use
                return 0;
            }
            allocations.push((start, size));
            start
        } else {
            let size = n.next_multiple_of(PAGE);
            let span = size + GRANULARITY;
            let wide = unsafe {
                mmap(
                    std::ptr::null_mut(),
                    span,
                    initial,
                    MAP_PRIVATE_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if wide as isize == -1 {
                error::set(8);
                return 0;
            }
            let start = (wide as usize).next_multiple_of(GRANULARITY);
            unsafe {
                if start > wide as usize {
                    munmap(wide, start - wide as usize);
                }
                let tail = wide as usize + span - (start + size);
                if tail > 0 {
                    munmap((start + size) as *mut c_void, tail);
                }
            }
            allocations.push((start, size));
            start
        };
        return base;
    }
    // Commit inside an existing reservation.
    let start = address & !(PAGE - 1);
    let size = (n + address - start).next_multiple_of(PAGE);
    let inside = allocations.iter().any(|&(base, length)| {
        start >= base && start - base <= length && size <= length - (start - base)
    });
    if inside && unsafe { mprotect(start as *mut c_void, size, prot) } == 0 {
        return start;
    }
    error::set(ERROR_INVALID_ADDRESS);
    0
}
pub extern "win64" fn free(address: usize, n: usize, kind: u32) -> i32 {
    let mut allocations = ALLOCATIONS.lock().unwrap();
    for i in 0..allocations.len() {
        let (base, size) = allocations[i];
        if kind == MEM_RELEASE && address == base && n == 0 {
            if unsafe { munmap(base as *mut c_void, size) } != 0 {
                error::set(ERROR_INVALID_ADDRESS);
                return 0;
            }
            allocations.swap_remove(i);
            return 1;
        }
        if kind == MEM_DECOMMIT
            && address >= base
            && address - base < size
            && n != 0
            && n <= size - (address - base)
        {
            let page = address & !(PAGE - 1);
            let length = (n + address - page).next_multiple_of(PAGE);
            let ok = unsafe {
                mprotect(page as *mut c_void, length, 0) == 0
                    && madvise(page as *mut c_void, length, MADV_DONTNEED) == 0
            };
            if !ok {
                error::set(ERROR_INVALID_ADDRESS);
            }
            return ok as i32;
        }
    }
    error::set(87);
    0
}
/// The /proc/self/maps line containing `address`, or the gap before the next one:
/// (start, end, perms) with perms None for a gap.
fn region(address: usize) -> Option<(usize, usize, Option<[u8; 4]>)> {
    let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        let (range, rest) = line.split_once(' ')?;
        let (lo, hi) = range.split_once('-')?;
        let (lo, hi) = (
            usize::from_str_radix(lo, 16).ok()?,
            usize::from_str_radix(hi, 16).ok()?,
        );
        if address < lo {
            return Some((address & !(PAGE - 1), lo, None));
        }
        if address < hi {
            return Some((lo, hi, Some(rest.as_bytes().get(..4)?.try_into().ok()?)));
        }
    }
    None
}
/// MEMORY_BASIC_INFORMATION (48 bytes).
#[repr(C)]
pub(crate) struct MemoryInfo {
    base: u64,
    allocation_base: u64,
    allocation_protect: u32,
    _partition: u32,
    size: u64,
    state: u32,
    protect: u32,
    kind: u32,
    _padding: u32,
}
pub unsafe extern "win64" fn query(address: usize, out: *mut MemoryInfo, n: usize) -> usize {
    if out.is_null() || n < size_of::<MemoryInfo>() {
        error::set(87);
        return 0;
    }
    let Some((lo, hi, perms)) = region(address) else {
        error::set(87);
        return 0;
    };
    // As on Windows, the region starts at the page containing the address and runs
    // to the end of the pages with the same attributes.
    let page = address & !(PAGE - 1);
    let mut info = MemoryInfo {
        base: page as u64,
        allocation_base: 0,
        allocation_protect: 0,
        _partition: 0,
        size: (hi - page) as u64,
        state: MEM_FREE,
        protect: 0,
        kind: 0,
        _padding: 0,
    };
    if let Some(perms) = perms {
        let protect = windows_protection(&perms);
        info.protect = protect;
        info.allocation_protect = protect;
        info.allocation_base = lo as u64;
        info.state = if protect == 0x01 {
            MEM_RESERVE
        } else {
            MEM_COMMIT
        };
        info.kind = MEM_PRIVATE;
        if let Some(module) = images::by_address(address) {
            info.allocation_base = module.base as u64;
            info.kind = MEM_IMAGE;
        }
        if let Some(&(base, _)) = ALLOCATIONS
            .lock()
            .unwrap()
            .iter()
            .find(|&&(base, size)| address >= base && address - base < size)
        {
            info.allocation_base = base as u64;
        }
    }
    unsafe { out.write(info) };
    size_of::<MemoryInfo>()
}
pub unsafe extern "win64" fn query_ex(
    process: usize,
    address: usize,
    out: *mut MemoryInfo,
    n: usize,
) -> usize {
    if process != !0 && process != 1 && process != std::process::id() as usize {
        error::set(6);
        return 0;
    }
    unsafe { query(address, out, n) }
}
pub unsafe extern "win64" fn read_process_memory(
    process: usize,
    base: *const c_void,
    buffer: *mut c_void,
    size: usize,
    bytes_read: *mut usize,
) -> i32 {
    if process != !0 && process != 1 && process != std::process::id() as usize {
        error::set(6);
        return 0;
    }
    if size == 0 {
        if !bytes_read.is_null() {
            unsafe { *bytes_read = 0 };
        }
        return 1;
    }
    if base.is_null() || buffer.is_null() {
        error::set(87);
        return 0;
    }
    if region(base as usize).is_none() {
        error::set(998);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(base as *const u8, buffer as *mut u8, size);
        if !bytes_read.is_null() {
            *bytes_read = size;
        }
    }
    1
}
pub unsafe extern "win64" fn protect(
    address: usize,
    n: usize,
    protection: u32,
    old: *mut u32,
) -> i32 {
    let Some(prot) = linux_protection(protection) else {
        error::set(87);
        return 0;
    };
    if n == 0 || old.is_null() || address.checked_add(n + PAGE).is_none() {
        error::set(87);
        return 0;
    }
    let start = address & !(PAGE - 1);
    let end = (address + n).next_multiple_of(PAGE);
    let Some((_, _, Some(perms))) = region(address) else {
        error::set(ERROR_INVALID_ADDRESS);
        return 0;
    };
    if unsafe { mprotect(start as *mut c_void, end - start, prot) } != 0 {
        error::set(ERROR_INVALID_ADDRESS);
        return 0;
    }
    unsafe { *old = windows_protection(&perms) };
    1
}
