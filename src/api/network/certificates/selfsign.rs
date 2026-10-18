//! CertCreateSelfSignCertificate over OpenSSL: a new RSA-2048 key and a v3 certificate
//! whose subject and issuer are the caller's encoded name. The private key stays on
//! the host side, keyed by the certificate's DER, for the SSPI credential that uses it.
use super::{der::Certificate, fail, new_context, registry};
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    ptr,
    sync::{Arc, Mutex},
};
type P = *mut c_void;
#[link(name = "crypto")]
unsafe extern "C" {
    fn EVP_PKEY_Q_keygen(context: P, properties: *const c_char, kind: *const c_char, ...) -> P;
    fn EVP_PKEY_free(key: P);
    fn X509_new() -> P;
    fn X509_free(cert: P);
    fn X509_set_version(cert: P, version: i64) -> i32;
    fn X509_get_serialNumber(cert: P) -> P;
    fn ASN1_INTEGER_set_int64(integer: P, value: i64) -> i32;
    fn d2i_X509_NAME(out: *mut P, input: *mut *const u8, length: i64) -> P;
    fn X509_NAME_free(name: P);
    fn X509_set_issuer_name(cert: P, name: P) -> i32;
    fn X509_set_subject_name(cert: P, name: P) -> i32;
    fn X509_getm_notBefore(cert: P) -> P;
    fn X509_getm_notAfter(cert: P) -> P;
    fn ASN1_TIME_set(time: P, seconds: i64) -> P;
    fn X509_set_pubkey(cert: P, key: P) -> i32;
    fn X509_sign(cert: P, key: P, digest: *const c_void) -> i32;
    fn EVP_sha1() -> *const c_void;
    fn EVP_sha256() -> *const c_void;
    fn EVP_sha384() -> *const c_void;
    fn EVP_sha512() -> *const c_void;
    fn i2d_X509(cert: P, out: *mut *mut u8) -> i32;
    fn i2d_PrivateKey(key: P, out: *mut *mut u8) -> i32;
}
unsafe extern "C" {
    fn getrandom(buffer: *mut u8, length: usize, flags: u32) -> isize;
}
const NTE_BAD_ALGID: u32 = 0x8009_0008;
const UNIX_TO_FILETIME_SECONDS: i64 = 11_644_473_600;

/// Private keys (DER, OpenSSL "traditional" form) of certificates created here.
static KEYS: Mutex<Option<HashMap<Vec<u8>, Vec<u8>>>> = Mutex::new(None);
/// The private key created with a self-signed certificate, by certificate DER.
pub(crate) fn private_key(certificate: &[u8]) -> Option<Vec<u8>> {
    KEYS.lock().unwrap().as_ref()?.get(certificate).cloned()
}

struct Owned(P, unsafe extern "C" fn(P));
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { (self.1)(self.0) }
    }
}
fn owned(pointer: P, free: unsafe extern "C" fn(P)) -> Result<Owned, u32> {
    if pointer.is_null() { Err(0x8009_0020) } else { Ok(Owned(pointer, free)) } // NTE_FAIL
}
fn der(f: impl Fn(*mut *mut u8) -> i32) -> Result<Vec<u8>, u32> {
    let size = f(ptr::null_mut());
    if size <= 0 {
        return Err(0x8009_0020);
    }
    let mut bytes = vec![0u8; size as usize];
    let mut out = bytes.as_mut_ptr();
    if f(&mut out) != size {
        return Err(0x8009_0020);
    }
    Ok(bytes)
}
/// SYSTEMTIME (UTC) to Unix seconds; invalid times fail like SystemTimeToFileTime.
fn unix_seconds(time: *const u16) -> Result<i64, u32> {
    let mut filetime = 0u64;
    if unsafe { crate::api::system::time::system_time_to_file_time(time, &mut filetime) } == 0 {
        return Err(87);
    }
    Ok((filetime / 10_000_000) as i64 - UNIX_TO_FILETIME_SECONDS)
}
/// CRYPT_ALGORITHM_IDENTIFIER.pszObjId; NULL means sha1RSA, the documented default.
fn signature_digest(algorithm: *const c_void) -> Result<*const c_void, u32> {
    if algorithm.is_null() {
        return Ok(unsafe { EVP_sha1() });
    }
    let id = unsafe { *algorithm.cast::<*const c_char>() };
    if id.is_null() {
        return Err(87);
    }
    Ok(match unsafe { CStr::from_ptr(id) }.to_bytes() {
        b"1.2.840.113549.1.1.5" => unsafe { EVP_sha1() },
        b"1.2.840.113549.1.1.11" => unsafe { EVP_sha256() },
        b"1.2.840.113549.1.1.12" => unsafe { EVP_sha384() },
        b"1.2.840.113549.1.1.13" => unsafe { EVP_sha512() },
        _ => return Err(NTE_BAD_ALGID),
    })
}
fn create(name: &[u8], digest: *const c_void, start: i64, end: i64) -> Result<(Vec<u8>, Vec<u8>), u32> {
    let key = owned(unsafe { EVP_PKEY_Q_keygen(ptr::null_mut(), ptr::null(), c"RSA".as_ptr(), 2048usize) }, EVP_PKEY_free)?;
    let cert = owned(unsafe { X509_new() }, X509_free)?;
    let mut input = name.as_ptr();
    let name = owned(unsafe { d2i_X509_NAME(ptr::null_mut(), &mut input, name.len() as i64) }, X509_NAME_free)
        .map_err(|_| 0x8009_2002u32)?; // CRYPT_E_BAD_ENCODE
    let mut serial = [0u8; 8];
    if unsafe { getrandom(serial.as_mut_ptr(), serial.len(), 0) } != serial.len() as isize {
        return Err(0x8009_0020);
    }
    let serial = (i64::from_le_bytes(serial) & i64::MAX).max(1);
    let ok = unsafe {
        X509_set_version(cert.0, 2) == 1
            && ASN1_INTEGER_set_int64(X509_get_serialNumber(cert.0), serial) == 1
            && X509_set_issuer_name(cert.0, name.0) == 1
            && X509_set_subject_name(cert.0, name.0) == 1
            && !ASN1_TIME_set(X509_getm_notBefore(cert.0), start).is_null()
            && !ASN1_TIME_set(X509_getm_notAfter(cert.0), end).is_null()
            && X509_set_pubkey(cert.0, key.0) == 1
            && X509_sign(cert.0, key.0, digest) > 0
    };
    if !ok {
        return Err(0x8009_0020);
    }
    let certificate = der(|out| unsafe { i2d_X509(cert.0, out) })?;
    let private = der(|out| unsafe { i2d_PrivateKey(key.0, out) })?;
    Ok((certificate, private))
}

/// Only the defaults are supported: no caller key or provider (a new key is created),
/// no flags, no extensions. Other requests fail with ERROR_NOT_SUPPORTED.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn create_self_sign(
    provider: usize,
    subject: *const c_void,
    flags: u32,
    key_info: *const c_void,
    algorithm: *const c_void,
    start: *const u16,
    end: *const u16,
    extensions: *const c_void,
) -> *const super::Context {
    // CERT_NAME_BLOB { cbData @0, pbData @8 }.
    if subject.is_null() {
        return fail(87, ptr::null());
    }
    let (size, data) = unsafe { (*subject.cast::<u32>(), *subject.cast::<u8>().add(8).cast::<*const u8>()) };
    if size == 0 || data.is_null() {
        return fail(87, ptr::null());
    }
    let extension_count = if extensions.is_null() { 0 } else { unsafe { *extensions.cast::<u32>() } };
    if provider != 0 || flags != 0 || !key_info.is_null() || extension_count != 0 {
        eprintln!("crypt32: CertCreateSelfSignCertificate provider={provider:#x} flags={flags:#x} key_info={key_info:?} extensions={extension_count}: unsupported");
        return fail(50, ptr::null());
    }
    let digest = match signature_digest(algorithm) {
        Ok(digest) => digest,
        Err(code) => return fail(code, ptr::null()),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let start = if start.is_null() { Ok(now) } else { unix_seconds(start) };
    let start = match start {
        Ok(start) => start,
        Err(code) => return fail(code, ptr::null()),
    };
    // Default end: one year after the start.
    let end = if end.is_null() { Ok(start + 365 * 86_400) } else { unix_seconds(end) };
    let end = match end {
        Ok(end) => end,
        Err(code) => return fail(code, ptr::null()),
    };
    let name = unsafe { std::slice::from_raw_parts(data, size as usize) };
    let (encoded, private) = match create(name, digest, start, end) {
        Ok(pair) => pair,
        Err(code) => return fail(code, ptr::null()),
    };
    let certificate = match Certificate::parse(encoded.clone()) {
        Ok(certificate) => Arc::new(certificate),
        Err(()) => return fail(0x8009_310b, ptr::null()),
    };
    KEYS.lock().unwrap().get_or_insert_with(HashMap::new).insert(encoded, private);
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!("crypt32: CertCreateSelfSignCertificate -> RSA-2048 certificate, {} DER bytes", certificate.encoded.len());
    }
    new_context(&mut registry().lock().unwrap(), 0, certificate)
}
