//! Windows chain contexts backed by real OpenSSL chain/signature/time verification.
use super::{der::Certificate, fail, new_context, openssl, registry, release, Context};
use crate::host::paths;
use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr},
    sync::{Arc, Mutex, OnceLock},
};
#[repr(C)]
pub struct Element {
    size: u32,
    cert: *const Context,
    error: u32,
    info: u32,
    revocation: usize,
    issuance: usize,
    application: usize,
    extended: usize,
}
#[repr(C)]
pub struct Simple {
    size: u32,
    error: u32,
    info: u32,
    count: u32,
    elements: *const *const Element,
    trust_list: usize,
    has_freshness: i32,
    freshness: u32,
}
#[repr(C)]
pub struct Chain {
    size: u32,
    error: u32,
    info: u32,
    count: u32,
    chains: *const *const Simple,
    lower_count: u32,
    lower: usize,
    has_freshness: i32,
    freshness: u32,
    flags: u32,
    id: [u8; 16],
}
struct OwnedChain {
    chain: Box<Chain>,
    _simple: Box<Simple>,
    _chains: Box<[*const Simple]>,
    // Each Element is boxed for a stable address: `_pointers` holds `*const Element`
    // into these, so the Vec's own buffer must not be what they point at.
    #[allow(clippy::vec_box)]
    _elements: Vec<Box<Element>>,
    _pointers: Box<[*const Element]>,
    contexts: Vec<usize>,
    encoded: Vec<Vec<u8>>,
    time: Option<i64>,
    refs: u32,
    depth: usize,
}
unsafe impl Send for OwnedChain {}
static CHAINS: OnceLock<Mutex<HashMap<usize, OwnedChain>>> = OnceLock::new();
fn chains() -> &'static Mutex<HashMap<usize, OwnedChain>> {
    CHAINS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn trust(error: i32) -> u32 {
    match error {
        0 => 0,
        9 | 10 => 1,
        23 => 4,
        4 | 6 | 7 => 8,
        26 | 28 => 0x10,
        18 | 19 => 0x20,
        2 | 20 | 21 => 0x10000,
        24 => 0x400,
        34 => 0x8000000,
        _ => 0x20,
    }
}
unsafe fn purpose(parameters: *const u8) -> Result<(u32, Vec<String>), u32> {
    if parameters.is_null() {
        return Err(87);
    }
    let size = unsafe { parameters.cast::<u32>().read_unaligned() } as usize;
    if !(32..=96).contains(&size) {
        return Err(87);
    }
    // Extra issuance-policy, strong-signing and revocation-freshness controls
    // are not implemented; non-default settings must not be ignored.
    for (offset, width) in [
        (40usize, 4usize),
        (56, 4),
        (60, 4),
        (64, 4),
        (72, 8),
        (80, 8),
        (88, 4),
    ] {
        if size >= offset + width
            && unsafe { std::slice::from_raw_parts(parameters.add(offset), width) }
                .iter()
                .any(|b| *b != 0)
        {
            return Err(50);
        }
    }
    let match_type = unsafe { parameters.add(8).cast::<u32>().read_unaligned() };
    if match_type > 1 {
        return Err(87);
    }
    let count = unsafe { parameters.add(16).cast::<u32>().read_unaligned() };
    if count == 0 {
        return Ok((match_type, Vec::new()));
    }
    if count > 128 {
        return Err(50);
    }
    let list = unsafe {
        parameters
            .add(24)
            .cast::<*const *const c_char>()
            .read_unaligned()
    };
    if list.is_null() {
        return Err(87);
    }
    let mut oids = Vec::new();
    for index in 0..count as usize {
        let name = unsafe { *list.add(index) };
        if name.is_null() {
            return Err(87);
        }
        let oid = unsafe { CStr::from_ptr(name) }
            .to_str()
            .map_err(|_| 87u32)?;
        oids.push(oid.to_owned());
    }
    Ok((match_type, oids))
}

pub unsafe extern "win64" fn get(
    engine: usize,
    cert: *const Context,
    time: *const u64,
    additional: usize,
    parameters: *const u8,
    flags: u32,
    reserved: *const c_void,
    out: *mut *const Chain,
) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    unsafe {
        *out = std::ptr::null();
    }
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!(
            "crypt32: GetCertificateChain engine={engine}, flags={flags:#x}, parameter size={}",
            if parameters.is_null() {
                0
            } else {
                unsafe { parameters.cast::<u32>().read_unaligned() }
            }
        );
    }
    if engine > 1 || !reserved.is_null() || flags & !(0xf0000000 | 1 | 4 | 0x100 | 0x2000) != 0 {
        return fail(50, 0);
    }
    let usage = match unsafe { purpose(parameters) } {
        Ok(usage) => usage,
        Err(code) => return fail(code, 0),
    };
    let (leaf, untrusted) = {
        let r = registry().lock().unwrap();
        let Some(reference) = r.contexts.get(&(cert as usize)) else {
            return fail(87, 0);
        };
        let mut untrusted = Vec::new();
        let mut stores = reference.owner.iter().cloned().collect::<Vec<_>>();
        if additional != 0 {
            let Some(store) = r.stores.get(&additional) else {
                return fail(6, 0);
            };
            stores.push(store.clone());
        }
        for store in stores {
            for cert in store.certificates.lock().unwrap().iter() {
                untrusted.push(cert.encoded.to_vec());
            }
        }
        (reference.certificate.encoded.to_vec(), untrusted)
    };
    let time = if time.is_null() {
        None
    } else {
        Some((unsafe { time.read_unaligned() } / 10000000) as i64 - 11644473600)
    };
    let verified = match openssl::verify(&leaf, &untrusted, time, 0) {
        Ok(verified) => verified,
        Err(code) => return fail(code, 0),
    };
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!(
            "crypt32: chain flags={flags:#x}, requested usage={usage:?}, OpenSSL error={}, depth={}",
            verified.error, verified.depth
        );
    }
    let mut error = trust(verified.error);
    if flags & 0x70000000 != 0 {
        error |= 0x1000040;
    }
    let mut certificates = Vec::new();
    for der in &verified.chain {
        match Certificate::parse(der.clone()) {
            Ok(cert) => certificates.push(Arc::new(cert)),
            Err(()) => return fail(0x80092002, 0),
        }
    }
    if !usage.1.is_empty() && !usage.1.iter().any(|oid| oid == "2.5.29.37.0") {
        let mut allowed = vec![true; usage.1.len()];
        for certificate in &certificates {
            let usages = match certificate.usages() {
                Ok(value) => value,
                Err(()) => return fail(0x80092002, 0),
            };
            if let Some(oids) = usages {
                if !oids.iter().any(|oid| oid == "2.5.29.37.0") {
                    for (index, wanted) in usage.1.iter().enumerate() {
                        allowed[index] &= oids.contains(wanted);
                    }
                }
            }
        }
        let matches = if usage.0 == 0 {
            allowed.iter().all(|value| *value)
        } else {
            allowed.iter().any(|value| *value)
        };
        if !matches {
            error |= 0x10;
        }
    }
    let mut elements = Vec::new();
    let mut contexts = Vec::new();
    {
        let mut r = registry().lock().unwrap();
        for (index, certificate) in certificates.into_iter().enumerate() {
            let context = new_context(&mut r, 0, certificate);
            contexts.push(context as usize);
            elements.push(Box::new(Element {
                size: 56,
                cert: context,
                error: if index == verified.depth.min(verified.chain.len() - 1) {
                    error
                } else {
                    0
                },
                info: 0,
                revocation: 0,
                issuance: 0,
                application: 0,
                extended: 0,
            }));
        }
    }
    let pointers = elements
        .iter()
        .map(|e| &**e as *const Element)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let simple = Box::new(Simple {
        size: 40,
        error,
        info: 0,
        count: elements.len() as u32,
        elements: pointers.as_ptr(),
        trust_list: 0,
        has_freshness: 0,
        freshness: 0,
    });
    let list = vec![&*simple as *const Simple].into_boxed_slice();
    let chain = Box::new(Chain {
        size: 72,
        error,
        info: 0,
        count: 1,
        chains: list.as_ptr(),
        lower_count: 0,
        lower: 0,
        has_freshness: 0,
        freshness: 0,
        flags,
        id: [0; 16],
    });
    let pointer = &*chain as *const Chain;
    chains().lock().unwrap().insert(
        pointer as usize,
        OwnedChain {
            chain,
            _simple: simple,
            _chains: list,
            _elements: elements,
            _pointers: pointers,
            contexts,
            encoded: verified.chain,
            time,
            refs: 1,
            depth: verified.depth,
        },
    );
    unsafe {
        *out = pointer;
    }
    1
}
pub extern "win64" fn free(chain: *const Chain) {
    let removed = {
        let mut all = chains().lock().unwrap();
        let Some(owned) = all.get_mut(&(chain as usize)) else {
            return;
        };
        owned.refs -= 1;
        if owned.refs == 0 {
            all.remove(&(chain as usize))
        } else {
            None
        }
    };
    if let Some(owned) = removed {
        let mut r = registry().lock().unwrap();
        for context in owned.contexts {
            release(&mut r, context);
        }
    }
}
pub extern "win64" fn duplicate(chain: *const Chain) -> *const Chain {
    let mut all = chains().lock().unwrap();
    let Some(owned) = all.get_mut(&(chain as usize)) else {
        return fail(87, std::ptr::null());
    };
    owned.refs += 1;
    chain
}
fn policy_error(error: u32) -> u32 {
    if error & 8 != 0 {
        0x80096004
    } else if error & 4 != 0 {
        0x800b010c
    } else if error & 0x20 != 0 {
        0x800b0109
    } else if error & 0x10000 != 0 {
        0x800b010a
    } else if error & 1 != 0 {
        0x800b0101
    } else if error & 0x10 != 0 {
        0x800b0110
    } else if error & 0x1000040 != 0 {
        0x80092013
    } else if error != 0 {
        0x800b010b
    } else {
        0
    }
}
pub unsafe extern "win64" fn policy(
    kind: usize,
    chain: *const Chain,
    parameters: *const u8,
    status: *mut u8,
) -> i32 {
    if status.is_null() || unsafe { status.cast::<u32>().read_unaligned() } < 24 {
        return fail(87, 0);
    }
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!("crypt32: policy kind={kind}");
    }
    if kind != 1 && kind != 4 {
        return fail(50, 0);
    }
    let (mut error, mut depth, encoded, time) = {
        let all = chains().lock().unwrap();
        let Some(owned) = all.get(&(chain as usize)) else {
            return fail(87, 0);
        };
        (
            owned.chain.error,
            owned.depth,
            owned.encoded.clone(),
            owned.time,
        )
    };
    let extra = if parameters.is_null() {
        std::ptr::null()
    } else {
        if unsafe { parameters.cast::<u32>().read_unaligned() } < 16 {
            return fail(87, 0);
        }
        if unsafe { parameters.add(4).cast::<u32>().read_unaligned() } != 0 {
            return fail(50, 0);
        }
        unsafe { parameters.add(8).cast::<*const u8>().read_unaligned() }
    };
    if kind == 4 {
        if extra.is_null() || unsafe { extra.cast::<u32>().read_unaligned() } < 24 {
            return fail(87, 0);
        }
        if unsafe { extra.add(8).cast::<u32>().read_unaligned() } != 0 {
            return fail(50, 0);
        }
        let auth = unsafe { extra.add(4).cast::<u32>().read_unaligned() };
        if auth != 1 && auth != 2 {
            return fail(87, 0);
        }
        let verified = match openssl::verify(
            &encoded[0],
            &encoded[1..],
            time,
            if auth == 2 { 2 } else { 1 },
        ) {
            Ok(value) => value,
            Err(code) => return fail(code, 0),
        };
        error |= trust(verified.error);
        if verified.error != 0 {
            depth = verified.depth;
        }
        let server = unsafe { extra.add(16).cast::<*const u16>().read_unaligned() };
        if error == 0 && !server.is_null() {
            let name = match unsafe { paths::wide(server) } {
                Ok(name) => name,
                Err(code) => return fail(code, 0),
            };
            match openssl::matches_name(&encoded[0], &name) {
                Ok(true) => {}
                Ok(false) => {
                    unsafe {
                        status.add(4).cast::<u32>().write_unaligned(0x800b010f);
                        status.add(8).cast::<i32>().write_unaligned(0);
                        status.add(12).cast::<i32>().write_unaligned(0);
                    }
                    return 1;
                }
                Err(code) => return fail(code, 0),
            }
        }
    }
    let result = policy_error(error);
    if std::env::var_os("PEEKGAME_TRACE_CERTS").is_some() {
        eprintln!("crypt32: policy error={result:#x}, trust={error:#x}");
    }
    unsafe {
        status.add(4).cast::<u32>().write_unaligned(result);
        status
            .add(8)
            .cast::<i32>()
            .write_unaligned(if result == 0 { -1 } else { 0 });
        status
            .add(12)
            .cast::<i32>()
            .write_unaligned(if result == 0 { -1 } else { depth as i32 });
    }
    1
}
