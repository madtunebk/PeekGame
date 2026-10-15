//! Access tokens, enough for programs that ask whether they run elevated and at
//! which integrity level (ported from peek's security.inc). Root is "elevated" (high
//! integrity); everyone else is a standard user (medium integrity). Token handles
//! close with CloseHandle; other token classes fail with ERROR_NOT_SUPPORTED.
use crate::api::system::error;
use std::{
    collections::HashSet,
    ffi::c_char,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Mutex,
    },
};
unsafe extern "C" {
    fn geteuid() -> u32;
}
static TOKENS: Mutex<Option<HashSet<usize>>> = Mutex::new(None);
static NEXT: AtomicUsize = AtomicUsize::new(0x6900_0000);

pub fn close(handle: usize) -> bool {
    TOKENS
        .lock()
        .unwrap()
        .as_mut()
        .is_some_and(|t| t.remove(&handle))
}
fn is_token(handle: usize) -> bool {
    TOKENS
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|t| t.contains(&handle))
}
/// TOKEN_QUERY (8), TOKEN_QUERY_SOURCE (0x10) and READ_CONTROL only.
pub unsafe extern "win64" fn open_process_token(
    process: usize,
    access: u32,
    out: *mut usize,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    if process != usize::MAX {
        eprintln!("OpenProcessToken: only the current process is supported");
        error::set(50);
        return 0;
    }
    if access & !(0x18 | 0x0002_0000) != 0 {
        eprintln!("OpenProcessToken(access {access:#x}): only query access is supported");
        error::set(5);
        return 0;
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    TOKENS
        .lock()
        .unwrap()
        .get_or_insert_with(HashSet::new)
        .insert(handle);
    unsafe { *out = handle };
    1
}
/// Threads never impersonate here: ERROR_NO_TOKEN.
pub unsafe extern "win64" fn open_thread_token(
    _thread: usize,
    _access: u32,
    _self: i32,
    out: *mut usize,
) -> i32 {
    if !out.is_null() {
        unsafe { *out = 0 };
    }
    error::set(1008);
    0
}
pub unsafe extern "win64" fn token_information(
    token: usize,
    class: u32,
    out: *mut u8,
    size: u32,
    needed: *mut u32,
) -> i32 {
    if !is_token(token) {
        error::set(6);
        return 0;
    }
    if needed.is_null() {
        error::set(998);
        return 0;
    }
    let root = unsafe { geteuid() } == 0;
    match class {
        // TokenElevationType (default) and TokenElevation.
        18 | 20 => {
            unsafe { *needed = 4 };
            if out.is_null() || size < 4 {
                error::set(122);
                return 0;
            }
            unsafe { (out as *mut u32).write_unaligned(if class == 18 { 1 } else { root as u32 }) };
            1
        }
        // TokenIntegrityLevel: TOKEN_MANDATORY_LABEL {PSID, SE_GROUP_INTEGRITY} + S-1-16-RID.
        25 => {
            unsafe { *needed = 28 };
            if out.is_null() || size < 28 {
                error::set(122);
                return 0;
            }
            unsafe {
                out.write_bytes(0, 28);
                let sid = out.add(16);
                (out as *mut u64).write_unaligned(sid as u64);
                (out.add(8) as *mut u32).write_unaligned(0x20);
                *sid = 1;
                *sid.add(1) = 1;
                *sid.add(7) = 16;
                (sid.add(8) as *mut u32).write_unaligned(if root { 0x3000 } else { 0x2000 });
            }
            1
        }
        _ => {
            eprintln!("GetTokenInformation(class {class}) is not supported");
            error::set(50);
            0
        }
    }
}
/// SID: Revision, SubAuthorityCount, IdentifierAuthority[6], SubAuthority[].
pub unsafe extern "win64" fn sid_sub_authority(sid: *mut u8, index: u32) -> *mut u32 {
    if sid.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { sid.add(8 + 4 * index as usize).cast() }
    }
}
pub unsafe extern "win64" fn sid_sub_authority_count(sid: *mut u8) -> *mut u8 {
    if sid.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { sid.add(1) }
    }
}
/// AllocateLocallyUniqueId: unique for the life of the process.
pub unsafe extern "win64" fn allocate_luid(luid: *mut u32) -> i32 {
    static NEXT_LUID: AtomicU64 = AtomicU64::new(0x10000);
    if luid.is_null() {
        error::set(998);
        return 0;
    }
    let value = NEXT_LUID.fetch_add(1, Ordering::Relaxed);
    unsafe {
        *luid = value as u32;
        *luid.add(1) = (value >> 32) as u32;
    }
    1
}
/// GetUserNameA: the effective Linux account; the size counts the NUL.
pub unsafe extern "win64" fn user_name_a(buffer: *mut c_char, size: *mut u32) -> i32 {
    if size.is_null() {
        error::set(87);
        return 0;
    }
    let name = std::env::var("USER")
        .ok()
        .filter(|n| !n.is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/passwd").ok().and_then(|p| {
                let uid = unsafe { geteuid() }.to_string();
                p.lines().find_map(|l| {
                    let f: Vec<&str> = l.split(':').collect();
                    (f.get(2) == Some(&uid.as_str())).then(|| f[0].to_owned())
                })
            })
        });
    let Some(name) = name else {
        error::set(1907);
        return 0;
    };
    let needed = name.len() as u32 + 1;
    if buffer.is_null() || unsafe { *size } < needed {
        unsafe { *size = needed };
        error::set(122);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(name.as_ptr(), buffer.cast(), name.len());
        *buffer.add(name.len()) = 0;
        *size = needed;
    }
    1
}
