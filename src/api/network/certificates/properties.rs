//! Certificate context properties; computed certificate hashes are real DER digests.
use super::{fail, openssl, registry, Context};
use std::ffi::c_void;
pub unsafe extern "win64" fn get(
    context: *const Context,
    id: u32,
    data: *mut c_void,
    size: *mut u32,
) -> i32 {
    if size.is_null() || id == 0 {
        return fail(87, 0);
    }
    let certificate = {
        let r = registry().lock().unwrap();
        let Some(reference) = r.contexts.get(&(context as usize)) else {
            return fail(87, 0);
        };
        reference.certificate.clone()
    };
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!("crypt32: CertGetCertificateContextProperty({id})");
    }
    // No user-set extended properties are attached to these immutable contexts.
    if !matches!(id, 3 | 4 | 107) {
        return fail(0x80092004, 0);
    }
    let bytes = match openssl::digest(&certificate.encoded, id) {
        Ok(bytes) => bytes,
        Err(code) => return fail(code, 0),
    };
    let capacity = unsafe { *size };
    unsafe {
        *size = bytes.len() as u32;
    }
    if data.is_null() {
        return 1;
    }
    if (capacity as usize) < bytes.len() {
        return fail(234, 0);
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.cast::<u8>(), bytes.len());
    }
    1
}
