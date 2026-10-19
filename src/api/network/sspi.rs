//! secur32.dll's SSPI dispatch table. InitSecurityInterfaceW/A return the table;
//! Schannel A calls share the implemented DTLS path, while unsupported packages
//! and TLS stream contexts return their SSPI error status.
use std::{ffi::{c_char, CStr, c_void}, sync::{atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering}, OnceLock}};
const SEC_E_UNSUPPORTED_FUNCTION: i32 = 0x8009_0302u32 as i32;
pub extern "win64" fn unsupported() -> i32 {
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        eprintln!("secur32: direct SSPI export called; package operation returns SEC_E_UNSUPPORTED_FUNCTION");
    }
    SEC_E_UNSUPPORTED_FUNCTION
}
extern "win64" fn unsupported_slot<const SLOT: u32>() -> i32 {
    static SEEN: AtomicU32 = AtomicU32::new(0);
    let mask = 1u32 << SLOT;
    if SEEN.fetch_or(mask, Ordering::Relaxed) & mask == 0 {
        eprintln!("secur32: SSPI dispatch slot {SLOT} called; returns SEC_E_UNSUPPORTED_FUNCTION");
    }
    SEC_E_UNSUPPORTED_FUNCTION
}
macro_rules! dispatch_slots {
    ($($slot:literal),+ $(,)?) => {
        [$(unsupported_slot::<$slot> as *const () as usize),+]
    };
}
pub unsafe extern "win64" fn acquire_credentials_handle_a(
    principal: *const c_char,
    package: *const c_char,
    usage: u32,
    logon_id: *const c_void,
    auth_data: *const c_void,
    get_key: usize,
    get_key_argument: usize,
    credential: *mut usize,
    expiry: *mut i64,
) -> i32 {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    let package_name = if package.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(package) }.to_string_lossy().into_owned()
    };
    if !package_name.eq_ignore_ascii_case("Microsoft Unified Security Protocol Provider")
        && !package_name.eq_ignore_ascii_case("Schannel")
    {
        if CALLS.fetch_add(1, Ordering::Relaxed) < 8 {
            eprintln!("secur32: AcquireCredentialsHandleA package={package_name:?} usage={usage:#x}; Schannel is the only supported package");
        }
        return 0x8009_0305u32 as i32; // SEC_E_SECPKG_NOT_FOUND
    }
    let (version, protocols, flags, tls_parameters) = if auth_data.is_null() {
        (0, 0, 0, 0)
    } else {
        let data = auth_data.cast::<u8>();
        unsafe {
            let version = data.cast::<u32>().read_unaligned();
            match version {
                4 => (version, data.add(56).cast::<u32>().read_unaligned(), data.add(72).cast::<u32>().read_unaligned(), 0),
                5 => (version, 0, data.add(52).cast::<u32>().read_unaligned(), data.add(56).cast::<u32>().read_unaligned()),
                _ => (version, 0, 0, 0),
            }
        }
    };
    if CALLS.fetch_add(1, Ordering::Relaxed) < 8 {
        eprintln!("secur32: AcquireCredentialsHandleA package={package_name:?} usage={usage:#x} schannel_cred_version={version} protocols={protocols:#x} flags={flags:#x} tls_parameters={tls_parameters}");
    }
    let wide_package: Vec<u16> = package_name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        crate::api::network::schannel::acquire_credentials_handle_w(
            principal.cast(),
            wide_package.as_ptr(),
            usage,
            logon_id,
            auth_data.cast(),
            get_key,
            get_key_argument,
            credential,
            expiry,
        )
    }
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn initialize_security_context_a(
    credential: *const usize,
    context: *const usize,
    target: *const c_char,
    requirements: u32,
    reserved1: u32,
    representation: u32,
    input: *const u8,
    reserved2: u32,
    new_context: *mut usize,
    output: *const u8,
    attributes: *mut u32,
    expiry: *mut i64,
) -> i32 {
    let wide_target: Option<Vec<u16>> = if target.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(target) }.to_str().ok().map(|s| s.encode_utf16().chain(std::iter::once(0)).collect())
    };
    let target_ptr = wide_target.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());
    let status = unsafe {
        crate::api::network::schannel::initialize_security_context_w(
            credential, context, target_ptr, requirements, reserved1,
            representation, input, reserved2, new_context, output, attributes, expiry,
        )
    };
    if std::env::var_os("PEEKGAME_TRACE_SSPI").is_some() {
        eprintln!("secur32: InitializeSecurityContextA requirements={requirements:#x} representation={representation} -> {status:#x}");
    }
    status
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
        let mut functions = dispatch_slots![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ];
        functions[2] = acquire_credentials_handle_a as *const () as usize;
        functions[3] = crate::api::network::schannel::free_credentials_handle as *const () as usize;
        functions[5] = initialize_security_context_a as *const () as usize;
        functions[6] = crate::api::network::schannel::accept_security_context as *const () as usize;
        functions[8] = crate::api::network::schannel::delete_security_context as *const () as usize;
        functions[10] = crate::api::network::schannel::query_context_attributes_w as *const () as usize;
        functions[15] = crate::api::network::schannel::free_context_buffer as *const () as usize;
        functions[24] = crate::api::network::schannel::encrypt_message as *const () as usize;
        functions[25] = crate::api::network::schannel::decrypt_message as *const () as usize;
        for reserved in [4, 17, 18, 22] {
            functions[reserved] = 0;
        }
        Shared(Table {
            version: 4,
            functions,
        })
    });
    &table.0
}

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
