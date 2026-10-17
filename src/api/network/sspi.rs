//! secur32.dll: the SSPI dispatch table without security packages (peek's
//! secur32.inc). InitSecurityInterfaceW/A return the table (libcurl loads it at
//! startup); every package function fails with SEC_E_UNSUPPORTED_FUNCTION, so
//! Schannel TLS and Negotiate/NTLM logins fail when used, as on a system without
//! those packages.
use std::sync::{atomic::AtomicBool, OnceLock};
const SEC_E_UNSUPPORTED_FUNCTION: i32 = 0x8009_0302u32 as i32;
pub extern "win64" fn unsupported() -> i32 {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        eprintln!("secur32: no security packages (Schannel/NTLM/Kerberos); SSPI calls fail with SEC_E_UNSUPPORTED_FUNCTION");
    }
    SEC_E_UNSUPPORTED_FUNCTION
}
/// SecurityFunctionTable: dwVersion, then 32 pointer slots (the version 4 layout);
/// the reserved slots stay NULL as on Windows.
#[repr(C)]
pub struct Table {
    version: u32,
    functions: [usize; 32],
}
struct Shared(Table);
unsafe impl Sync for Shared {}
pub extern "win64" fn init_security_interface() -> *const Table {
    static TABLE: OnceLock<Shared> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut functions = [unsupported as *const () as usize; 32];
        for reserved in [4, 17, 18, 21] {
            functions[reserved] = 0;
        }
        Shared(Table {
            version: 4,
            functions,
        })
    });
    &table.0
}

use std::ffi::c_void;
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Luid {
    pub low_part: u32,
    pub high_part: i32,
}

#[repr(C)]
pub struct UnicodeString {
    pub length: u16,
    pub maximum_length: u16,
    pub buffer: *mut u16,
}

#[repr(C)]
pub struct SecurityLogonSessionData {
    pub size: u32,
    pub logon_id: Luid,
    pub user_name: UnicodeString,
    pub logon_domain: UnicodeString,
    pub authentication_package: UnicodeString,
    pub logon_type: u32,
    pub session: u32,
    pub sid: *mut u8,
    pub logon_time: u64,
    pub logon_server: UnicodeString,
    pub dns_domain_name: UnicodeString,
    pub upn: UnicodeString,
}

pub unsafe extern "win64" fn lsa_enumerate_logon_sessions(
    count: *mut u32,
    list: *mut *mut Luid,
) -> i32 {
    if count.is_null() || list.is_null() {
        return 0xC000000Du32 as i32; // STATUS_INVALID_PARAMETER
    }
    let ptr = unsafe { malloc(size_of::<Luid>()) as *mut Luid };
    if ptr.is_null() {
        return 0xC0000017u32 as i32; // STATUS_NO_MEMORY
    }
    unsafe {
        *ptr = Luid {
            low_part: 0x3e7,
            high_part: 0,
        };
        *count = 1;
        *list = ptr;
    }
    0
}

pub unsafe extern "win64" fn lsa_free_return_buffer(buffer: *mut c_void) -> i32 {
    if !buffer.is_null() {
        unsafe { free(buffer) };
    }
    0
}

pub unsafe extern "win64" fn lsa_get_logon_session_data(
    logon_id: *const Luid,
    data: *mut *mut SecurityLogonSessionData,
) -> i32 {
    if logon_id.is_null() || data.is_null() {
        return 0xC000000Du32 as i32;
    }
    let ptr = unsafe { malloc(size_of::<SecurityLogonSessionData>()) as *mut SecurityLogonSessionData };
    if ptr.is_null() {
        return 0xC0000017u32 as i32;
    }
    unsafe {
        ptr.write_bytes(0, 1);
        (*ptr).size = size_of::<SecurityLogonSessionData>() as u32;
        (*ptr).logon_id = *logon_id;
        (*ptr).logon_type = 2; // Interactive
        *data = ptr;
    }
    0
}
