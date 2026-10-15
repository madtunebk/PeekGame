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
