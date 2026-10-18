//! SspiCli: the Schannel package ("Microsoft Unified Security Protocol Provider") for
//! DTLS 1.0/1.2 over OpenSSL, as PlayFab Party uses it.
//!
//! - Credentials: SCHANNEL_CRED (version 4) with at most one certificate created by
//!   CertCreateSelfSignCertificate (its private key is kept by CRYPT32 here), or
//!   PSK-only (SCH_USE_PRESHAREDKEY_ONLY with SECBUFFER_PRESHARED_KEY[_IDENTITY]).
//! - Contexts: InitializeSecurityContextW/AcceptSecurityContext with ISC/ASC_REQ_DATAGRAM.
//!   Every output token is one UDP datagram; a flight that needs several datagrams
//!   returns SEC_I_MESSAGE_FRAGMENT until the last one. A call without input while the
//!   handshake waits retransmits the last flight (the caller owns the timing).
//!   Peer certificates are not validated here (Party validates them itself through
//!   SECPKG_ATTR_REMOTE_CERT_CONTEXT, as with SCH_CRED_MANUAL_CRED_VALIDATION).
//! - EncryptMessage/DecryptMessage: one DTLS record per call (header/data/trailer
//!   buffers); DecryptMessage reports further records of the datagram as SECBUFFER_EXTRA.
//! - QueryContextAttributesW: SECPKG_ATTR_STREAM_SIZES and SECPKG_ATTR_REMOTE_CERT_CONTEXT.
//!
//! Stream (TLS) contexts and other packages fail with SEC_E_UNSUPPORTED_FUNCTION.
use crate::api::network::certificates;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::{c_char, c_int, c_long, c_uint, c_void},
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

type P = *mut c_void;
#[link(name = "ssl")]
unsafe extern "C" {
    fn DTLS_method() -> P;
    fn SSL_CTX_new(method: P) -> P;
    fn SSL_CTX_free(ctx: P);
    fn SSL_CTX_ctrl(ctx: P, cmd: c_int, larg: c_long, parg: P) -> c_long;
    fn SSL_CTX_set_cipher_list(ctx: P, list: *const c_char) -> c_int;
    fn SSL_CTX_set_security_level(ctx: P, level: c_int);
    fn SSL_CTX_use_certificate(ctx: P, cert: P) -> c_int;
    fn SSL_CTX_use_PrivateKey(ctx: P, key: P) -> c_int;
    fn SSL_CTX_set_verify(ctx: P, mode: c_int, callback: Option<unsafe extern "C" fn(c_int, P) -> c_int>);
    fn SSL_new(ctx: P) -> P;
    fn SSL_free(ssl: P);
    fn SSL_set_bio(ssl: P, rbio: P, wbio: P);
    fn SSL_set_connect_state(ssl: P);
    fn SSL_set_accept_state(ssl: P);
    fn SSL_do_handshake(ssl: P) -> c_int;
    fn SSL_get_error(ssl: P, ret: c_int) -> c_int;
    fn SSL_read(ssl: P, buffer: *mut c_void, length: c_int) -> c_int;
    fn SSL_write(ssl: P, buffer: *const c_void, length: c_int) -> c_int;
    fn SSL_ctrl(ssl: P, cmd: c_int, larg: c_long, parg: P) -> c_long;
    fn SSL_set_ex_data(ssl: P, index: c_int, data: P) -> c_int;
    fn SSL_get_ex_data(ssl: P, index: c_int) -> P;
    fn SSL_get1_peer_certificate(ssl: P) -> P;
    fn SSL_get_current_cipher(ssl: P) -> P;
    fn SSL_CIPHER_get_cipher_nid(cipher: P) -> c_int;
    fn SSL_set_psk_client_callback(
        ssl: P,
        callback: Option<unsafe extern "C" fn(P, *const c_char, *mut c_char, c_uint, *mut u8, c_uint) -> c_uint>,
    );
    fn SSL_set_psk_server_callback(
        ssl: P,
        callback: Option<unsafe extern "C" fn(P, *const c_char, *mut u8, c_uint) -> c_uint>,
    );
    fn SSL_get_psk_identity(ssl: P) -> *const c_char;
    fn DTLS_get_data_mtu(ssl: P) -> usize;
    fn DTLS_set_timer_cb(ssl: P, callback: Option<unsafe extern "C" fn(P, c_uint) -> c_uint>);
}
#[link(name = "crypto")]
unsafe extern "C" {
    fn BIO_meth_new(kind: c_int, name: *const c_char) -> P;
    fn BIO_meth_set_write(method: P, f: unsafe extern "C" fn(P, *const c_char, c_int) -> c_int) -> c_int;
    fn BIO_meth_set_read(method: P, f: unsafe extern "C" fn(P, *mut c_char, c_int) -> c_int) -> c_int;
    fn BIO_meth_set_ctrl(method: P, f: unsafe extern "C" fn(P, c_int, c_long, P) -> c_long) -> c_int;
    fn BIO_meth_set_create(method: P, f: unsafe extern "C" fn(P) -> c_int) -> c_int;
    fn BIO_new(method: P) -> P;
    fn BIO_set_data(bio: P, data: P);
    fn BIO_get_data(bio: P) -> P;
    fn BIO_set_init(bio: P, init: c_int);
    fn BIO_set_flags(bio: P, flags: c_int);
    fn BIO_clear_flags(bio: P, flags: c_int);
    fn d2i_X509(out: *mut P, input: *mut *const u8, length: c_long) -> P;
    fn i2d_X509(cert: P, out: *mut *mut u8) -> c_int;
    fn X509_free(cert: P);
    fn d2i_AutoPrivateKey(out: *mut P, input: *mut *const u8, length: c_long) -> P;
    fn EVP_PKEY_free(key: P);
    fn EVP_get_cipherbyname(name: *const c_char) -> P;
    fn OBJ_nid2sn(nid: c_int) -> *const c_char;
    fn EVP_CIPHER_get_mode(cipher: P) -> c_int;
    fn EVP_CIPHER_get_block_size(cipher: P) -> c_int;
    fn ERR_clear_error();
}
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

// ---- Windows constants ------------------------------------------------------------
const SEC_E_OK: i32 = 0;
const SEC_I_CONTINUE_NEEDED: i32 = 0x0009_0312;
const SEC_I_CONTEXT_EXPIRED: i32 = 0x0009_0317;
const SEC_I_MESSAGE_FRAGMENT: i32 = 0x0009_0364;
const SEC_E_INSUFFICIENT_MEMORY: i32 = 0x8009_0300u32 as i32;
const SEC_E_INVALID_HANDLE: i32 = 0x8009_0301u32 as i32;
const SEC_E_UNSUPPORTED_FUNCTION: i32 = 0x8009_0302u32 as i32;
const SEC_E_INTERNAL_ERROR: i32 = 0x8009_0304u32 as i32;
const SEC_E_SECPKG_NOT_FOUND: i32 = 0x8009_0305u32 as i32;
const SEC_E_INVALID_TOKEN: i32 = 0x8009_0308u32 as i32;
const SEC_E_UNKNOWN_CREDENTIALS: i32 = 0x8009_030Du32 as i32;
const SEC_E_NO_CREDENTIALS: i32 = 0x8009_030Eu32 as i32;
const SEC_E_MESSAGE_ALTERED: i32 = 0x8009_030Fu32 as i32;
const SEC_E_INCOMPLETE_MESSAGE: i32 = 0x8009_0318u32 as i32;
const SEC_E_BUFFER_TOO_SMALL: i32 = 0x8009_0321u32 as i32;
const SEC_E_ILLEGAL_MESSAGE: i32 = 0x8009_0326u32 as i32;
const SEC_E_ALGORITHM_MISMATCH: i32 = 0x8009_0331u32 as i32;

const SECBUFFER_EMPTY: u32 = 0;
const SECBUFFER_DATA: u32 = 1;
const SECBUFFER_TOKEN: u32 = 2;
const SECBUFFER_EXTRA: u32 = 5;
const SECBUFFER_STREAM_TRAILER: u32 = 6;
const SECBUFFER_STREAM_HEADER: u32 = 7;
const SECBUFFER_ALERT: u32 = 17;
const SECBUFFER_PRESHARED_KEY: u32 = 22;
const SECBUFFER_PRESHARED_KEY_IDENTITY: u32 = 23;
const SECBUFFER_DTLS_MTU: u32 = 24;
const SECBUFFER_ATTRMASK: u32 = 0xF000_0000;

const SECPKG_CRED_INBOUND: u32 = 1;
const SECPKG_CRED_OUTBOUND: u32 = 2;
const REQ_ALLOCATE_MEMORY: u32 = 0x100;
const REQ_DATAGRAM: u32 = 0x400;
const SCH_USE_PRESHAREDKEY_ONLY: u32 = 0x0080_0000;
const SP_PROT_DTLS1_0: u32 = 0x0001_0000 | 0x0002_0000;
const SP_PROT_DTLS1_2: u32 = 0x0004_0000 | 0x0008_0000;
const SECPKG_ATTR_STREAM_SIZES: u32 = 4;
const SECPKG_ATTR_REMOTE_CERT_CONTEXT: u32 = 0x53;
const PACKAGE: &str = "Microsoft Unified Security Protocol Provider";

// ---- OpenSSL constants ------------------------------------------------------------
const SSL_ERROR_WANT_READ: c_int = 2;
const SSL_ERROR_WANT_WRITE: c_int = 3;
const SSL_ERROR_ZERO_RETURN: c_int = 6;
const SSL_CTRL_OPTIONS: c_int = 32;
const SSL_CTRL_SET_MTU: c_int = 17;
const DTLS_CTRL_SET_LINK_MTU: c_int = 120;
const SSL_CTRL_SET_MIN_PROTO_VERSION: c_int = 123;
const SSL_CTRL_SET_MAX_PROTO_VERSION: c_int = 124;
const SSL_OP_NO_QUERY_MTU: c_long = 0x1000;
const DTLS1_VERSION: c_long = 0xFEFF;
const DTLS1_2_VERSION: c_long = 0xFEFD;
const SSL_VERIFY_PEER: c_int = 1;
const BIO_CTRL_FLUSH: c_int = 11;
const BIO_CTRL_PENDING: c_int = 10;
const BIO_CTRL_WPENDING: c_int = 13;
const BIO_CTRL_DGRAM_QUERY_MTU: c_int = 40;
const BIO_CTRL_DGRAM_GET_MTU_OVERHEAD: c_int = 49;
const BIO_FLAGS_READ: c_int = 0x01;
const BIO_FLAGS_SHOULD_RETRY: c_int = 0x08;
const BIO_TYPE_SOURCE_SINK: c_int = 0x0400;
const EVP_CIPH_CBC_MODE: c_int = 0x2;
const EVP_CIPH_GCM_MODE: c_int = 0x6;
const EVP_CIPH_CCM_MODE: c_int = 0x7;
const DTLS_RECORD_HEADER: usize = 13;

// ---- datagram BIO -----------------------------------------------------------------
/// Datagrams in both directions; each BIO_write/BIO_read is one datagram.
#[derive(Default)]
struct Datagrams {
    incoming: VecDeque<Vec<u8>>,
    outgoing: VecDeque<Vec<u8>>,
    mtu: usize,
}
unsafe extern "C" fn bio_create(bio: P) -> c_int {
    unsafe { BIO_set_init(bio, 1) };
    1
}
unsafe extern "C" fn bio_write(bio: P, data: *const c_char, length: c_int) -> c_int {
    let queue = unsafe { &mut *(BIO_get_data(bio) as *mut Datagrams) };
    if length <= 0 {
        return 0;
    }
    queue.outgoing.push_back(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length as usize) }.to_vec());
    length
}
unsafe extern "C" fn bio_read(bio: P, out: *mut c_char, length: c_int) -> c_int {
    let queue = unsafe { &mut *(BIO_get_data(bio) as *mut Datagrams) };
    unsafe { BIO_clear_flags(bio, BIO_FLAGS_READ | BIO_FLAGS_SHOULD_RETRY) };
    let Some(datagram) = queue.incoming.pop_front() else {
        unsafe { BIO_set_flags(bio, BIO_FLAGS_READ | BIO_FLAGS_SHOULD_RETRY) };
        return -1;
    };
    let n = datagram.len().min(length.max(0) as usize);
    unsafe { ptr::copy_nonoverlapping(datagram.as_ptr(), out.cast::<u8>(), n) };
    n as c_int
}
unsafe extern "C" fn bio_ctrl(bio: P, command: c_int, _number: c_long, _pointer: P) -> c_long {
    let queue = unsafe { &mut *(BIO_get_data(bio) as *mut Datagrams) };
    match command {
        BIO_CTRL_FLUSH => 1,
        BIO_CTRL_PENDING => queue.incoming.front().map_or(0, |d| d.len()) as c_long,
        BIO_CTRL_WPENDING => 0,
        BIO_CTRL_DGRAM_QUERY_MTU => queue.mtu as c_long,
        BIO_CTRL_DGRAM_GET_MTU_OVERHEAD => 28, // IPv4 + UDP; only used for diagnostics
        _ => 0,
    }
}
fn bio_method() -> P {
    struct Method(P);
    unsafe impl Send for Method {}
    unsafe impl Sync for Method {}
    static METHOD: std::sync::OnceLock<Method> = std::sync::OnceLock::new();
    METHOD
        .get_or_init(|| unsafe {
            let m = BIO_meth_new(BIO_TYPE_SOURCE_SINK | 0x80, c"peekgame-datagram".as_ptr());
            BIO_meth_set_write(m, bio_write);
            BIO_meth_set_read(m, bio_read);
            BIO_meth_set_ctrl(m, bio_ctrl);
            BIO_meth_set_create(m, bio_create);
            Method(m)
        })
        .0
}

// ---- credentials and contexts -------------------------------------------------------
struct Credential {
    server: bool,
    protocols: u32,
    psk_only: bool,
    certificate: Option<(Vec<u8>, Vec<u8>)>, // certificate DER, private key DER
}
struct Context {
    /// Contexts keep their credential alive, as on Windows.
    _credential: Arc<Credential>,
    ssl: P,
    ctx: P,
    datagrams: Box<Datagrams>,
    done: bool,
    psk: Option<(Vec<u8>, Vec<u8>)>, // identity, key
    peer_identity: Option<Vec<u8>>,
    /// The datagrams of the last flight this side sent, for caller-driven retransmission.
    last_flight: Vec<Vec<u8>>,
}
// The OpenSSL objects belong to this context alone and are used under its mutex.
unsafe impl Send for Context {}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            SSL_free(self.ssl); // also frees the BIO
            SSL_CTX_free(self.ctx);
        }
    }
}
static NEXT: AtomicUsize = AtomicUsize::new(0x5c00_0010);
const MAGIC: usize = 0x5343_4841_4e4e_454c; // "SCHANNEL"
static CREDENTIALS: Mutex<Option<HashMap<usize, Arc<Credential>>>> = Mutex::new(None);
static CONTEXTS: Mutex<Option<HashMap<usize, Arc<Mutex<Context>>>>> = Mutex::new(None);
static ALLOCATED: Mutex<Option<HashSet<usize>>> = Mutex::new(None);

fn trace() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_SSPI").is_some())
}
/// SecHandle { dwLower, dwUpper }.
unsafe fn handle_id(handle: *const usize) -> Option<usize> {
    if handle.is_null() {
        return None;
    }
    let (lower, upper) = unsafe { (*handle, *handle.add(1)) };
    (upper == MAGIC).then_some(lower)
}
unsafe fn set_handle(handle: *mut usize, id: usize) {
    unsafe {
        *handle = id;
        *handle.add(1) = MAGIC;
    }
}
unsafe fn set_expiry(expiry: *mut i64) {
    if !expiry.is_null() {
        unsafe { *expiry = i64::MAX };
    }
}

/// One SecBuffer (x64): cbBuffer @0, BufferType @4, pvBuffer @8.
#[repr(C)]
struct SecBuffer {
    size: u32,
    kind: u32,
    data: *mut u8,
}
unsafe fn buffers<'a>(desc: *const u8) -> &'a mut [SecBuffer] {
    if desc.is_null() {
        return &mut [];
    }
    let count = unsafe { *(desc.add(4) as *const u32) } as usize;
    let list = unsafe { *(desc.add(8) as *const *mut SecBuffer) };
    if list.is_null() || count == 0 {
        return &mut [];
    }
    unsafe { std::slice::from_raw_parts_mut(list, count) }
}
fn kind(buffer: &SecBuffer) -> u32 {
    buffer.kind & !SECBUFFER_ATTRMASK
}
fn bytes(buffer: &SecBuffer) -> Vec<u8> {
    if buffer.data.is_null() || buffer.size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(buffer.data, buffer.size as usize) }.to_vec()
    }
}

pub unsafe extern "win64" fn acquire_credentials_handle_w(
    _principal: *const u16,
    package: *const u16,
    usage: u32,
    _logon_id: *const c_void,
    auth_data: *const u8,
    _get_key: usize,
    _get_key_argument: usize,
    credential: *mut usize,
    expiry: *mut i64,
) -> i32 {
    let name = unsafe { crate::api::text::string(package) }.unwrap_or_default();
    if !name.eq_ignore_ascii_case(PACKAGE) && !name.eq_ignore_ascii_case("Schannel") {
        eprintln!("SspiCli: security package {name:?} is not available");
        return SEC_E_SECPKG_NOT_FOUND;
    }
    if credential.is_null() || !matches!(usage, SECPKG_CRED_INBOUND | SECPKG_CRED_OUTBOUND) {
        return SEC_E_UNKNOWN_CREDENTIALS;
    }
    // SCHANNEL_CRED (x64): dwVersion @0, cCreds @4, paCred @8, grbitEnabledProtocols @56, dwFlags @72.
    let (count, certificates_list, protocols, flags) = if auth_data.is_null() {
        (0, ptr::null::<usize>(), 0, 0)
    } else {
        unsafe {
            let version = *(auth_data as *const u32);
            if version != 4 {
                eprintln!("SspiCli: SCHANNEL_CRED version {version} is not supported");
                return SEC_E_UNKNOWN_CREDENTIALS;
            }
            (
                *(auth_data.add(4) as *const u32),
                *(auth_data.add(8) as *const *const usize),
                *(auth_data.add(56) as *const u32),
                *(auth_data.add(72) as *const u32),
            )
        }
    };
    let dtls = SP_PROT_DTLS1_0 | SP_PROT_DTLS1_2;
    let protocols = if protocols == 0 { dtls } else { protocols };
    if protocols & dtls == 0 {
        eprintln!("SspiCli: only DTLS is supported (protocols {protocols:#x})");
        return SEC_E_ALGORITHM_MISMATCH;
    }
    let certificate = if count == 0 {
        None
    } else {
        if count != 1 || certificates_list.is_null() {
            eprintln!("SspiCli: {count} certificates in SCHANNEL_CRED are not supported");
            return SEC_E_UNKNOWN_CREDENTIALS;
        }
        let context = unsafe { *certificates_list } as *const c_void;
        let Some(der) = certificates::context_der(context) else {
            return SEC_E_UNKNOWN_CREDENTIALS;
        };
        let Some(key) = certificates::private_key(&der) else {
            eprintln!("SspiCli: the certificate has no private key here (only CertCreateSelfSignCertificate keys are kept)");
            return SEC_E_NO_CREDENTIALS;
        };
        Some((der, key))
    };
    let psk_only = flags & SCH_USE_PRESHAREDKEY_ONLY != 0;
    let server = usage == SECPKG_CRED_INBOUND;
    if server && certificate.is_none() && !psk_only {
        return SEC_E_NO_CREDENTIALS;
    }
    let id = NEXT.fetch_add(16, Ordering::Relaxed);
    CREDENTIALS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(id, Arc::new(Credential { server, protocols: protocols & dtls, psk_only, certificate }));
    unsafe {
        set_handle(credential, id);
        set_expiry(expiry);
    }
    if trace() {
        eprintln!("[sspi] AcquireCredentialsHandle {} protocols={protocols:#x} flags={flags:#x} certs={count} -> {id:#x}", if server { "server" } else { "client" });
    }
    SEC_E_OK
}
pub unsafe extern "win64" fn free_credentials_handle(credential: *mut usize) -> i32 {
    let Some(id) = (unsafe { handle_id(credential) }) else {
        return SEC_E_INVALID_HANDLE;
    };
    if CREDENTIALS.lock().unwrap().get_or_insert_with(HashMap::new).remove(&id).is_none() {
        return SEC_E_INVALID_HANDLE;
    }
    SEC_E_OK
}

unsafe extern "C" fn accept_any_certificate(_ok: c_int, _store: P) -> c_int {
    1 // validated by the caller (manual credential validation)
}
fn ex_index() -> c_int {
    0 // index 0 is the application data slot of every SSL object
}
unsafe extern "C" fn psk_client(ssl: P, _hint: *const c_char, identity: *mut c_char, max_identity: c_uint, psk: *mut u8, max_psk: c_uint) -> c_uint {
    let context = unsafe { &*(SSL_get_ex_data(ssl, ex_index()) as *const Context) };
    let Some((id, key)) = &context.psk else { return 0 };
    if id.len() >= max_identity as usize || key.len() > max_psk as usize {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(id.as_ptr(), identity.cast::<u8>(), id.len());
        *identity.add(id.len()) = 0;
        ptr::copy_nonoverlapping(key.as_ptr(), psk, key.len());
    }
    key.len() as c_uint
}
unsafe extern "C" fn psk_server(ssl: P, _identity: *const c_char, psk: *mut u8, max_psk: c_uint) -> c_uint {
    let context = unsafe { &*(SSL_get_ex_data(ssl, ex_index()) as *const Context) };
    let Some((_, key)) = &context.psk else { return 0 };
    if key.len() > max_psk as usize {
        return 0;
    }
    unsafe { ptr::copy_nonoverlapping(key.as_ptr(), psk, key.len()) };
    key.len() as c_uint
}

fn create_context(credential: Arc<Credential>, psk: Option<(Vec<u8>, Vec<u8>)>) -> Result<Context, i32> {
    unsafe {
        ERR_clear_error();
        let ctx = SSL_CTX_new(DTLS_method());
        if ctx.is_null() {
            return Err(SEC_E_INSUFFICIENT_MEMORY);
        }
        let min = if credential.protocols & SP_PROT_DTLS1_0 != 0 { DTLS1_VERSION } else { DTLS1_2_VERSION };
        let max = if credential.protocols & SP_PROT_DTLS1_2 != 0 { DTLS1_2_VERSION } else { DTLS1_VERSION };
        SSL_CTX_ctrl(ctx, SSL_CTRL_SET_MIN_PROTO_VERSION, min, ptr::null_mut());
        SSL_CTX_ctrl(ctx, SSL_CTRL_SET_MAX_PROTO_VERSION, max, ptr::null_mut());
        // Schannel accepts the DTLS 1.0 and SHA-1 combinations OpenSSL's default level refuses.
        SSL_CTX_set_security_level(ctx, 0);
        let ciphers = if credential.psk_only { c"PSK:!aNULL" } else { c"DEFAULT:!aNULL:@SECLEVEL=0" };
        SSL_CTX_set_cipher_list(ctx, ciphers.as_ptr());
        if let Some((der, key)) = &credential.certificate {
            let mut input = der.as_ptr();
            let x509 = d2i_X509(ptr::null_mut(), &mut input, der.len() as c_long);
            let mut input = key.as_ptr();
            let pkey = d2i_AutoPrivateKey(ptr::null_mut(), &mut input, key.len() as c_long);
            let ok = !x509.is_null() && !pkey.is_null() && SSL_CTX_use_certificate(ctx, x509) == 1 && SSL_CTX_use_PrivateKey(ctx, pkey) == 1;
            if !x509.is_null() {
                X509_free(x509);
            }
            if !pkey.is_null() {
                EVP_PKEY_free(pkey);
            }
            if !ok {
                SSL_CTX_free(ctx);
                return Err(SEC_E_UNKNOWN_CREDENTIALS);
            }
        }
        // Ask for (and accept) the peer certificate; the caller validates it.
        SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, Some(accept_any_certificate));
        let ssl = SSL_new(ctx);
        if ssl.is_null() {
            SSL_CTX_free(ctx);
            return Err(SEC_E_INSUFFICIENT_MEMORY);
        }
        let mut datagrams = Box::new(Datagrams { mtu: 1200, ..Default::default() });
        let bio = BIO_new(bio_method());
        BIO_set_data(bio, (&mut *datagrams as *mut Datagrams).cast());
        SSL_set_bio(ssl, bio, bio);
        SSL_ctrl(ssl, SSL_CTRL_OPTIONS, SSL_OP_NO_QUERY_MTU, ptr::null_mut());
        DTLS_set_timer_cb(ssl, Some(caller_driven_timer));
        if credential.server {
            SSL_set_accept_state(ssl);
            SSL_set_psk_server_callback(ssl, Some(psk_server));
        } else {
            SSL_set_connect_state(ssl);
            SSL_set_psk_client_callback(ssl, Some(psk_client));
        }
        Ok(Context { _credential: credential, ssl, ctx, datagrams, done: false, psk, peer_identity: None, last_flight: Vec::new() })
    }
}
/// Schannel leaves retransmission timing to its caller: a call without input while the
/// handshake waits sends the last flight again (`Context::last_flight`). OpenSSL's own
/// retransmission timer is pushed out of the way (50 minutes) so it never fires inside
/// a handshake call. A resent copy the peer already has is dropped by its replay
/// window; a lost one arrives now.
unsafe extern "C" fn caller_driven_timer(_ssl: P, _previous_us: c_uint) -> c_uint {
    3_000_000_000
}
fn set_mtu(context: &mut Context, mtu: usize) {
    let mtu = mtu.clamp(256, 1500);
    context.datagrams.mtu = mtu;
    unsafe {
        SSL_ctrl(context.ssl, DTLS_CTRL_SET_LINK_MTU, mtu as c_long, ptr::null_mut());
        SSL_ctrl(context.ssl, SSL_CTRL_SET_MTU, (mtu - 28) as c_long, ptr::null_mut());
    }
}

/// One handshake step for InitializeSecurityContextW/AcceptSecurityContext.
#[allow(clippy::too_many_arguments)]
unsafe fn handshake(
    credential: *const usize,
    context_handle: *const usize,
    requirements: u32,
    input: *const u8,
    new_context: *mut usize,
    output: *const u8,
    attributes: *mut u32,
    expiry: *mut i64,
    server: bool,
) -> i32 {
    if requirements & REQ_DATAGRAM == 0 {
        eprintln!("SspiCli: Schannel stream (TLS) contexts are not supported");
        return SEC_E_UNSUPPORTED_FUNCTION;
    }
    let inputs = unsafe { buffers(input) };
    let outputs = unsafe { buffers(output) };
    let token_in = inputs.iter().find(|b| kind(b) == SECBUFFER_TOKEN).map(bytes).unwrap_or_default();
    let psk_key = inputs.iter().find(|b| kind(b) == SECBUFFER_PRESHARED_KEY).map(bytes);
    let psk_identity = inputs.iter().find(|b| kind(b) == SECBUFFER_PRESHARED_KEY_IDENTITY).map(bytes);
    let mtu = inputs.iter().find(|b| kind(b) == SECBUFFER_DTLS_MTU && b.size >= 2 && !b.data.is_null())
        .map(|b| unsafe { (b.data as *const u16).read_unaligned() } as usize);
    let shared = match unsafe { handle_id(context_handle) } {
        Some(id) => match CONTEXTS.lock().unwrap().get_or_insert_with(HashMap::new).get(&id) {
            Some(c) => (id, c.clone()),
            None => return SEC_E_INVALID_HANDLE,
        },
        None => {
            let Some(cred_id) = (unsafe { handle_id(credential) }) else { return SEC_E_INVALID_HANDLE };
            let Some(cred) = CREDENTIALS.lock().unwrap().get_or_insert_with(HashMap::new).get(&cred_id).cloned() else {
                return SEC_E_INVALID_HANDLE;
            };
            if cred.server != server {
                return SEC_E_UNKNOWN_CREDENTIALS;
            }
            // A pre-shared key counts when it has non-zero bytes (an all-zero placeholder
            // does not); clients also need the identity they send.
            let psk = match (psk_key, psk_identity) {
                (Some(key), identity) if key.iter().any(|b| *b != 0) && (server || identity.is_some()) => {
                    Some((identity.unwrap_or_default(), key))
                }
                _ => None,
            };
            if cred.psk_only && psk.is_none() {
                eprintln!("SspiCli: PSK-only credentials without a usable pre-shared key are not supported");
                return SEC_E_NO_CREDENTIALS;
            }
            let mut context = match create_context(cred, psk) {
                Ok(c) => c,
                Err(code) => return code,
            };
            let id = NEXT.fetch_add(16, Ordering::Relaxed);
            set_mtu(&mut context, mtu.unwrap_or(1200));
            let shared = Arc::new(Mutex::new(context));
            CONTEXTS.lock().unwrap().get_or_insert_with(HashMap::new).insert(id, shared.clone());
            (id, shared)
        }
    };
    let (id, shared) = shared;
    let mut context = shared.lock().unwrap();
    // The PSK callbacks find the context (fixed inside the Arc) through ex_data.
    unsafe { SSL_set_ex_data(context.ssl, ex_index(), (&*context as *const Context) as P) };
    if let Some(mtu) = mtu {
        set_mtu(&mut context, mtu);
    }
    let token_out = outputs.iter().position(|b| kind(b) == SECBUFFER_TOKEN);
    let alert_out = outputs.iter().position(|b| kind(b) == SECBUFFER_ALERT);
    if let Some(index) = token_out {
        if requirements & REQ_ALLOCATE_MEMORY == 0 && outputs[index].size != 0 {
            set_mtu(&mut context, (outputs[index].size as usize).min(mtu.unwrap_or(1200)));
        }
    }
    let mut failed = None;
    let retransmit = token_in.is_empty() && context.datagrams.outgoing.is_empty() && !context.done && !context.last_flight.is_empty();
    if retransmit {
        let flight = context.last_flight.clone();
        context.datagrams.outgoing.extend(flight);
    } else if context.datagrams.outgoing.is_empty() || !token_in.is_empty() {
        if !token_in.is_empty() {
            context.datagrams.incoming.push_back(token_in.clone());
        }
        if !context.done {
            let r = unsafe { SSL_do_handshake(context.ssl) };
            if r == 1 {
                context.done = true;
            } else {
                let e = unsafe { SSL_get_error(context.ssl, r) };
                if e != SSL_ERROR_WANT_READ && e != SSL_ERROR_WANT_WRITE {
                    failed = Some(e);
                }
            }
        }
        context.datagrams.incoming.clear();
        if failed.is_none() && !context.datagrams.outgoing.is_empty() {
            context.last_flight = context.datagrams.outgoing.iter().cloned().collect();
        }
    }
    unsafe { set_handle(new_context, id) };
    unsafe { set_expiry(expiry) };
    if !attributes.is_null() {
        // REPLAY/SEQUENCE detection, confidentiality, datagram, extended error are provided.
        // MUTUAL_AUTH (ASC), USE_SUPPLIED_CREDS and MANUAL_CRED_VALIDATION (ISC) too.
        unsafe { *attributes = requirements & (0x2 | 0x4 | 0x8 | 0x10 | 0x80 | REQ_ALLOCATE_MEMORY | REQ_DATAGRAM | 0x4000 | 0x8000 | 0x8_0000) };
    }
    if context.done && server {
        if let Some(index) = outputs.iter().position(|b| kind(b) == SECBUFFER_PRESHARED_KEY_IDENTITY) {
            let identity = unsafe { SSL_get_psk_identity(context.ssl) };
            if !identity.is_null() {
                let text = unsafe { std::ffi::CStr::from_ptr(identity) }.to_bytes().to_vec();
                let b = &mut outputs[index];
                if !b.data.is_null() && b.size as usize >= text.len() {
                    unsafe { ptr::copy_nonoverlapping(text.as_ptr(), b.data, text.len()) };
                    b.size = text.len() as u32;
                }
                context.peer_identity = Some(text);
            }
        }
    }
    // Deliver the next outgoing datagram (an alert after a failure).
    let next = context.datagrams.outgoing.pop_front();
    let target = if failed.is_some() { alert_out.or(token_out) } else { token_out };
    let capacity = target.map_or(0, |i| outputs[i].size as usize);
    for b in outputs.iter_mut() {
        if matches!(kind(b), SECBUFFER_TOKEN | SECBUFFER_ALERT) {
            b.size = 0;
        }
    }
    if let (Some(datagram), Some(index)) = (next, target) {
        let b = &mut outputs[index];
        if requirements & REQ_ALLOCATE_MEMORY != 0 {
            let block = unsafe { malloc(datagram.len()) }.cast::<u8>();
            if block.is_null() {
                return SEC_E_INSUFFICIENT_MEMORY;
            }
            unsafe { ptr::copy_nonoverlapping(datagram.as_ptr(), block, datagram.len()) };
            ALLOCATED.lock().unwrap().get_or_insert_with(HashSet::new).insert(block as usize);
            b.data = block;
            b.size = datagram.len() as u32;
        } else if b.data.is_null() || capacity < datagram.len() {
            b.size = capacity as u32;
            context.datagrams.outgoing.push_front(datagram);
            return SEC_E_BUFFER_TOO_SMALL;
        } else {
            unsafe { ptr::copy_nonoverlapping(datagram.as_ptr(), b.data, datagram.len()) };
            b.size = datagram.len() as u32;
        }
    }
    let status = if let Some(e) = failed {
        eprintln!("SspiCli: DTLS handshake failed (OpenSSL error {e})");
        SEC_E_ILLEGAL_MESSAGE
    } else if !context.datagrams.outgoing.is_empty() {
        SEC_I_MESSAGE_FRAGMENT
    } else if context.done {
        SEC_E_OK
    } else {
        SEC_I_CONTINUE_NEEDED
    };
    if trace() {
        let out = token_out.map_or(0, |i| outputs[i].size);
        eprintln!("[sspi] {} ctx={id:#x} in={} out={out} -> {status:#x}", if server { "AcceptSecurityContext" } else { "InitializeSecurityContext" }, token_in.len());
    }
    status
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn initialize_security_context_w(
    credential: *const usize,
    context: *const usize,
    _target: *const u16,
    requirements: u32,
    _reserved1: u32,
    _representation: u32,
    input: *const u8,
    _reserved2: u32,
    new_context: *mut usize,
    output: *const u8,
    attributes: *mut u32,
    expiry: *mut i64,
) -> i32 {
    unsafe { handshake(credential, context, requirements, input, new_context, output, attributes, expiry, false) }
}
#[allow(clippy::too_many_arguments)]
pub unsafe extern "win64" fn accept_security_context(
    credential: *const usize,
    context: *const usize,
    input: *const u8,
    requirements: u32,
    _representation: u32,
    new_context: *mut usize,
    output: *const u8,
    attributes: *mut u32,
    expiry: *mut i64,
) -> i32 {
    unsafe { handshake(credential, context, requirements, input, new_context, output, attributes, expiry, true) }
}
pub unsafe extern "win64" fn delete_security_context(context: *mut usize) -> i32 {
    let Some(id) = (unsafe { handle_id(context) }) else {
        return SEC_E_INVALID_HANDLE;
    };
    if CONTEXTS.lock().unwrap().get_or_insert_with(HashMap::new).remove(&id).is_none() {
        return SEC_E_INVALID_HANDLE;
    }
    SEC_E_OK
}
pub unsafe extern "win64" fn free_context_buffer(buffer: *mut c_void) -> i32 {
    if buffer.is_null() {
        return SEC_E_OK;
    }
    if ALLOCATED.lock().unwrap().get_or_insert_with(HashSet::new).remove(&(buffer as usize)) {
        unsafe { free(buffer) };
        SEC_E_OK
    } else {
        SEC_E_INVALID_HANDLE
    }
}

fn context(handle: *const usize) -> Option<Arc<Mutex<Context>>> {
    let id = unsafe { handle_id(handle) }?;
    CONTEXTS.lock().unwrap().get_or_insert_with(HashMap::new).get(&id).cloned()
}
/// (record header incl. explicit nonce/IV, worst-case trailer, block size, max message)
fn sizes(context: &Context) -> Option<(usize, usize, usize, usize)> {
    if !context.done {
        return None;
    }
    let cipher = unsafe { SSL_get_current_cipher(context.ssl) };
    if cipher.is_null() {
        return None;
    }
    let nid = unsafe { SSL_CIPHER_get_cipher_nid(cipher) };
    let evp = unsafe { EVP_get_cipherbyname(OBJ_nid2sn(nid)) };
    let (explicit, block) = if evp.is_null() {
        (0, 1)
    } else {
        match unsafe { EVP_CIPHER_get_mode(evp) } {
            EVP_CIPH_GCM_MODE | EVP_CIPH_CCM_MODE => (8, 1),
            EVP_CIPH_CBC_MODE => {
                let b = unsafe { EVP_CIPHER_get_block_size(evp) }.max(1) as usize;
                (b, b)
            }
            _ => (0, 1),
        }
    };
    let header = DTLS_RECORD_HEADER + explicit;
    let data = unsafe { DTLS_get_data_mtu(context.ssl) };
    let record = context.datagrams.mtu.saturating_sub(28);
    let trailer = record.saturating_sub(header + data).max(16 + 48);
    Some((header, trailer, block, data))
}
pub unsafe extern "win64" fn query_context_attributes_w(handle: *const usize, attribute: u32, out: *mut u8) -> i32 {
    let Some(shared) = context(handle) else { return SEC_E_INVALID_HANDLE };
    let context = shared.lock().unwrap();
    if out.is_null() {
        return SEC_E_INTERNAL_ERROR;
    }
    match attribute {
        SECPKG_ATTR_STREAM_SIZES => {
            let Some((header, trailer, block, data)) = sizes(&context) else { return SEC_E_INVALID_HANDLE };
            // SecPkgContext_StreamSizes: cbHeader, cbTrailer, cbMaximumMessage, cBuffers, cbBlockSize.
            let values = [header as u32, trailer as u32, data as u32, 4, block as u32];
            unsafe { ptr::copy_nonoverlapping(values.as_ptr(), out.cast::<u32>(), 5) };
            SEC_E_OK
        }
        SECPKG_ATTR_REMOTE_CERT_CONTEXT => {
            let peer = unsafe { SSL_get1_peer_certificate(context.ssl) };
            if peer.is_null() {
                return SEC_E_NO_CREDENTIALS;
            }
            let length = unsafe { i2d_X509(peer, ptr::null_mut()) };
            let mut der = vec![0u8; length.max(0) as usize];
            let mut cursor = der.as_mut_ptr();
            let written = unsafe { i2d_X509(peer, &mut cursor) };
            unsafe { X509_free(peer) };
            if written <= 0 {
                return SEC_E_INTERNAL_ERROR;
            }
            let Some(certificate) = certificates::context_from_der(der) else { return SEC_E_INTERNAL_ERROR };
            unsafe { *(out as *mut *const c_void) = certificate };
            SEC_E_OK
        }
        _ => {
            eprintln!("SspiCli: QueryContextAttributesW attribute {attribute:#x} is not supported");
            SEC_E_UNSUPPORTED_FUNCTION
        }
    }
}

/// EncryptMessage: STREAM_HEADER + DATA + STREAM_TRAILER, one DTLS record. The DATA
/// buffer is encrypted in place; header and trailer receive the rest of the record.
pub unsafe extern "win64" fn encrypt_message(handle: *const usize, _qop: u32, message: *const u8, _sequence: u32) -> i32 {
    let Some(shared) = context(handle) else { return SEC_E_INVALID_HANDLE };
    let mut context = shared.lock().unwrap();
    let list = unsafe { buffers(message) };
    let (Some(h), Some(d), Some(t)) = (
        list.iter().position(|b| kind(b) == SECBUFFER_STREAM_HEADER),
        list.iter().position(|b| kind(b) == SECBUFFER_DATA),
        list.iter().position(|b| kind(b) == SECBUFFER_STREAM_TRAILER),
    ) else {
        return SEC_E_INVALID_TOKEN;
    };
    let Some((header, _, _, max)) = sizes(&context) else { return SEC_E_INVALID_HANDLE };
    let plain = bytes(&list[d]);
    if plain.len() > max {
        return SEC_E_BUFFER_TOO_SMALL;
    }
    context.datagrams.outgoing.clear();
    let n = unsafe { SSL_write(context.ssl, plain.as_ptr().cast(), plain.len() as c_int) };
    if n as usize != plain.len() {
        return SEC_E_INTERNAL_ERROR;
    }
    let Some(record) = context.datagrams.outgoing.pop_front() else { return SEC_E_INTERNAL_ERROR };
    context.datagrams.outgoing.clear();
    if record.len() < header + plain.len() {
        return SEC_E_INTERNAL_ERROR;
    }
    let tail = &record[header + plain.len()..];
    if (list[h].size as usize) < header || (list[t].size as usize) < tail.len() || list[h].data.is_null() || (list[t].data.is_null() && !tail.is_empty()) {
        return SEC_E_BUFFER_TOO_SMALL;
    }
    unsafe {
        ptr::copy_nonoverlapping(record.as_ptr(), list[h].data, header);
        if !plain.is_empty() {
            ptr::copy_nonoverlapping(record.as_ptr().add(header), list[d].data, plain.len());
        }
        if !tail.is_empty() {
            ptr::copy_nonoverlapping(tail.as_ptr(), list[t].data, tail.len());
        }
    }
    list[h].size = header as u32;
    list[t].size = tail.len() as u32;
    SEC_E_OK
}
/// DecryptMessage: the first SECBUFFER_DATA holds a received datagram. Its first
/// record is decrypted in place; the buffers become STREAM_HEADER, DATA (plaintext),
/// STREAM_TRAILER and, when more records follow, EXTRA.
pub unsafe extern "win64" fn decrypt_message(handle: *const usize, message: *const u8, _sequence: u32, qop: *mut u32) -> i32 {
    let Some(shared) = context(handle) else { return SEC_E_INVALID_HANDLE };
    let mut context = shared.lock().unwrap();
    let list = unsafe { buffers(message) };
    let Some(d) = list.iter().position(|b| kind(b) == SECBUFFER_DATA) else { return SEC_E_INVALID_TOKEN };
    if list.len() < 4 {
        return SEC_E_INVALID_TOKEN;
    }
    let base = list[d].data;
    let total = list[d].size as usize;
    if base.is_null() || total < DTLS_RECORD_HEADER {
        return SEC_E_INCOMPLETE_MESSAGE;
    }
    let datagram = unsafe { std::slice::from_raw_parts(base, total) };
    let record = DTLS_RECORD_HEADER + u16::from_be_bytes([datagram[11], datagram[12]]) as usize;
    if record > total {
        return SEC_E_INCOMPLETE_MESSAGE;
    }
    if !qop.is_null() {
        unsafe { *qop = 0 };
    }
    context.datagrams.incoming.clear();
    context.datagrams.incoming.push_back(datagram[..record].to_vec());
    let mut plain = vec![0u8; record];
    let n = unsafe { SSL_read(context.ssl, plain.as_mut_ptr().cast(), plain.len() as c_int) };
    context.datagrams.incoming.clear();
    // Handshake retransmissions OpenSSL produced here cannot be returned by DecryptMessage.
    context.datagrams.outgoing.clear();
    if n <= 0 {
        let e = unsafe { SSL_get_error(context.ssl, n) };
        if trace() {
            eprintln!("[sspi] DecryptMessage record of {record} bytes (type {}) not delivered: OpenSSL error {e}", datagram[0]);
        }
        return if e == SSL_ERROR_ZERO_RETURN { SEC_I_CONTEXT_EXPIRED } else { SEC_E_MESSAGE_ALTERED };
    }
    let n = n as usize;
    let header = sizes(&context).map_or(DTLS_RECORD_HEADER, |s| s.0).min(record - n);
    unsafe { ptr::copy(plain.as_ptr(), base.add(header), n) };
    let set = |b: &mut SecBuffer, kind: u32, data: *mut u8, size: usize| {
        b.kind = kind;
        b.data = data;
        b.size = size as u32;
    };
    let mut slots: Vec<usize> = (0..list.len()).collect();
    slots.retain(|&i| i == d || matches!(kind(&list[i]), SECBUFFER_EMPTY));
    if slots.len() < 4 {
        return SEC_E_INVALID_TOKEN;
    }
    set(&mut list[slots[0]], SECBUFFER_STREAM_HEADER, base, header);
    set(&mut list[slots[1]], SECBUFFER_DATA, unsafe { base.add(header) }, n);
    set(&mut list[slots[2]], SECBUFFER_STREAM_TRAILER, unsafe { base.add(header + n) }, record - header - n);
    if record < total {
        set(&mut list[slots[3]], SECBUFFER_EXTRA, unsafe { base.add(record) }, total - record);
    } else {
        set(&mut list[slots[3]], SECBUFFER_EMPTY, ptr::null_mut(), 0);
    }
    SEC_E_OK
}
