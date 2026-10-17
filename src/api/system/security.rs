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
    // TOKEN_QUERY, TOKEN_QUERY_SOURCE, plus READ_CONTROL / WRITE_DAC / WRITE_OWNER.
    if access & !(0x18 | 0x000e_0000) != 0 {
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
fn current_user_name() -> Option<String> {
    std::env::var("USER")
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
        })
}

/// GetUserNameA: the effective Linux account; the size counts the NUL.
pub unsafe extern "win64" fn user_name_a(buffer: *mut c_char, size: *mut u32) -> i32 {
    if size.is_null() {
        error::set(87);
        return 0;
    }
    let Some(name) = current_user_name() else {
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

/// GetUserNameW: the effective Linux account in UTF-16; the size counts the NUL.
pub unsafe extern "win64" fn user_name_w(buffer: *mut u16, size: *mut u32) -> i32 {
    if size.is_null() {
        error::set(87);
        return 0;
    }
    let Some(name) = current_user_name() else {
        error::set(1907);
        return 0;
    };
    let utf16: Vec<u16> = name.encode_utf16().collect();
    let needed = utf16.len() as u32 + 1;
    if buffer.is_null() || unsafe { *size } < needed {
        unsafe { *size = needed };
        error::set(122);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), buffer, utf16.len());
        *buffer.add(utf16.len()) = 0;
        *size = needed;
    }
    1
}
pub unsafe extern "win64" fn is_valid_sid(sid: *const u8) -> i32 {
    if sid.is_null() {
        return 0;
    }
    let (revision, count) = unsafe { (*sid, *sid.add(1)) };
    (revision == 1 && count <= 15) as i32
}
pub unsafe extern "win64" fn get_length_sid(sid: *const u8) -> u32 {
    if sid.is_null() {
        return 0;
    }
    let count = unsafe { *sid.add(1) } as u32;
    8 + 4 * count
}
pub unsafe extern "win64" fn copy_sid(dest_len: u32, dest: *mut u8, src: *const u8) -> i32 {
    if dest.is_null() || src.is_null() || unsafe { is_valid_sid(src) } == 0 {
        error::set(87);
        return 0;
    }
    let len = unsafe { get_length_sid(src) };
    if dest_len < len {
        error::set(122);
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(src, dest, len as usize);
    }
    1
}
pub unsafe extern "win64" fn system_function_036(buffer: *mut u8, length: u32) -> u8 {
    (unsafe { super::random::bcrypt_gen_random(0, buffer, length, 2) } == 0) as u8
}
pub unsafe extern "win64" fn lookup_account_sid_w(
    _system_name: *const u16,
    sid: *const u8,
    name: *mut u16,
    cch_name: *mut u32,
    domain: *mut u16,
    cch_domain: *mut u32,
    use_kind: *mut u32,
) -> i32 {
    if sid.is_null() || cch_name.is_null() || cch_domain.is_null() || unsafe { is_valid_sid(sid) } == 0 {
        error::set(87);
        return 0;
    }
    let user_name = std::env::var("USER").unwrap_or_else(|_| "User".to_string());
    let domain_name = "WORKGROUP";
    let user_utf16: Vec<u16> = user_name.encode_utf16().chain(std::iter::once(0)).collect();
    let domain_utf16: Vec<u16> = domain_name.encode_utf16().chain(std::iter::once(0)).collect();
    let req_user = user_utf16.len() as u32;
    let req_domain = domain_utf16.len() as u32;
    let avail_user = unsafe { *cch_name };
    let avail_domain = unsafe { *cch_domain };
    unsafe {
        *cch_name = req_user;
        *cch_domain = req_domain;
    }
    if avail_user < req_user || avail_domain < req_domain {
        error::set(122);
        return 0;
    }
    if !name.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(user_utf16.as_ptr(), name, user_utf16.len()) };
    }
    if !domain.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(domain_utf16.as_ptr(), domain, domain_utf16.len()) };
    }
    if !use_kind.is_null() {
        unsafe { *use_kind = 1 };
    }
    1
}

/// GetSecurityInfo: retrieves a copy of the security descriptor for an object.
/// Returns ERROR_SUCCESS (0) on success or a Win32 error code.
pub unsafe extern "win64" fn get_security_info(
    _handle: usize,
    _object_type: u32,
    _security_info: u32,
    ppsid_owner: *mut *mut u8,
    ppsid_group: *mut *mut u8,
    pp_dacl: *mut *mut u8,
    pp_sacl: *mut *mut u8,
    pp_security_descriptor: *mut *mut u8,
) -> u32 {
    let mem = crate::api::memory::local::allocate(0x40, 64) as *mut u8;
    if mem.is_null() {
        return 8; // ERROR_NOT_ENOUGH_MEMORY
    }
    unsafe {
        // SECURITY_DESCRIPTOR_RELATIVE
        // Revision = 1, Sbz1 = 0
        *mem = 1;
        *mem.add(1) = 0;
        // Control = SE_SELF_RELATIVE (0x8000) | SE_DACL_PRESENT (0x0004)
        (mem.add(2) as *mut u16).write_unaligned(0x8004);
        // Owner offset = 20
        (mem.add(4) as *mut u32).write_unaligned(20);
        // Group offset = 36
        (mem.add(8) as *mut u32).write_unaligned(36);
        // Sacl offset = 0
        (mem.add(12) as *mut u32).write_unaligned(0);
        // Dacl offset = 52
        (mem.add(16) as *mut u32).write_unaligned(52);

        // Owner SID at 20: S-1-5-32-544 (16 bytes)
        let owner = mem.add(20);
        *owner = 1; // Revision
        *owner.add(1) = 2; // SubAuthorityCount = 2
        *owner.add(7) = 5; // IdentifierAuthority = NT Authority
        (owner.add(8) as *mut u32).write_unaligned(32);
        (owner.add(12) as *mut u32).write_unaligned(544);

        // Group SID at 36: S-1-5-32-544 (16 bytes)
        let group = mem.add(36);
        *group = 1;
        *group.add(1) = 2;
        *group.add(7) = 5;
        (group.add(8) as *mut u32).write_unaligned(32);
        (group.add(12) as *mut u32).write_unaligned(544);

        // Empty DACL at 52 (8 bytes)
        let dacl = mem.add(52);
        *dacl = 2; // AclRevision = ACL_REVISION (2)
        *dacl.add(1) = 0; // Sbz1 = 0
        (dacl.add(2) as *mut u16).write_unaligned(8); // AclSize = 8
        (dacl.add(4) as *mut u16).write_unaligned(0); // AceCount = 0
        (dacl.add(6) as *mut u16).write_unaligned(0); // Sbz2 = 0

        if !ppsid_owner.is_null() {
            *ppsid_owner = owner;
        }
        if !ppsid_group.is_null() {
            *ppsid_group = group;
        }
        if !pp_dacl.is_null() {
            *pp_dacl = dacl;
        }
        if !pp_sacl.is_null() {
            *pp_sacl = std::ptr::null_mut();
        }
        if !pp_security_descriptor.is_null() {
            *pp_security_descriptor = mem;
        }
    }
    0 // ERROR_SUCCESS
}

pub unsafe extern "win64" fn is_valid_security_descriptor(sd: *const u8) -> i32 {
    if sd.is_null() {
        return 0;
    }
    unsafe { (*sd == 1) as i32 }
}

pub unsafe extern "win64" fn get_security_descriptor_dacl(
    sd: *const u8,
    present: *mut i32,
    dacl: *mut *mut u8,
    defaulted: *mut i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control = (sd.add(2) as *const u16).read_unaligned();
        let is_present = (control & 0x0004) != 0;
        if !present.is_null() {
            *present = is_present as i32;
        }
        if !defaulted.is_null() {
            *defaulted = 0;
        }
        if !dacl.is_null() {
            if is_present {
                if control & 0x8000 != 0 {
                    let off = (sd.add(16) as *const u32).read_unaligned() as usize;
                    *dacl = if off != 0 { sd.add(off) as *mut u8 } else { std::ptr::null_mut() };
                } else {
                    *dacl = (sd.add(16) as *const *mut u8).read_unaligned();
                }
            } else {
                *dacl = std::ptr::null_mut();
            }
        }
    }
    1
}

pub unsafe extern "win64" fn get_security_descriptor_owner(
    sd: *const u8,
    owner: *mut *mut u8,
    defaulted: *mut i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control = (sd.add(2) as *const u16).read_unaligned();
        if !defaulted.is_null() {
            *defaulted = 0;
        }
        if !owner.is_null() {
            if control & 0x8000 != 0 {
                let off = (sd.add(4) as *const u32).read_unaligned() as usize;
                *owner = if off != 0 { sd.add(off) as *mut u8 } else { std::ptr::null_mut() };
            } else {
                *owner = (sd.add(4) as *const *mut u8).read_unaligned();
            }
        }
    }
    1
}

pub unsafe extern "win64" fn get_security_descriptor_group(
    sd: *const u8,
    group: *mut *mut u8,
    defaulted: *mut i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control = (sd.add(2) as *const u16).read_unaligned();
        if !defaulted.is_null() {
            *defaulted = 0;
        }
        if !group.is_null() {
            if control & 0x8000 != 0 {
                let off = (sd.add(8) as *const u32).read_unaligned() as usize;
                *group = if off != 0 { sd.add(off) as *mut u8 } else { std::ptr::null_mut() };
            } else {
                *group = (sd.add(8) as *const *mut u8).read_unaligned();
            }
        }
    }
    1
}

pub unsafe extern "win64" fn get_security_descriptor_length(sd: *const u8) -> u32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        return 0;
    }
    64
}

pub unsafe extern "win64" fn get_security_descriptor_control(
    sd: *const u8,
    control: *mut u16,
    revision: *mut u32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        if !control.is_null() {
            *control = (sd.add(2) as *const u16).read_unaligned();
        }
        if !revision.is_null() {
            *revision = 1;
        }
    }
    1
}

pub unsafe extern "win64" fn initialize_security_descriptor(
    sd: *mut u8,
    revision: u32,
) -> i32 {
    if sd.is_null() || revision != 1 {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        std::ptr::write_bytes(sd, 0, 20);
        *sd = 1; // Revision
    }
    1
}

pub unsafe extern "win64" fn set_security_descriptor_dacl(
    sd: *mut u8,
    present: i32,
    dacl: *const u8,
    defaulted: i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control_ptr = sd.add(2) as *mut u16;
        let mut control = control_ptr.read_unaligned();
        if present != 0 {
            control |= 0x0004; // SE_DACL_PRESENT
            if defaulted != 0 {
                control |= 0x0008; // SE_DACL_DEFAULTED
            } else {
                control &= !0x0008;
            }
            (sd.add(16) as *mut usize).write_unaligned(dacl as usize);
        } else {
            control &= !0x0004;
            (sd.add(16) as *mut usize).write_unaligned(0);
        }
        control_ptr.write_unaligned(control);
    }
    1
}

pub unsafe extern "win64" fn get_acl_information(
    acl: *const u8,
    info: *mut u8,
    len: u32,
    class: u32,
) -> i32 {
    if acl.is_null() || info.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    if class == 2 && len >= 12 {
        let count = unsafe { (acl.add(4) as *const u16).read_unaligned() as u32 };
        let size = unsafe { (acl.add(2) as *const u16).read_unaligned() as u32 };
        unsafe {
            (info as *mut u32).write_unaligned(count);
            (info.add(4) as *mut u32).write_unaligned(size);
            (info.add(8) as *mut u32).write_unaligned(0);
        }
        return 1;
    }
    1
}

pub unsafe extern "win64" fn get_ace(
    acl: *const u8,
    index: u32,
    ace: *mut *mut u8,
) -> i32 {
    if acl.is_null() || ace.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    let count = unsafe { (acl.add(4) as *const u16).read_unaligned() as u32 };
    if index >= count {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe { *ace = std::ptr::null_mut() };
    1
}

pub unsafe extern "win64" fn is_valid_acl(acl: *const u8) -> i32 {
    if acl.is_null() {
        return 0;
    }
    1
}

pub unsafe extern "win64" fn initialize_acl(
    acl: *mut u8,
    len: u32,
    rev: u32,
) -> i32 {
    if acl.is_null() || len < 8 || rev != 2 {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        *acl = rev as u8;
        *acl.add(1) = 0;
        (acl.add(2) as *mut u16).write_unaligned(len as u16);
        (acl.add(4) as *mut u16).write_unaligned(0);
        (acl.add(6) as *mut u16).write_unaligned(0);
    }
    1
}

pub unsafe extern "win64" fn add_access_allowed_ace(
    acl: *mut u8,
    _rev: u32,
    mask: u32,
    sid: *const u8,
) -> i32 {
    unsafe { add_access_allowed_ace_ex(acl, _rev, 0, mask, sid) }
}

pub unsafe extern "win64" fn add_access_allowed_ace_ex(
    acl: *mut u8,
    _rev: u32,
    _flags: u32,
    _mask: u32,
    sid: *const u8,
) -> i32 {
    if acl.is_null() || sid.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    let current_count = unsafe { (acl.add(4) as *const u16).read_unaligned() as usize };
    unsafe {
        (acl.add(4) as *mut u16).write_unaligned((current_count + 1) as u16);
    }
    1
}

pub unsafe extern "win64" fn get_security_descriptor_sacl(
    sd: *const u8,
    present: *mut i32,
    sacl: *mut *mut u8,
    defaulted: *mut i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control = (sd.add(2) as *const u16).read_unaligned();
        let is_present = (control & 0x0010) != 0;
        if !present.is_null() {
            *present = is_present as i32;
        }
        if !defaulted.is_null() {
            *defaulted = 0;
        }
        if !sacl.is_null() {
            if is_present {
                if control & 0x8000 != 0 {
                    let off = (sd.add(12) as *const u32).read_unaligned() as usize;
                    *sacl = if off != 0 { sd.add(off) as *mut u8 } else { std::ptr::null_mut() };
                } else {
                    *sacl = (sd.add(12) as *const *mut u8).read_unaligned();
                }
            } else {
                *sacl = std::ptr::null_mut();
            }
        }
    }
    1
}

pub unsafe extern "win64" fn set_security_descriptor_sacl(
    sd: *mut u8,
    present: i32,
    sacl: *const u8,
    defaulted: i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control_ptr = sd.add(2) as *mut u16;
        let mut control = control_ptr.read_unaligned();
        if present != 0 {
            control |= 0x0010;
            if defaulted != 0 {
                control |= 0x0020;
            } else {
                control &= !0x0020;
            }
            (sd.add(12) as *mut usize).write_unaligned(sacl as usize);
        } else {
            control &= !0x0010;
            (sd.add(12) as *mut usize).write_unaligned(0);
        }
        control_ptr.write_unaligned(control);
    }
    1
}

pub unsafe extern "win64" fn set_security_descriptor_owner(
    sd: *mut u8,
    owner: *const u8,
    defaulted: i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control_ptr = sd.add(2) as *mut u16;
        let mut control = control_ptr.read_unaligned();
        if defaulted != 0 {
            control |= 0x0001;
        } else {
            control &= !0x0001;
        }
        control_ptr.write_unaligned(control);
        (sd.add(4) as *mut usize).write_unaligned(owner as usize);
    }
    1
}

pub unsafe extern "win64" fn set_security_descriptor_group(
    sd: *mut u8,
    group: *const u8,
    defaulted: i32,
) -> i32 {
    if sd.is_null() || unsafe { *sd != 1 } {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        let control_ptr = sd.add(2) as *mut u16;
        let mut control = control_ptr.read_unaligned();
        if defaulted != 0 {
            control |= 0x0002;
        } else {
            control &= !0x0002;
        }
        control_ptr.write_unaligned(control);
        (sd.add(8) as *mut usize).write_unaligned(group as usize);
    }
    1
}

pub unsafe extern "win64" fn make_absolute_sd(
    self_rel: *const u8,
    abs: *mut u8,
    abs_size: *mut u32,
    dacl: *mut u8,
    dacl_size: *mut u32,
    _sacl: *mut u8,
    sacl_size: *mut u32,
    owner: *mut u8,
    owner_size: *mut u32,
    group: *mut u8,
    group_size: *mut u32,
) -> i32 {
    if abs_size.is_null() || dacl_size.is_null() || owner_size.is_null() || group_size.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        *abs_size = 20;
        *dacl_size = 8;
        if !sacl_size.is_null() { *sacl_size = 0; }
        *owner_size = 16;
        *group_size = 16;
    }
    if abs.is_null() || dacl.is_null() || owner.is_null() || group.is_null() {
        crate::api::system::error::set(122);
        return 0;
    }
    unsafe {
        if !self_rel.is_null() {
            let o_off = (self_rel.add(4) as *const u32).read_unaligned() as usize;
            if o_off != 0 {
                std::ptr::copy_nonoverlapping(self_rel.add(o_off), owner, 16);
            }
            let g_off = (self_rel.add(8) as *const u32).read_unaligned() as usize;
            if g_off != 0 {
                std::ptr::copy_nonoverlapping(self_rel.add(g_off), group, 16);
            }
            let d_off = (self_rel.add(16) as *const u32).read_unaligned() as usize;
            if d_off != 0 {
                std::ptr::copy_nonoverlapping(self_rel.add(d_off), dacl, 8);
            }
        }
        *abs = 1;
        *abs.add(1) = 0;
        (abs.add(2) as *mut u16).write_unaligned(0x0004);
        (abs.add(4) as *mut usize).write_unaligned(owner as usize);
        (abs.add(8) as *mut usize).write_unaligned(group as usize);
        (abs.add(12) as *mut usize).write_unaligned(0);
        (abs.add(16) as *mut usize).write_unaligned(dacl as usize);
    }
    1
}

pub unsafe extern "win64" fn make_self_relative_sd(
    _abs: *const u8,
    self_rel: *mut u8,
    len: *mut u32,
) -> i32 {
    if len.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    if self_rel.is_null() || unsafe { *len } < 64 {
        unsafe { *len = 64 };
        crate::api::system::error::set(122);
        return 0;
    }
    unsafe {
        *len = 64;
        let mut owner_ptr: *mut u8 = std::ptr::null_mut();
        let mut group_ptr: *mut u8 = std::ptr::null_mut();
        let mut dacl_ptr: *mut u8 = std::ptr::null_mut();
        let mut sacl_ptr: *mut u8 = std::ptr::null_mut();
        let mut dummy: *mut u8 = std::ptr::null_mut();
        get_security_info(0, 0, 0, &mut owner_ptr, &mut group_ptr, &mut dacl_ptr, &mut sacl_ptr, &mut dummy);
        std::ptr::copy_nonoverlapping(dummy, self_rel, 64);
    }
    1
}




/// Windows TRUSTEE_W (accctrl.h). The SID is borrowed from the caller.
#[repr(C)]
pub struct TrusteeW {
    pub multiple_trustee: *mut TrusteeW,
    pub multiple_trustee_operation: i32,
    pub trustee_form: i32,
    pub trustee_type: i32,
    pub name: *mut u16,
}

pub unsafe extern "win64" fn build_trustee_with_sid_w(trustee: *mut TrusteeW, sid: *mut u8) {
    // The Windows API leaves behavior undefined for an invalid trustee pointer.
    if trustee.is_null() {
        return;
    }
    unsafe {
        trustee.write(TrusteeW {
            multiple_trustee: std::ptr::null_mut(),
            multiple_trustee_operation: 0, // NO_MULTIPLE_TRUSTEE
            trustee_form: 0,               // TRUSTEE_IS_SID
            trustee_type: 0,               // TRUSTEE_IS_UNKNOWN
            name: sid.cast(),
        });
    }
}

#[repr(C)]
pub struct ExplicitAccessW {
    pub permissions: u32,
    pub mode: u32,
    pub inheritance: u32,
    pub trustee: TrusteeW,
}

/// Construct an ACL for SID-based entries. The returned buffer belongs to
/// LocalAlloc and must be released by the caller with LocalFree.
pub unsafe extern "win64" fn set_entries_in_acl_w(
    count: u32,
    entries: *const ExplicitAccessW,
    old_acl: *const u8,
    new_acl: *mut *mut u8,
) -> u32 {
    const ERROR_INVALID_PARAMETER: u32 = 87;
    const ERROR_NOT_SUPPORTED: u32 = 50;
    if new_acl.is_null() || count > 64 || (count != 0 && entries.is_null()) {
        return ERROR_INVALID_PARAMETER;
    }
    unsafe { new_acl.write(std::ptr::null_mut()) };

    // The current GetSecurityInfo implementation supplies an empty DACL.
    // Merging populated ACLs needs ACE replacement and ordering semantics.
    if !old_acl.is_null() {
        let revision = unsafe { *old_acl };
        let size = unsafe { (old_acl.add(2) as *const u16).read_unaligned() };
        let old_count = unsafe { (old_acl.add(4) as *const u16).read_unaligned() };
        if revision != 2 || size < 8 {
            return ERROR_INVALID_PARAMETER;
        }
        if old_count != 0 {
            eprintln!("SetEntriesInAclW: merging a populated ACL is not supported");
            return ERROR_NOT_SUPPORTED;
        }
    }

    let mut acl_size = 8usize;
    for index in 0..count as usize {
        let entry = unsafe { &*entries.add(index) };
        if entry.trustee.multiple_trustee_operation != 0
            || !entry.trustee.multiple_trustee.is_null()
            || entry.trustee.trustee_form != 0
            || entry.inheritance > u8::MAX as u32
        {
            eprintln!("SetEntriesInAclW: unsupported trustee or inheritance");
            return ERROR_NOT_SUPPORTED;
        }
        if !matches!(entry.mode, 1..=3) {
            eprintln!("SetEntriesInAclW: unsupported access mode {}", entry.mode);
            return ERROR_NOT_SUPPORTED;
        }
        if entry.mode == 2 && count != 1 {
            eprintln!("SetEntriesInAclW: SET_ACCESS with multiple entries is not supported");
            return ERROR_NOT_SUPPORTED;
        }
        let sid = entry.trustee.name.cast::<u8>();
        if unsafe { is_valid_sid(sid) } == 0 {
            return ERROR_INVALID_PARAMETER;
        }
        acl_size += 8 + unsafe { get_length_sid(sid) } as usize;
    }
    if acl_size > u16::MAX as usize {
        return ERROR_INVALID_PARAMETER;
    }
    let acl = crate::api::memory::local::allocate(0, acl_size) as *mut u8;
    if acl.is_null() {
        return 8; // ERROR_NOT_ENOUGH_MEMORY
    }
    unsafe {
        *acl = 2; // ACL_REVISION
        *acl.add(1) = 0;
        (acl.add(2) as *mut u16).write_unaligned(acl_size as u16);
        (acl.add(4) as *mut u16).write_unaligned(count as u16);
        (acl.add(6) as *mut u16).write_unaligned(0);
        let mut offset = 8usize;
        // Deny entries precede allow entries in a Windows DACL.
        for denied in [true, false] {
            for index in 0..count as usize {
                let entry = &*entries.add(index);
                if (entry.mode == 3) != denied {
                    continue;
                }
                let sid = entry.trustee.name.cast::<u8>();
                let sid_len = get_length_sid(sid) as usize;
                let ace = acl.add(offset);
                *ace = if denied { 1 } else { 0 }; // ACCESS_DENIED/ALLOWED_ACE_TYPE
                *ace.add(1) = entry.inheritance as u8;
                (ace.add(2) as *mut u16).write_unaligned((8 + sid_len) as u16);
                (ace.add(4) as *mut u32).write_unaligned(entry.permissions);
                std::ptr::copy_nonoverlapping(sid, ace.add(8), sid_len);
                offset += 8 + sid_len;
            }
        }
        new_acl.write(acl);
    }
    0 // ERROR_SUCCESS
}
