//! CRYPT32 certificate stores and reference-counted Windows contexts.
//! The ROOT store is a read-only snapshot of the Linux trust bundle. Loading
//! these certificates does not skip signature, hostname or chain verification.
pub(crate) mod chain;
mod der;
mod encode;
mod openssl;
mod pem;
mod selfsign;
pub use encode::encode_object_ex;
pub use selfsign::create_self_sign;
use crate::{api::system::error, host::paths};
use der::Certificate;
use std::{
    collections::HashMap,
    ffi::{c_char, c_void},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
};
#[repr(C)]
pub struct Context {
    encoding: u32,
    encoded: *const u8,
    size: u32,
    info: *const der::Info,
    store: usize,
}
struct Reference {
    context: Box<Context>,
    certificate: Arc<Certificate>,
    owner: Option<Arc<Store>>,
    refs: u32,
}
// Context pointers refer to immutable allocations owned by the adjacent Arc.
unsafe impl Send for Reference {}
struct Store {
    certificates: Mutex<Vec<Arc<Certificate>>>,
    readonly: bool,
    /// CERT_STORE_PROV_COLLECTION: the sibling stores whose certificates it presents.
    siblings: Option<Mutex<Vec<Arc<Store>>>>,
}
impl Store {
    /// The certificates the store presents: its own, then each sibling's in order.
    fn all(&self) -> Vec<Arc<Certificate>> {
        let mut out = self.certificates.lock().unwrap().clone();
        if let Some(siblings) = &self.siblings {
            for sibling in siblings.lock().unwrap().iter() {
                out.extend(sibling.all());
            }
        }
        out
    }
}
#[derive(Default)]
struct Registry {
    stores: HashMap<usize, Arc<Store>>,
    contexts: HashMap<usize, Reference>,
}
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x6b000000);
fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}
fn fail<T>(code: u32, value: T) -> T {
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!("crypt32: API failure {code:#x}");
    }
    error::set(code);
    value
}
fn new_context(r: &mut Registry, store: usize, certificate: Arc<Certificate>) -> *const Context {
    let context = Box::new(Context {
        encoding: 1,
        encoded: certificate.encoded.as_ptr(),
        size: certificate.encoded.len() as u32,
        info: &*certificate.info,
        store,
    });
    let pointer = &*context as *const Context;
    r.contexts.insert(
        pointer as usize,
        Reference {
            context,
            certificate,
            owner: r.stores.get(&store).cloned(),
            refs: 1,
        },
    );
    pointer
}
fn release(r: &mut Registry, pointer: usize) -> bool {
    let Some(reference) = r.contexts.get_mut(&pointer) else {
        return false;
    };
    reference.refs -= 1;
    if reference.refs == 0 {
        r.contexts.remove(&pointer);
    }
    true
}
pub unsafe extern "win64" fn open(
    provider: *const c_char,
    _encoding: u32,
    crypt_provider: usize,
    flags: u32,
    parameter: *const c_void,
) -> usize {
    if crypt_provider != 0 {
        return fail(87, 0);
    }
    let kind = if provider as usize >= 65536 {
        match unsafe { paths::narrow(provider) }.ok().as_deref() {
            Some("Memory") => 2,
            Some("System") => 10,
            _ => return fail(50, 0),
        }
    } else {
        provider as usize
    };
    if !matches!(kind, 2 | 9 | 10 | 11) {
        return fail(50, 0);
    }
    // No-crypt-release, deferred close, read-only, open-existing; other operations
    // (delete, persistence and physical providers) cannot silently be accepted.
    if flags & 0xffff & !(1 | 4 | 0x8000 | 0x4000 | 0x2000) != 0 {
        return fail(50, 0);
    }
    if flags & 0x6000 == 0x6000 {
        return fail(87, 0);
    }
    let (certificates, readonly) = if kind == 2 || kind == 11 {
        if flags >> 16 != 0 {
            return fail(87, 0);
        }
        (Vec::new(), flags & 0x8000 != 0)
    } else {
        if !matches!(flags >> 16, 1 | 2) {
            return fail(50, 0);
        }
        let name = if kind == 9 {
            unsafe { paths::narrow(parameter.cast()) }
        } else {
            unsafe { paths::wide(parameter.cast()) }
        };
        let name = match name {
            Ok(name) => name,
            Err(code) => return fail(code, 0),
        };
        if flags & 0x2000 != 0 {
            return fail(0x80092005, 0);
        }
        if name.eq_ignore_ascii_case("MY") {
            // The personal store exists and is empty: nothing persists host certificates
            // with private keys for Windows programs.
            if flags & 0x8000 == 0 {
                eprintln!("crypt32: writable MY store is not supported");
                return fail(50, 0);
            }
            (Vec::new(), true)
        } else if !name.eq_ignore_ascii_case("ROOT") {
            return fail(2, 0);
        } else {
        match pem::roots() {
            Ok(certs) => {
                eprintln!(
                    "crypt32: ROOT store with {} certificates from the host CA bundle",
                    certs.len()
                );
                (certs, true)
            }
            Err(code) => return fail(code, 0),
        }
        }
    };
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    registry().lock().unwrap().stores.insert(
        handle,
        Arc::new(Store {
            certificates: Mutex::new(certificates),
            readonly,
            siblings: (kind == 11).then(|| Mutex::new(Vec::new())),
        }),
    );
    handle
}
pub unsafe extern "win64" fn open_system_a(provider: usize, name: *const c_char) -> usize {
    unsafe { open(9 as *const c_char, 0, provider, 0x10000, name.cast()) }
}
pub unsafe extern "win64" fn open_system_w(provider: usize, name: *const u16) -> usize {
    unsafe { open(10 as *const c_char, 0, provider, 0x10000, name.cast()) }
}
pub extern "win64" fn enumerate(store: usize, previous: *const Context) -> *const Context {
    let mut r = registry().lock().unwrap();
    let previous = if previous.is_null() {
        None
    } else {
        let Some(reference) = r.contexts.get(&(previous as usize)) else {
            return fail(87, std::ptr::null());
        };
        let value = (reference.context.store, reference.certificate.clone());
        release(&mut r, previous as usize);
        Some(value)
    };
    let Some(opened) = r.stores.get(&store) else {
        return fail(6, std::ptr::null());
    };
    let certificates = opened.all();
    let next = match previous {
        None => 0,
        Some((owner, cert)) => {
            if owner != store {
                return fail(87, std::ptr::null());
            }
            match certificates.iter().position(|c| Arc::ptr_eq(c, &cert)) {
                Some(index) => index + 1,
                None => return fail(87, std::ptr::null()),
            }
        }
    };
    let Some(certificate) = certificates.get(next).cloned() else {
        return fail(0x80092004, std::ptr::null());
    };
    drop(certificates);
    new_context(&mut r, store, certificate)
}
pub extern "win64" fn duplicate(context: *const Context) -> *const Context {
    let mut r = registry().lock().unwrap();
    let Some(reference) = r.contexts.get_mut(&(context as usize)) else {
        return fail(87, std::ptr::null());
    };
    reference.refs += 1;
    context
}
pub extern "win64" fn free(context: *const Context) -> i32 {
    if context.is_null() {
        return 1;
    }
    if release(&mut registry().lock().unwrap(), context as usize) {
        1
    } else {
        fail(87, 0)
    }
}
pub extern "win64" fn close(store: usize, flags: u32) -> i32 {
    if flags & !2 != 0 {
        return fail(50, 0);
    }
    let mut r = registry().lock().unwrap();
    if r.stores.remove(&store).is_none() {
        return fail(6, 0);
    }
    if flags & 2 != 0
        && r.contexts
            .values()
            .any(|reference| reference.context.store == store)
    {
        return fail(0x8009200f, 0);
    }
    1
}
pub extern "win64" fn control_store(
    store: usize,
    _flags: u32,
    _ctrl_type: u32,
    _ctrl_para: *const std::ffi::c_void,
) -> i32 {
    let r = registry().lock().unwrap();
    if !r.stores.contains_key(&store) {
        return fail(6, 0);
    }
    1
}
pub unsafe extern "win64" fn create_context(
    encoding: u32,
    data: *const u8,
    size: u32,
) -> *const Context {
    if encoding & 0xffff != 1 || data.is_null() || size == 0 {
        return fail(87, std::ptr::null());
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, size as usize) }.to_vec();
    let certificate = match Certificate::parse(bytes) {
        Ok(certificate) => Arc::new(certificate),
        Err(()) => return fail(0x8009310b, std::ptr::null()),
    };
    new_context(&mut registry().lock().unwrap(), 0, certificate)
}
pub unsafe extern "win64" fn add(
    store: usize,
    encoding: u32,
    data: *const u8,
    size: u32,
    disposition: u32,
    out: *mut *const Context,
) -> i32 {
    if !out.is_null() {
        unsafe {
            *out = std::ptr::null();
        }
    }
    if encoding & 0xffff != 1 || data.is_null() || size == 0 || !(1..=7).contains(&disposition) {
        return fail(87, 0);
    }
    if disposition > 4 {
        return fail(50, 0);
    }
    let certificate = match Certificate::parse(
        unsafe { std::slice::from_raw_parts(data, size as usize) }.to_vec(),
    ) {
        Ok(cert) => Arc::new(cert),
        Err(()) => return fail(0x8009310b, 0),
    };
    let mut r = registry().lock().unwrap();
    let Some(opened) = r.stores.get_mut(&store) else {
        return fail(6, 0);
    };
    if opened.readonly {
        return fail(5, 0);
    }
    let mut certificates = opened.certificates.lock().unwrap();
    let existing = certificates
        .iter()
        .position(|c| same_identity(c, &certificate));
    let certificate = match (existing, disposition) {
        (Some(_), 1) => return fail(0x80092005, 0),
        (Some(index), 2) => certificates[index].clone(),
        (Some(index), 3) => {
            certificates[index] = certificate.clone();
            certificate
        }
        _ => {
            certificates.push(certificate.clone());
            certificate
        }
    };
    drop(certificates);
    if !out.is_null() {
        unsafe {
            *out = new_context(&mut r, store, certificate);
        }
    }
    1
}
fn same_identity(a: &Certificate, b: &Certificate) -> bool {
    unsafe {
        std::slice::from_raw_parts(a.info.serial.data, a.info.serial.size as usize)
            == std::slice::from_raw_parts(b.info.serial.data, b.info.serial.size as usize)
            && std::slice::from_raw_parts(a.info.issuer.data, a.info.issuer.size as usize)
                == std::slice::from_raw_parts(b.info.issuer.data, b.info.issuer.size as usize)
    }
}

pub(crate) mod properties;

/// CertAddStoreToCollection: flags and priority do not change enumeration order here
/// (siblings are presented in the order they were added).
pub extern "win64" fn add_store_to_collection(collection: usize, sibling: usize, _flags: u32, _priority: u32) -> i32 {
    let r = registry().lock().unwrap();
    let (Some(collection), Some(sibling)) = (r.stores.get(&collection), r.stores.get(&sibling)) else {
        return fail(6, 0);
    };
    let Some(siblings) = &collection.siblings else {
        return fail(87, 0);
    };
    siblings.lock().unwrap().push(sibling.clone());
    1
}
/// CertFindCertificateInStore: CERT_FIND_ANY and CERT_FIND_SHA1_HASH (CRYPT_HASH_BLOB).
/// Other find types fail with ERROR_NOT_SUPPORTED. The previous context is released.
pub unsafe extern "win64" fn find(
    store: usize, encoding: u32, flags: u32, kind: u32, parameter: *const c_void, previous: *const Context,
) -> *const Context {
    const CERT_FIND_ANY: u32 = 0;
    const CERT_FIND_SHA1_HASH: u32 = 1 << 16;
    let mut r = registry().lock().unwrap();
    let after = if previous.is_null() {
        None
    } else {
        let Some(reference) = r.contexts.get(&(previous as usize)) else {
            return fail(87, std::ptr::null());
        };
        let certificate = reference.certificate.clone();
        release(&mut r, previous as usize);
        Some(certificate)
    };
    if encoding & 0xffff != 1 && kind != CERT_FIND_ANY {
        return fail(87, std::ptr::null());
    }
    let wanted = match kind {
        CERT_FIND_ANY => None,
        CERT_FIND_SHA1_HASH => {
            if parameter.is_null() {
                return fail(87, std::ptr::null());
            }
            let (size, data) = unsafe { (*parameter.cast::<u32>(), *parameter.cast::<u8>().add(8).cast::<*const u8>()) };
            if size != 0 && data.is_null() {
                return fail(87, std::ptr::null());
            }
            Some(if size == 0 { Vec::new() } else { unsafe { std::slice::from_raw_parts(data, size as usize) }.to_vec() })
        }
        _ => {
            eprintln!("crypt32: CertFindCertificateInStore type {kind:#x} flags {flags:#x}: unsupported");
            return fail(50, std::ptr::null());
        }
    };
    let Some(opened) = r.stores.get(&store) else {
        return fail(6, std::ptr::null());
    };
    let all = opened.all();
    let start = match &after {
        None => 0,
        Some(cert) => match all.iter().position(|c| Arc::ptr_eq(c, cert)) {
            Some(index) => index + 1,
            None => return fail(87, std::ptr::null()),
        },
    };
    for certificate in &all[start..] {
        let matched = match &wanted {
            None => true,
            Some(hash) => openssl::digest(&certificate.encoded, 3).is_ok_and(|h| &h == hash),
        };
        if matched {
            return new_context(&mut r, store, certificate.clone());
        }
    }
    fail(0x80092004, std::ptr::null())
}
/// CryptHashCertificate2: a CNG algorithm name (SHA1, SHA256, SHA384, SHA512, MD5).
pub unsafe extern "win64" fn hash_certificate2(
    algorithm: *const u16, flags: u32, reserved: *const c_void,
    data: *const u8, size: u32, hash: *mut u8, hash_size: *mut u32,
) -> i32 {
    if algorithm.is_null() || hash_size.is_null() || (data.is_null() && size != 0) {
        return fail(87, 0);
    }
    if flags != 0 || !reserved.is_null() {
        return fail(87, 0);
    }
    let name = match unsafe { paths::wide(algorithm) } {
        Ok(name) => name.to_ascii_uppercase(),
        Err(code) => return fail(code, 0),
    };
    let name = match name.as_str() {
        "SHA1" => c"SHA1",
        "SHA256" => c"SHA256",
        "SHA384" => c"SHA384",
        "SHA512" => c"SHA512",
        "MD5" => c"MD5",
        _ => return fail(0x80090008, 0), // NTE_BAD_ALGID
    };
    let input = if size == 0 { &[][..] } else { unsafe { std::slice::from_raw_parts(data, size as usize) } };
    let digest = match openssl::digest_named(input, name) {
        Ok(digest) => digest,
        Err(code) => return fail(code, 0),
    };
    let capacity = unsafe { *hash_size } as usize;
    unsafe { *hash_size = digest.len() as u32 };
    if hash.is_null() {
        return 1;
    }
    if capacity < digest.len() {
        return fail(234, 0);
    }
    unsafe { std::ptr::copy_nonoverlapping(digest.as_ptr(), hash, digest.len()) };
    1
}
/// CertVerifyTimeValidity: -1 before NotBefore, 1 after NotAfter, otherwise 0.
/// `time` is a FILETIME, or NULL for the current time.
pub unsafe extern "win64" fn verify_time_validity(time: *const u64, info: *const c_void) -> i32 {
    if info.is_null() {
        return fail(87, 0);
    }
    let now = if time.is_null() {
        let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        since.as_secs() * 10_000_000 + since.subsec_nanos() as u64 / 100 + 116_444_736_000_000_000
    } else {
        unsafe { time.read_unaligned() }
    };
    let info = unsafe { &*info.cast::<der::Info>() };
    let filetime = |t: [u32; 2]| t[0] as u64 | (t[1] as u64) << 32;
    if now < filetime(info.not_before) {
        -1
    } else if now > filetime(info.not_after) {
        1
    } else {
        0
    }
}

/// The DER bytes of a certificate context this module handed out.
pub(crate) fn context_der(context: *const c_void) -> Option<Vec<u8>> {
    let r = registry().lock().unwrap();
    r.contexts.get(&(context as usize)).map(|reference| reference.certificate.encoded.to_vec())
}
/// A new standalone certificate context (freed with CertFreeCertificateContext).
pub(crate) fn context_from_der(der: Vec<u8>) -> Option<*const c_void> {
    let certificate = Arc::new(Certificate::parse(der).ok()?);
    Some(new_context(&mut registry().lock().unwrap(), 0, certificate).cast())
}
/// The private key created with a self-signed certificate (OpenSSL DER), if any.
pub(crate) fn private_key(der: &[u8]) -> Option<Vec<u8>> {
    selfsign::private_key(der)
}
