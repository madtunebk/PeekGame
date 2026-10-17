//! NT query information APIs: NtQuerySystemInformation, NtQueryInformationProcess.
use std::ffi::c_void;

const STATUS_SUCCESS: i32 = 0;
const STATUS_INVALID_INFO_CLASS: i32 = 0xC0000003u32 as i32;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC0000004u32 as i32;
const STATUS_INVALID_HANDLE: i32 = 0xC0000008u32 as i32;

pub unsafe extern "win64" fn query_information_process(
    handle: usize,
    class: u32,
    info: *mut c_void,
    length: u32,
    ret_len: *mut u32,
) -> i32 {
    if handle != !0 && handle != 1 && handle != std::process::id() as usize {
        return STATUS_INVALID_HANDLE;
    }
    match class {
        0 => {
            let needed = 48u32;
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            if length < needed {
                return STATUS_INFO_LENGTH_MISMATCH;
            }
            if info.is_null() {
                return 0xC000000Du32 as i32;
            }
            let peb = crate::host::teb::peb_address();
            let pid = std::process::id() as usize;
            unsafe {
                let p = info as *mut u8;
                p.write_bytes(0, needed as usize);
                (p.add(8) as *mut usize).write_unaligned(peb);
                (p.add(16) as *mut usize).write_unaligned(1);
                (p.add(24) as *mut i32).write_unaligned(8);
                (p.add(32) as *mut usize).write_unaligned(pid);
            }
            STATUS_SUCCESS
        }
        7 => {
            let needed = 8u32;
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            if length < needed {
                return STATUS_INFO_LENGTH_MISMATCH;
            }
            if info.is_null() {
                return 0xC000000Du32 as i32;
            }
            unsafe { (info as *mut usize).write_unaligned(0) };
            STATUS_SUCCESS
        }
        26 => {
            let needed = 8u32;
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            if length < needed {
                return STATUS_INFO_LENGTH_MISMATCH;
            }
            if info.is_null() {
                return 0xC000000Du32 as i32;
            }
            unsafe { (info as *mut usize).write_unaligned(0) };
            STATUS_SUCCESS
        }
        _ => STATUS_INVALID_INFO_CLASS,
    }
}

pub unsafe extern "win64" fn query_system_information(
    class: u32,
    info: *mut c_void,
    length: u32,
    ret_len: *mut u32,
) -> i32 {
    match class {
        0 => {
            let needed = 64u32;
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            if length < needed {
                return STATUS_INFO_LENGTH_MISMATCH;
            }
            if info.is_null() {
                return 0xC000000Du32 as i32;
            }
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1) as u32;
            unsafe {
                let p = info as *mut u8;
                p.write_bytes(0, needed as usize);
                (p.add(4) as *mut u32).write_unaligned(4096);
                *p.add(40) = cpus as u8;
            }
            STATUS_SUCCESS
        }
        2 => {
            let needed = length.min(312);
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            if !info.is_null() {
                unsafe { (info as *mut u8).write_bytes(0, needed as usize) };
            }
            STATUS_SUCCESS
        }
        3 => {
            let needed = 48u32;
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            if length < needed {
                return STATUS_INFO_LENGTH_MISMATCH;
            }
            if !info.is_null() {
                const EPOCH: u128 = 11_644_473_600;
                const UNITS: u128 = 10_000_000;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let now_time = ((now.as_secs() as u128 + EPOCH) * UNITS) as u64;
                unsafe {
                    let p = info as *mut u8;
                    p.write_bytes(0, needed as usize);
                    (p as *mut u64).write_unaligned(now_time);
                    (p.add(8) as *mut u64).write_unaligned(now_time);
                }
            }
            STATUS_SUCCESS
        }
        8 => {
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1) as u32;
            let needed = cpus * 48;
            if !ret_len.is_null() {
                unsafe { *ret_len = needed };
            }
            let copy_len = length.min(needed);
            if !info.is_null() {
                unsafe { (info as *mut u8).write_bytes(0, copy_len as usize) };
            }
            STATUS_SUCCESS
        }
        _ => STATUS_INVALID_INFO_CLASS,
    }
}
