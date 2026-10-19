//! Host-only OpenSSL objects. Never expose these pointers to Windows code.
use std::{
    ffi::{c_char, c_void, CString},
    ptr,
};
type P = *mut c_void;
#[link(name = "crypto")]
unsafe extern "C" {
    fn d2i_X509(out: *mut P, input: *mut *const u8, length: i64) -> P;
    fn i2d_X509(cert: P, out: *mut *mut u8) -> i32;
    fn X509_free(cert: P);
    fn X509_STORE_new() -> P;
    fn X509_STORE_free(store: P);
    fn X509_STORE_set_default_paths(store: P) -> i32;
    fn X509_STORE_CTX_new() -> P;
    fn X509_STORE_CTX_free(ctx: P);
    fn X509_STORE_CTX_init(ctx: P, store: P, leaf: P, untrusted: P) -> i32;
    fn X509_STORE_CTX_set_time(ctx: P, flags: u64, time: i64);
    fn X509_STORE_CTX_set_purpose(ctx: P, purpose: i32) -> i32;
    fn X509_STORE_CTX_get0_param(ctx: P) -> P;
    fn X509_VERIFY_PARAM_set_auth_level(param: P, level: i32);
    fn X509_verify_cert(ctx: P) -> i32;
    fn X509_STORE_CTX_get_error(ctx: P) -> i32;
    fn X509_STORE_CTX_get_error_depth(ctx: P) -> i32;
    fn X509_STORE_CTX_get0_chain(ctx: P) -> P;
    fn OPENSSL_sk_new_null() -> P;
    fn OPENSSL_sk_free(stack: P);
    fn OPENSSL_sk_push(stack: P, value: P) -> i32;
    fn OPENSSL_sk_num(stack: P) -> i32;
    fn OPENSSL_sk_value(stack: P, index: i32) -> P;
    fn X509_check_host(cert: P, name: *const c_char, len: usize, flags: u32, peer: *mut P) -> i32;
    fn X509_check_ip_asc(cert: P, name: *const c_char, flags: u32) -> i32;
}
struct Owned {
    pointer: P,
    free: unsafe extern "C" fn(P),
}
impl Owned {
    fn new(pointer: P, free: unsafe extern "C" fn(P)) -> Result<Self, u32> {
        if pointer.is_null() {
            Err(8)
        } else {
            Ok(Self { pointer, free })
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            (self.free)(self.pointer);
        }
    }
}
fn parse(bytes: &[u8]) -> Result<Owned, u32> {
    let mut input = bytes.as_ptr();
    let p = unsafe { d2i_X509(ptr::null_mut(), &mut input, bytes.len() as i64) };
    Owned::new(p, X509_free).map_err(|_| 0x80092002)
}
pub struct Verified {
    pub error: i32,
    pub depth: usize,
    pub chain: Vec<Vec<u8>>,
}
pub fn verify(
    leaf: &[u8],
    intermediates: &[Vec<u8>],
    time: Option<i64>,
    purpose: i32,
    // When set, enforce a strong-signature policy: OpenSSL auth level 2 rejects
    // SHA-1/MD5 signatures and sub-2048-bit RSA, matching the intent of the
    // Windows well-known "OS current" strong-sign parameters.
    require_strong: bool,
) -> Result<Verified, u32> {
    unsafe {
        let leaf = parse(leaf)?;
        let roots = Owned::new(X509_STORE_new(), X509_STORE_free)?;
        if X509_STORE_set_default_paths(roots.pointer) != 1 {
            return Err(50);
        }
        let stack = Owned::new(OPENSSL_sk_new_null(), OPENSSL_sk_free)?;
        let mut owned = Vec::new();
        for bytes in intermediates {
            let cert = parse(bytes)?;
            if OPENSSL_sk_push(stack.pointer, cert.pointer) <= 0 {
                return Err(8);
            }
            owned.push(cert);
        }
        let ctx = Owned::new(X509_STORE_CTX_new(), X509_STORE_CTX_free)?;
        if X509_STORE_CTX_init(ctx.pointer, roots.pointer, leaf.pointer, stack.pointer) != 1 {
            return Err(8);
        }
        if require_strong {
            let param = X509_STORE_CTX_get0_param(ctx.pointer);
            if !param.is_null() {
                X509_VERIFY_PARAM_set_auth_level(param, 2);
            }
        }
        if let Some(seconds) = time {
            X509_STORE_CTX_set_time(ctx.pointer, 0, seconds);
        }
        if purpose != 0 && X509_STORE_CTX_set_purpose(ctx.pointer, purpose) != 1 {
            return Err(50);
        }
        let verified = X509_verify_cert(ctx.pointer);
        if verified < 0 {
            return Err(0x80092002);
        }
        let error = if verified == 1 {
            0
        } else {
            X509_STORE_CTX_get_error(ctx.pointer)
        };
        let depth = X509_STORE_CTX_get_error_depth(ctx.pointer).max(0) as usize;
        let built = X509_STORE_CTX_get0_chain(ctx.pointer);
        let count = if built.is_null() {
            0
        } else {
            OPENSSL_sk_num(built)
        };
        let mut chain = Vec::new();
        for i in 0..count {
            let cert = OPENSSL_sk_value(built, i);
            let size = i2d_X509(cert, ptr::null_mut());
            if size <= 0 {
                return Err(0x80092002);
            }
            let mut bytes = vec![0u8; size as usize];
            let mut out = bytes.as_mut_ptr();
            if i2d_X509(cert, &mut out) != size {
                return Err(0x80092002);
            }
            chain.push(bytes);
        }
        if chain.is_empty() {
            return Err(0x80092002);
        }
        Ok(Verified {
            error,
            depth,
            chain,
        })
    }
}
pub fn matches_name(leaf: &[u8], name: &str) -> Result<bool, u32> {
    let leaf = parse(leaf)?;
    let text = CString::new(name).map_err(|_| 87u32)?;
    let result = unsafe {
        if name.parse::<std::net::IpAddr>().is_ok() {
            X509_check_ip_asc(leaf.pointer, text.as_ptr(), 0)
        } else {
            X509_check_host(leaf.pointer, text.as_ptr(), name.len(), 4, ptr::null_mut())
        }
    };
    if result < 0 {
        Err(87)
    } else {
        Ok(result == 1)
    }
}

#[link(name = "crypto")]
unsafe extern "C" {
    fn EVP_Q_digest(
        context: *mut c_void,
        name: *const c_char,
        properties: *const c_char,
        data: *const u8,
        length: usize,
        out: *mut u8,
        size: *mut usize,
    ) -> i32;
}
pub(super) fn digest(bytes: &[u8], kind: u32) -> Result<Vec<u8>, u32> {
    let name = match kind {
        3 => c"SHA1",
        4 => c"MD5",
        107 => c"SHA256",
        _ => return Err(0x80092004),
    };
    digest_named(bytes, name)
}
/// A digest by OpenSSL name ("SHA256", "SHA1", ...).
pub(super) fn digest_named(bytes: &[u8], name: &std::ffi::CStr) -> Result<Vec<u8>, u32> {
    let mut out = vec![0u8; 64];
    let mut size = out.len();
    if unsafe {
        EVP_Q_digest(
            ptr::null_mut(),
            name.as_ptr(),
            ptr::null(),
            bytes.as_ptr(),
            bytes.len(),
            out.as_mut_ptr(),
            &mut size,
        )
    } != 1
    {
        return Err(50);
    }
    out.truncate(size);
    Ok(out)
}
