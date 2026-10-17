//! BCrypt (CNG) primitives: RNG on Linux getrandom(2); SHA-1/2, MD5, HMAC and AES
//! (CBC, ECB, GCM) on OpenSSL libcrypto. Handles are process-local tokens; the
//! caller's hash/key object buffers are accepted but not used.
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
};

unsafe extern "C" {
    fn getrandom(buffer: *mut u8, length: usize, flags: u32) -> isize;
}

#[link(name = "crypto")]
unsafe extern "C" {
    fn EVP_MD_CTX_new() -> *mut c_void;
    fn EVP_MD_CTX_free(ctx: *mut c_void);
    fn EVP_MD_CTX_copy_ex(out: *mut c_void, in_: *const c_void) -> i32;
    fn EVP_DigestInit_ex(ctx: *mut c_void, type_: *const c_void, impl_: *mut c_void) -> i32;
    fn EVP_DigestUpdate(ctx: *mut c_void, d: *const c_void, cnt: usize) -> i32;
    fn EVP_DigestFinal_ex(ctx: *mut c_void, md: *mut u8, s: *mut u32) -> i32;
    fn EVP_sha256() -> *const c_void;
    fn EVP_sha384() -> *const c_void;
    fn EVP_sha512() -> *const c_void;
    fn EVP_sha1() -> *const c_void;
    fn EVP_md5() -> *const c_void;
    fn EVP_CIPHER_CTX_new() -> *mut c_void;
    fn EVP_CIPHER_CTX_free(ctx: *mut c_void);
    fn EVP_CIPHER_CTX_ctrl(ctx: *mut c_void, type_: i32, arg: i32, ptr: *mut c_void) -> i32;
    fn EVP_CIPHER_CTX_set_padding(ctx: *mut c_void, padding: i32) -> i32;
    fn EVP_CipherInit_ex(ctx: *mut c_void, cipher: *const c_void, impl_: *mut c_void, key: *const u8, iv: *const u8, enc: i32) -> i32;
    fn EVP_CipherUpdate(ctx: *mut c_void, out: *mut u8, outl: *mut i32, in_: *const u8, inl: i32) -> i32;
    fn EVP_CipherFinal_ex(ctx: *mut c_void, out: *mut u8, outl: *mut i32) -> i32;
    fn EVP_aes_128_gcm() -> *const c_void;
    fn EVP_aes_192_gcm() -> *const c_void;
    fn EVP_aes_256_gcm() -> *const c_void;
    fn EVP_aes_128_cbc() -> *const c_void;
    fn EVP_aes_192_cbc() -> *const c_void;
    fn EVP_aes_256_cbc() -> *const c_void;
    fn EVP_aes_128_ecb() -> *const c_void;
    fn EVP_aes_192_ecb() -> *const c_void;
    fn EVP_aes_256_ecb() -> *const c_void;
}

const INVALID_PARAMETER: i32 = 0xC000000Du32 as i32;
const INVALID_HANDLE: i32 = 0xC0000008u32 as i32;
const NOT_FOUND: i32 = 0xC0000225u32 as i32;
const NOT_SUPPORTED: i32 = 0xC00000BBu32 as i32;
const BUFFER_TOO_SMALL: i32 = 0xC0000023u32 as i32;
const INVALID_BUFFER_SIZE: i32 = 0xC0000206u32 as i32;
const AUTH_TAG_MISMATCH: i32 = 0xC000A002u32 as i32;
const INTERNAL_ERROR: i32 = 0xC00000E5u32 as i32;
const ALG_HANDLE_HMAC_FLAG: u32 = 0x8;
const BLOCK_PADDING: u32 = 0x1;
const AES_BLOCK: usize = 16;
static NEXT: AtomicUsize = AtomicUsize::new(0x5042_0000);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
    Sha384,
    Sha512,
    Sha1,
    Md5,
}

impl HashAlgo {
    fn digest_length(self) -> u32 {
        match self {
            HashAlgo::Sha256 => 32,
            HashAlgo::Sha384 => 48,
            HashAlgo::Sha512 => 64,
            HashAlgo::Sha1 => 20,
            HashAlgo::Md5 => 16,
        }
    }
    /// The HMAC block size.
    fn block_length(self) -> usize {
        match self {
            HashAlgo::Sha384 | HashAlgo::Sha512 => 128,
            _ => 64,
        }
    }
    fn evp_md(self) -> *const c_void {
        unsafe {
            match self {
                HashAlgo::Sha256 => EVP_sha256(),
                HashAlgo::Sha384 => EVP_sha384(),
                HashAlgo::Sha512 => EVP_sha512(),
                HashAlgo::Sha1 => EVP_sha1(),
                HashAlgo::Md5 => EVP_md5(),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Chaining {
    Cbc,
    Ecb,
    Gcm,
}

pub enum ProviderKind {
    Rng,
    Hash { algo: HashAlgo, hmac: bool },
    Aes(Chaining),
}

/// A symmetric key: its secret, chaining mode (copied from the provider when the
/// key is made, changeable per key) and the IV state used when a call passes no IV.
struct Key {
    secret: Vec<u8>,
    chaining: Chaining,
    iv: [u8; AES_BLOCK],
}

/// A running digest. HMAC keeps the inner context keyed with ipad and the opad
/// block for the outer pass; every finish re-keys it so the handle stays usable.
struct Hash {
    algo: HashAlgo,
    ctx: usize,
    ipad: Option<Vec<u8>>,
    opad: Option<Vec<u8>>,
}

impl Drop for Hash {
    fn drop(&mut self) {
        unsafe { EVP_MD_CTX_free(self.ctx as *mut c_void) };
    }
}

static PROVIDERS: LazyLock<Mutex<HashMap<usize, ProviderKind>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static HASHES: LazyLock<Mutex<HashMap<usize, Hash>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static KEYS: LazyLock<Mutex<HashMap<usize, Key>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn trace() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_CRYPTO").is_some())
}

unsafe fn wide(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let len = (0..32768).position(|i| unsafe { *ptr.add(i) } == 0)?;
    String::from_utf16(unsafe { std::slice::from_raw_parts(ptr, len) }).ok()
}

/// A property value given as a counted UTF-16 string (the count includes the NUL).
unsafe fn wide_value(input: *const u8, bytes: u32) -> Option<String> {
    if input.is_null() || bytes < 2 {
        return None;
    }
    let units = unsafe { std::slice::from_raw_parts(input as *const u16, bytes as usize / 2) };
    let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
    String::from_utf16(&units[..end]).ok()
}

fn store_u32(output: *mut u8, cb_output: u32, pcb_result: *mut u32, value: u32) -> i32 {
    unsafe { *pcb_result = 4 };
    if output.is_null() {
        return 0;
    }
    if cb_output < 4 {
        return BUFFER_TOO_SMALL;
    }
    unsafe { (output as *mut u32).write_unaligned(value) };
    0
}

/// RNG, AES, SHA-1/256/384/512 and MD5 (optionally as HMAC) from any provider name;
/// every other algorithm is STATUS_NOT_FOUND.
pub unsafe extern "win64" fn bcrypt_open_algorithm_provider(
    out: *mut usize,
    algorithm: *const u16,
    _implementation: *const u16,
    flags: u32,
) -> i32 {
    if out.is_null() {
        return INVALID_PARAMETER;
    }
    unsafe { *out = 0 };
    let Some(name) = (unsafe { wide(algorithm) }) else {
        return INVALID_PARAMETER;
    };
    if flags & !ALG_HANDLE_HMAC_FLAG != 0 {
        return INVALID_PARAMETER;
    }
    let hmac = flags & ALG_HANDLE_HMAC_FLAG != 0;
    let hash = |algo| ProviderKind::Hash { algo, hmac };
    let kind = match name.to_ascii_uppercase().as_str() {
        "RNG" if !hmac => ProviderKind::Rng,
        "AES" if !hmac => ProviderKind::Aes(Chaining::Cbc),
        "SHA256" => hash(HashAlgo::Sha256),
        "SHA384" => hash(HashAlgo::Sha384),
        "SHA512" => hash(HashAlgo::Sha512),
        "SHA1" => hash(HashAlgo::Sha1),
        "MD5" => hash(HashAlgo::Md5),
        _ => {
            if trace() {
                eprintln!("[crypto] BCryptOpenAlgorithmProvider '{name}' flags={flags:#x}: not found");
            }
            return NOT_FOUND;
        }
    };
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    PROVIDERS.lock().unwrap().insert(handle, kind);
    if trace() {
        eprintln!("[crypto] BCryptOpenAlgorithmProvider '{name}' flags={flags:#x} -> {handle:#x}");
    }
    unsafe { *out = handle };
    0
}

pub extern "win64" fn bcrypt_close_algorithm_provider(handle: usize, flags: u32) -> i32 {
    if flags != 0 {
        return INVALID_PARAMETER;
    }
    if PROVIDERS.lock().unwrap().remove(&handle).is_some() {
        0
    } else {
        INVALID_HANDLE
    }
}

fn chaining_name(chaining: Chaining) -> &'static str {
    match chaining {
        Chaining::Cbc => "ChainingModeCBC",
        Chaining::Ecb => "ChainingModeECB",
        Chaining::Gcm => "ChainingModeGCM",
    }
}

/// Provider, hash and key properties: lengths, chaining mode and GCM tag lengths.
pub unsafe extern "win64" fn bcrypt_get_property(
    object: usize,
    property: *const u16,
    output: *mut u8,
    cb_output: u32,
    pcb_result: *mut u32,
    _flags: u32,
) -> i32 {
    if pcb_result.is_null() {
        return INVALID_PARAMETER;
    }
    let Some(name) = (unsafe { wide(property) }) else {
        return INVALID_PARAMETER;
    };
    enum Object {
        Hash(HashAlgo),
        Aes(Chaining, Option<usize>),
        Rng,
    }
    let found = match PROVIDERS.lock().unwrap().get(&object) {
        Some(ProviderKind::Hash { algo, .. }) => Some(Object::Hash(*algo)),
        Some(ProviderKind::Aes(chaining)) => Some(Object::Aes(*chaining, None)),
        Some(ProviderKind::Rng) => Some(Object::Rng),
        None => None,
    };
    let found = found
        .or_else(|| HASHES.lock().unwrap().get(&object).map(|h| Object::Hash(h.algo)))
        .or_else(|| {
            KEYS.lock()
                .unwrap()
                .get(&object)
                .map(|k| Object::Aes(k.chaining, Some(k.secret.len())))
        });
    let Some(found) = found else {
        return INVALID_HANDLE;
    };
    match (name.as_str(), &found) {
        ("ObjectLength", Object::Hash(_) | Object::Aes(..)) => {
            store_u32(output, cb_output, pcb_result, 512)
        }
        ("HashDigestLength", Object::Hash(algo)) => {
            store_u32(output, cb_output, pcb_result, algo.digest_length())
        }
        ("HashBlockLength", Object::Hash(algo)) => {
            store_u32(output, cb_output, pcb_result, algo.block_length() as u32)
        }
        ("BlockLength", Object::Aes(..)) => store_u32(output, cb_output, pcb_result, 16),
        ("KeyLength", Object::Aes(_, Some(bytes))) => {
            store_u32(output, cb_output, pcb_result, *bytes as u32 * 8)
        }
        ("ChainingMode", Object::Aes(chaining, _)) => {
            let value: Vec<u16> = chaining_name(*chaining).encode_utf16().chain([0]).collect();
            let bytes = value.len() as u32 * 2;
            unsafe { *pcb_result = bytes };
            if output.is_null() {
                return 0;
            }
            if cb_output < bytes {
                return BUFFER_TOO_SMALL;
            }
            unsafe { std::ptr::copy_nonoverlapping(value.as_ptr() as *const u8, output, bytes as usize) };
            0
        }
        ("AuthTagLength", Object::Aes(Chaining::Gcm, _)) => {
            unsafe { *pcb_result = 12 };
            if output.is_null() {
                return 0;
            }
            if cb_output < 12 {
                return BUFFER_TOO_SMALL;
            }
            // BCRYPT_AUTH_TAG_LENGTHS_STRUCT: min, max, increment.
            for (i, v) in [12u32, 16, 1].into_iter().enumerate() {
                unsafe { (output as *mut u32).add(i).write_unaligned(v) };
            }
            0
        }
        _ => {
            if trace() {
                eprintln!("[crypto] BCryptGetProperty {object:#x} '{name}': not supported");
            }
            NOT_SUPPORTED
        }
    }
}

/// Only ChainingMode (CBC, ECB or GCM) on an AES provider or key.
pub unsafe extern "win64" fn bcrypt_set_property(
    object: usize,
    property: *const u16,
    input: *const u8,
    cb_input: u32,
    _flags: u32,
) -> i32 {
    let Some(name) = (unsafe { wide(property) }) else {
        return INVALID_PARAMETER;
    };
    if name != "ChainingMode" {
        if trace() {
            eprintln!("[crypto] BCryptSetProperty {object:#x} '{name}': not supported");
        }
        return NOT_SUPPORTED;
    }
    let chaining = match unsafe { wide_value(input, cb_input) }.as_deref() {
        Some("ChainingModeCBC") => Chaining::Cbc,
        Some("ChainingModeECB") => Chaining::Ecb,
        Some("ChainingModeGCM") => Chaining::Gcm,
        other => {
            if trace() {
                eprintln!("[crypto] BCryptSetProperty ChainingMode {other:?}: not supported");
            }
            return NOT_SUPPORTED;
        }
    };
    if let Some(ProviderKind::Aes(mode)) = PROVIDERS.lock().unwrap().get_mut(&object) {
        *mode = chaining;
        return 0;
    }
    if let Some(key) = KEYS.lock().unwrap().get_mut(&object) {
        key.chaining = chaining;
        return 0;
    }
    INVALID_HANDLE
}

#[repr(C)]
struct AuthModeInfo {
    cb_size: u32,
    dw_info_version: u32,
    pb_nonce: *const u8,
    cb_nonce: u32,
    pb_auth_data: *const u8,
    cb_auth_data: u32,
    pb_tag: *mut u8,
    cb_tag: u32,
    pb_mac_context: *mut u8,
    cb_mac_context: u32,
    cb_aad: u32,
    cb_data: u64,
    dw_flags: u32,
}

pub unsafe extern "win64" fn bcrypt_generate_symmetric_key(
    algorithm: usize,
    out_key: *mut usize,
    _key_object: *mut u8,
    _cb_key_object: u32,
    secret: *const u8,
    cb_secret: u32,
    flags: u32,
) -> i32 {
    if out_key.is_null() || secret.is_null() || flags != 0 {
        return INVALID_PARAMETER;
    }
    let chaining = match PROVIDERS.lock().unwrap().get(&algorithm) {
        Some(ProviderKind::Aes(chaining)) => *chaining,
        _ => return INVALID_HANDLE,
    };
    if ![16, 24, 32].contains(&cb_secret) {
        return INVALID_PARAMETER;
    }
    let secret = unsafe { std::slice::from_raw_parts(secret, cb_secret as usize) }.to_vec();
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    KEYS.lock().unwrap().insert(handle, Key { secret, chaining, iv: [0; AES_BLOCK] });
    unsafe { *out_key = handle };
    0
}

pub unsafe extern "win64" fn bcrypt_destroy_key(key: usize) -> i32 {
    if KEYS.lock().unwrap().remove(&key).is_some() {
        0
    } else {
        INVALID_HANDLE
    }
}

pub unsafe extern "win64" fn bcrypt_duplicate_key(
    key: usize,
    out_key: *mut usize,
    _key_object: *mut u8,
    _cb_key_object: u32,
    flags: u32,
) -> i32 {
    if out_key.is_null() || flags != 0 {
        return INVALID_PARAMETER;
    }
    let mut keys = KEYS.lock().unwrap();
    let Some(entry) = keys.get(&key) else {
        return INVALID_HANDLE;
    };
    let copy = Key { secret: entry.secret.clone(), chaining: entry.chaining, iv: entry.iv };
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    keys.insert(handle, copy);
    unsafe { *out_key = handle };
    0
}

/// One OpenSSL cipher pass; the context is freed on every path.
unsafe fn run_cipher(
    cipher: *const c_void,
    key: &[u8],
    iv: *const u8,
    padding: bool,
    encrypt: bool,
    input: &[u8],
    output: *mut u8,
) -> Result<usize, i32> {
    let ctx = unsafe { EVP_CIPHER_CTX_new() };
    if ctx.is_null() {
        return Err(INTERNAL_ERROR);
    }
    let result = (|| unsafe {
        if EVP_CipherInit_ex(ctx, cipher, std::ptr::null_mut(), key.as_ptr(), iv, encrypt as i32) != 1 {
            return Err(INTERNAL_ERROR);
        }
        EVP_CIPHER_CTX_set_padding(ctx, padding as i32);
        let (mut n, mut tail) = (0i32, 0i32);
        if !input.is_empty()
            && EVP_CipherUpdate(ctx, output, &mut n, input.as_ptr(), input.len() as i32) != 1
        {
            return Err(INTERNAL_ERROR);
        }
        if EVP_CipherFinal_ex(ctx, output.add(n as usize), &mut tail) != 1 {
            // Bad PKCS#7 padding on decryption.
            return Err(INVALID_PARAMETER);
        }
        Ok((n + tail) as usize)
    })();
    unsafe { EVP_CIPHER_CTX_free(ctx) };
    result
}

unsafe fn gcm(
    secret: &[u8],
    info: &AuthModeInfo,
    encrypt: bool,
    input: &[u8],
    output: *mut u8,
) -> Result<usize, i32> {
    if info.dw_flags != 0 {
        return Err(NOT_SUPPORTED); // chained GCM calls
    }
    if info.pb_nonce.is_null() || info.cb_nonce == 0 || info.pb_tag.is_null() {
        return Err(INVALID_PARAMETER);
    }
    if !(12..=16).contains(&info.cb_tag) {
        return Err(INVALID_PARAMETER);
    }
    let cipher = unsafe {
        match secret.len() {
            16 => EVP_aes_128_gcm(),
            24 => EVP_aes_192_gcm(),
            _ => EVP_aes_256_gcm(),
        }
    };
    let ctx = unsafe { EVP_CIPHER_CTX_new() };
    if ctx.is_null() {
        return Err(INTERNAL_ERROR);
    }
    let result = (|| unsafe {
        let null = std::ptr::null_mut();
        if EVP_CipherInit_ex(ctx, cipher, null, std::ptr::null(), std::ptr::null(), encrypt as i32) != 1
            || EVP_CIPHER_CTX_ctrl(ctx, 0x9, info.cb_nonce as i32, null) != 1 // SET_IVLEN
            || EVP_CipherInit_ex(ctx, std::ptr::null(), null, secret.as_ptr(), info.pb_nonce, encrypt as i32) != 1
        {
            return Err(INTERNAL_ERROR);
        }
        let mut n = 0i32;
        if !info.pb_auth_data.is_null()
            && info.cb_auth_data > 0
            && EVP_CipherUpdate(ctx, null.cast(), &mut n, info.pb_auth_data, info.cb_auth_data as i32) != 1
        {
            return Err(INTERNAL_ERROR);
        }
        n = 0;
        if !input.is_empty()
            && EVP_CipherUpdate(ctx, output, &mut n, input.as_ptr(), input.len() as i32) != 1
        {
            return Err(INTERNAL_ERROR);
        }
        if !encrypt && EVP_CIPHER_CTX_ctrl(ctx, 0x11, info.cb_tag as i32, info.pb_tag.cast()) != 1 {
            return Err(INTERNAL_ERROR); // SET_TAG
        }
        let mut tail = 0i32;
        if EVP_CipherFinal_ex(ctx, output.add(n as usize), &mut tail) != 1 {
            return Err(if encrypt { INTERNAL_ERROR } else { AUTH_TAG_MISMATCH });
        }
        if encrypt && EVP_CIPHER_CTX_ctrl(ctx, 0x10, info.cb_tag as i32, info.pb_tag.cast()) != 1 {
            return Err(INTERNAL_ERROR); // GET_TAG
        }
        Ok((n + tail) as usize)
    })();
    unsafe { EVP_CIPHER_CTX_free(ctx) };
    result
}

/// BCryptEncrypt/BCryptDecrypt. CBC and ECB pad only with BCRYPT_BLOCK_PADDING and
/// otherwise need whole blocks; CBC writes the last ciphertext block back to the IV
/// (or to the key's own IV when none is given), as CNG does for chained calls.
#[allow(clippy::too_many_arguments)]
unsafe fn crypt(
    encrypt: bool,
    key: usize,
    input: *const u8,
    cb_input: u32,
    padding_info: *const c_void,
    iv: *mut u8,
    cb_iv: u32,
    output: *mut u8,
    cb_output: u32,
    pcb_result: *mut u32,
    flags: u32,
) -> i32 {
    if pcb_result.is_null() || (input.is_null() && cb_input != 0) || flags & !BLOCK_PADDING != 0 {
        return INVALID_PARAMETER;
    }
    let mut keys = KEYS.lock().unwrap();
    let Some(entry) = keys.get_mut(&key) else {
        return INVALID_HANDLE;
    };
    let data = if cb_input == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(input, cb_input as usize) }
    };
    let padding = flags & BLOCK_PADDING != 0;
    let len = cb_input as usize;
    let result = if entry.chaining == Chaining::Gcm {
        if padding_info.is_null() || padding {
            return INVALID_PARAMETER;
        }
        let info = unsafe { &*(padding_info as *const AuthModeInfo) };
        unsafe { *pcb_result = cb_input };
        if output.is_null() {
            return 0;
        }
        if cb_output < cb_input {
            return BUFFER_TOO_SMALL;
        }
        unsafe { gcm(&entry.secret, info, encrypt, data, output) }
    } else {
        if !padding_info.is_null() {
            return INVALID_PARAMETER;
        }
        let needed = if encrypt && padding { (len / AES_BLOCK + 1) * AES_BLOCK } else { len };
        if !padding || encrypt {
            if !padding && !len.is_multiple_of(AES_BLOCK) {
                return INVALID_BUFFER_SIZE;
            }
        } else if !len.is_multiple_of(AES_BLOCK) || len == 0 {
            return INVALID_BUFFER_SIZE;
        }
        unsafe { *pcb_result = needed as u32 };
        if output.is_null() {
            return 0;
        }
        if (cb_output as usize) < needed {
            return BUFFER_TOO_SMALL;
        }
        let cbc = entry.chaining == Chaining::Cbc;
        if cbc && !iv.is_null() && cb_iv as usize != AES_BLOCK {
            return INVALID_PARAMETER;
        }
        let iv_block: [u8; AES_BLOCK] = if !cbc {
            [0; AES_BLOCK]
        } else if iv.is_null() {
            entry.iv
        } else {
            unsafe { std::ptr::read_unaligned(iv as *const [u8; AES_BLOCK]) }
        };
        // Decryption chains on the last input block: save it before an in-place call.
        let last_input: Option<[u8; AES_BLOCK]> =
            (len >= AES_BLOCK).then(|| data[len - AES_BLOCK..].try_into().unwrap());
        let cipher = unsafe {
            match (entry.secret.len(), cbc) {
                (16, true) => EVP_aes_128_cbc(),
                (24, true) => EVP_aes_192_cbc(),
                (_, true) => EVP_aes_256_cbc(),
                (16, false) => EVP_aes_128_ecb(),
                (24, false) => EVP_aes_192_ecb(),
                (_, false) => EVP_aes_256_ecb(),
            }
        };
        let iv_ptr = if cbc { iv_block.as_ptr() } else { std::ptr::null() };
        let done = if padding && !encrypt {
            // OpenSSL may write a block past the plaintext while it strips padding.
            let mut plain = vec![0u8; len + AES_BLOCK];
            let done = unsafe { run_cipher(cipher, &entry.secret, iv_ptr, true, false, data, plain.as_mut_ptr()) };
            if let Ok(n) = done {
                unsafe { std::ptr::copy_nonoverlapping(plain.as_ptr(), output, n) };
            }
            done
        } else {
            unsafe { run_cipher(cipher, &entry.secret, iv_ptr, padding, encrypt, data, output) }
        };
        if let (Ok(n), true) = (&done, cbc) {
            let next = if encrypt {
                (*n >= AES_BLOCK).then(|| unsafe {
                    std::ptr::read_unaligned(output.add(n - AES_BLOCK) as *const [u8; AES_BLOCK])
                })
            } else {
                last_input
            };
            if let Some(next) = next {
                if iv.is_null() {
                    entry.iv = next;
                } else {
                    unsafe { std::ptr::write_unaligned(iv as *mut [u8; AES_BLOCK], next) };
                }
            }
        }
        done
    };
    if trace() {
        eprintln!(
            "[crypto] {} key={key:#x} n={cb_input:#x} flags={flags:#x} -> {result:?}",
            if encrypt { "BCryptEncrypt" } else { "BCryptDecrypt" }
        );
    }
    match result {
        Ok(n) => {
            unsafe { *pcb_result = n as u32 };
            0
        }
        Err(status) => status,
    }
}

pub unsafe extern "win64" fn bcrypt_decrypt(
    key: usize,
    input: *const u8,
    cb_input: u32,
    padding_info: *const c_void,
    iv: *mut u8,
    cb_iv: u32,
    output: *mut u8,
    cb_output: u32,
    pcb_result: *mut u32,
    flags: u32,
) -> i32 {
    unsafe { crypt(false, key, input, cb_input, padding_info, iv, cb_iv, output, cb_output, pcb_result, flags) }
}

pub unsafe extern "win64" fn bcrypt_encrypt(
    key: usize,
    input: *const u8,
    cb_input: u32,
    padding_info: *const c_void,
    iv: *mut u8,
    cb_iv: u32,
    output: *mut u8,
    cb_output: u32,
    pcb_result: *mut u32,
    flags: u32,
) -> i32 {
    unsafe { crypt(true, key, input, cb_input, padding_info, iv, cb_iv, output, cb_output, pcb_result, flags) }
}

/// Starts (or restarts) a digest, keyed with ipad for HMAC.
fn hash_start(hash: &Hash) -> bool {
    let ctx = hash.ctx as *mut c_void;
    unsafe {
        EVP_DigestInit_ex(ctx, hash.algo.evp_md(), std::ptr::null_mut()) == 1
            && hash
                .ipad
                .as_ref()
                .is_none_or(|pad| EVP_DigestUpdate(ctx, pad.as_ptr().cast(), pad.len()) == 1)
    }
}

/// One-shot digest of `parts`, used for long HMAC keys and the outer HMAC pass.
fn digest(algo: HashAlgo, parts: &[&[u8]], out: &mut [u8]) -> bool {
    let ctx = unsafe { EVP_MD_CTX_new() };
    if ctx.is_null() {
        return false;
    }
    let mut n = 0u32;
    let ok = unsafe {
        EVP_DigestInit_ex(ctx, algo.evp_md(), std::ptr::null_mut()) == 1
            && parts.iter().all(|p| EVP_DigestUpdate(ctx, p.as_ptr().cast(), p.len()) == 1)
            && EVP_DigestFinal_ex(ctx, out.as_mut_ptr(), &mut n) == 1
    };
    unsafe { EVP_MD_CTX_free(ctx) };
    ok
}

/// A hash, or an HMAC when the provider was opened with BCRYPT_ALG_HANDLE_HMAC_FLAG
/// (the secret is the HMAC key). Flags other than BCRYPT_HASH_REUSABLE_FLAG are
/// rejected; handles are always reusable after BCryptFinishHash.
pub unsafe extern "win64" fn bcrypt_create_hash(
    algorithm: usize,
    out_hash: *mut usize,
    _hash_object: *mut u8,
    _cb_hash_object: u32,
    secret: *const u8,
    cb_secret: u32,
    flags: u32,
) -> i32 {
    if out_hash.is_null() || flags & !0x20 != 0 || (secret.is_null() && cb_secret != 0) {
        return INVALID_PARAMETER;
    }
    unsafe { *out_hash = 0 };
    let (algo, hmac) = match PROVIDERS.lock().unwrap().get(&algorithm) {
        Some(ProviderKind::Hash { algo, hmac }) => (*algo, *hmac),
        _ => return INVALID_HANDLE,
    };
    let (ipad, opad) = if hmac {
        let block = algo.block_length();
        let mut key = vec![0u8; block];
        let secret = if cb_secret == 0 {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(secret, cb_secret as usize) }
        };
        if secret.len() > block {
            if !digest(algo, &[secret], &mut key[..algo.digest_length() as usize]) {
                return INTERNAL_ERROR;
            }
        } else {
            key[..secret.len()].copy_from_slice(secret);
        }
        (
            Some(key.iter().map(|b| b ^ 0x36).collect()),
            Some(key.iter().map(|b| b ^ 0x5c).collect()),
        )
    } else {
        (None, None)
    };
    let ctx = unsafe { EVP_MD_CTX_new() };
    if ctx.is_null() {
        return INTERNAL_ERROR;
    }
    let hash = Hash { algo, ctx: ctx as usize, ipad, opad };
    if !hash_start(&hash) {
        return INTERNAL_ERROR;
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    HASHES.lock().unwrap().insert(handle, hash);
    unsafe { *out_hash = handle };
    0
}

pub unsafe extern "win64" fn bcrypt_hash_data(
    hash: usize,
    input: *const u8,
    cb_input: u32,
    flags: u32,
) -> i32 {
    if (input.is_null() && cb_input != 0) || flags != 0 {
        return INVALID_PARAMETER;
    }
    let hashes = HASHES.lock().unwrap();
    let Some(entry) = hashes.get(&hash) else {
        return INVALID_HANDLE;
    };
    if cb_input > 0
        && unsafe { EVP_DigestUpdate(entry.ctx as *mut c_void, input.cast(), cb_input as usize) } != 1
    {
        return INTERNAL_ERROR;
    }
    0
}

/// cbOutput must equal the digest length, as in CNG.
pub unsafe extern "win64" fn bcrypt_finish_hash(
    hash: usize,
    output: *mut u8,
    cb_output: u32,
    flags: u32,
) -> i32 {
    if output.is_null() || flags != 0 {
        return INVALID_PARAMETER;
    }
    let hashes = HASHES.lock().unwrap();
    let Some(entry) = hashes.get(&hash) else {
        return INVALID_HANDLE;
    };
    let size = entry.algo.digest_length();
    if cb_output != size {
        return INVALID_PARAMETER;
    }
    let mut inner = [0u8; 64];
    let mut n = 0u32;
    if unsafe { EVP_DigestFinal_ex(entry.ctx as *mut c_void, inner.as_mut_ptr(), &mut n) } != 1 {
        return INTERNAL_ERROR;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(output, size as usize) };
    match &entry.opad {
        Some(opad) => {
            if !digest(entry.algo, &[opad, &inner[..size as usize]], out) {
                return INTERNAL_ERROR;
            }
        }
        None => out.copy_from_slice(&inner[..size as usize]),
    }
    if !hash_start(entry) {
        return INTERNAL_ERROR;
    }
    0
}

pub unsafe extern "win64" fn bcrypt_destroy_hash(hash: usize) -> i32 {
    match HASHES.lock().unwrap().remove(&hash) {
        Some(_) => 0,
        None => INVALID_HANDLE,
    }
}

pub unsafe extern "win64" fn bcrypt_duplicate_hash(
    hash: usize,
    out_hash: *mut usize,
    _hash_object: *mut u8,
    _cb_hash_object: u32,
    flags: u32,
) -> i32 {
    if out_hash.is_null() || flags != 0 {
        return INVALID_PARAMETER;
    }
    unsafe { *out_hash = 0 };
    let mut hashes = HASHES.lock().unwrap();
    let Some(entry) = hashes.get(&hash) else {
        return INVALID_HANDLE;
    };
    let ctx = unsafe { EVP_MD_CTX_new() };
    if ctx.is_null() {
        return INTERNAL_ERROR;
    }
    let copy = Hash { algo: entry.algo, ctx: ctx as usize, ipad: entry.ipad.clone(), opad: entry.opad.clone() };
    if unsafe { EVP_MD_CTX_copy_ex(ctx, entry.ctx as *const c_void) } != 1 {
        return INTERNAL_ERROR;
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    hashes.insert(handle, copy);
    unsafe { *out_hash = handle };
    0
}

pub unsafe extern "win64" fn bcrypt_gen_random(
    algorithm: usize,
    buffer: *mut u8,
    length: u32,
    flags: u32,
) -> i32 {
    if buffer.is_null() && length != 0 {
        return INVALID_PARAMETER;
    }
    if algorithm == 0 {
        if flags != 2 {
            return INVALID_PARAMETER;
        }
    } else if flags & !1 != 0 {
        return INVALID_PARAMETER;
    } else if let Some(kind) = PROVIDERS.lock().unwrap().get(&algorithm) {
        if !matches!(kind, ProviderKind::Rng) {
            return INVALID_HANDLE;
        }
    } else {
        return INVALID_HANDLE;
    }
    let mut done = 0usize;
    while done < length as usize {
        let n = unsafe { getrandom(buffer.add(done), length as usize - done, 0) };
        if n < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(4) {
                continue;
            }
            return INTERNAL_ERROR;
        }
        if n == 0 {
            return INTERNAL_ERROR;
        }
        done += n as usize;
    }
    0
}

pub unsafe extern "win64" fn process_prng(buffer: *mut u8, length: usize) -> i32 {
    if length > u32::MAX as usize {
        return 0;
    }
    (unsafe { bcrypt_gen_random(0, buffer, length as u32, 2) } == 0) as i32
}

