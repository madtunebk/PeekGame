//! Names and addresses: getaddrinfo/GetAddrInfoW, getnameinfo, gethostbyname,
//! getprotobyname, getservbyname, inet_* conversions.
use super::{family_to_host, family_to_windows, started, Address};
use crate::api::{system::error, text};
use std::{
    cell::RefCell,
    ffi::{c_char, c_void, CStr, CString},
};

#[repr(C)]
struct HostAddrInfo {
    flags: i32,
    family: i32,
    socktype: i32,
    protocol: i32,
    addrlen: u32,
    addr: *mut c_void,
    canonname: *mut c_char,
    next: *mut HostAddrInfo,
}
unsafe extern "C" {
    fn getaddrinfo(
        node: *const c_char,
        service: *const c_char,
        hints: *const HostAddrInfo,
        result: *mut *mut HostAddrInfo,
    ) -> i32;
    fn freeaddrinfo(list: *mut HostAddrInfo);
    fn getnameinfo(
        address: *const c_void,
        length: u32,
        host: *mut c_char,
        host_length: u32,
        service: *mut c_char,
        service_length: u32,
        flags: i32,
    ) -> i32;
    fn inet_pton(family: i32, text: *const c_char, out: *mut c_void) -> i32;
    fn inet_ntop(family: i32, address: *const c_void, out: *mut c_char, size: u32)
        -> *const c_char;
    fn inet_aton(text: *const c_char, out: *mut u32) -> i32;
    fn getprotobyname(name: *const c_char) -> *const HostEntry;
    fn getprotobynumber(number: i32) -> *const HostEntry;
    fn getservbyname(name: *const c_char, protocol: *const c_char) -> *const HostService;
    fn getservbyport(port: i32, protocol: *const c_char) -> *const HostService;
}
// glibc's protoent shares HostEntry's first fields: name, aliases, then an int.
#[repr(C)]
struct HostEntry {
    name: *const c_char,
    aliases: *const *const c_char,
    number: i32,
}
#[repr(C)]
struct HostService {
    name: *const c_char,
    aliases: *const *const c_char,
    port: i32,
    protocol: *const c_char,
}

/// ADDRINFOA/ADDRINFOW (x64): flags, family, socktype, protocol, size_t addrlen @16,
/// canonname @24, addr @32, next @40. Allocated here, freed by freeaddrinfo.
#[repr(C)]
pub struct WinAddrInfo {
    flags: i32,
    family: i32,
    socktype: i32,
    protocol: i32,
    addrlen: usize,
    canonname: *mut c_void,
    addr: *mut c_void,
    next: *mut WinAddrInfo,
    // Private tail.
    storage: [u8; 128],
    wide: bool,
    magic: u32,
}
const MAGIC: u32 = 0x5041_4931;
fn flags_to_host(f: i32) -> i32 {
    let mut out = f & 7; // PASSIVE 1, CANONNAME 2, NUMERICHOST 4
    if f & 8 != 0 {
        out |= 0x400; // NUMERICSERV
    }
    if f & 0x100 != 0 {
        out |= 0x10; // ALL
    }
    if f & 0x400 != 0 {
        out |= 0x20; // ADDRCONFIG
    }
    if f & 0x800 != 0 {
        out |= 8; // V4MAPPED
    }
    out
}
fn eai_to_wsa(e: i32) -> u32 {
    match e {
        -3 => 11002,      // EAI_AGAIN: WSATRY_AGAIN
        -1 => 10022,      // EAI_BADFLAGS
        -4 => 11003,      // EAI_FAIL: WSANO_RECOVERY
        -6 => 10047,      // EAI_FAMILY
        -10 => 8,         // EAI_MEMORY
        -2 | -5 => 11001, // EAI_NONAME/NODATA: WSAHOST_NOT_FOUND
        -8 => 10109,      // EAI_SERVICE: WSATYPE_NOT_FOUND
        -7 => 10044,      // EAI_SOCKTYPE
        _ => 11003,
    }
}
fn resolve(
    node: Option<CString>,
    service: Option<CString>,
    hints: *const i32,
    wide: bool,
    result: *mut *mut WinAddrInfo,
) -> i32 {
    if result.is_null() {
        error::set(10014);
        return 10014;
    }
    unsafe { *result = std::ptr::null_mut() };
    let mut h: HostAddrInfo = unsafe { std::mem::zeroed() };
    if !hints.is_null() {
        let v = unsafe { std::slice::from_raw_parts(hints, 4) };
        h.flags = flags_to_host(v[0]);
        h.family = family_to_host(v[1]);
        h.socktype = v[2];
        h.protocol = v[3];
    }
    let mut list = std::ptr::null_mut();
    let rc = unsafe {
        getaddrinfo(
            node.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
            service.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            if hints.is_null() {
                std::ptr::null()
            } else {
                &h
            },
            &mut list,
        )
    };
    if rc != 0 {
        let e = eai_to_wsa(rc);
        error::set(e);
        return e as i32;
    }
    let mut first: *mut WinAddrInfo = std::ptr::null_mut();
    let mut tail: *mut *mut WinAddrInfo = &mut first;
    let mut a = list;
    while !a.is_null() {
        let host = unsafe { &*a };
        let mut node = Box::new(WinAddrInfo {
            flags: if hints.is_null() {
                0
            } else {
                unsafe { *hints }
            },
            family: family_to_windows(host.family),
            socktype: host.socktype,
            protocol: host.protocol,
            addrlen: host.addrlen as usize,
            canonname: std::ptr::null_mut(),
            addr: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
            storage: [0; 128],
            wide,
            magic: MAGIC,
        });
        let n = (host.addrlen as usize).min(128);
        unsafe {
            std::ptr::copy_nonoverlapping(host.addr.cast::<u8>(), node.storage.as_mut_ptr(), n)
        };
        node.storage[..2].copy_from_slice(&(node.family as u16).to_le_bytes());
        node.addr = node.storage.as_mut_ptr().cast();
        if !host.canonname.is_null() {
            let name = unsafe { CStr::from_ptr(host.canonname) };
            node.canonname = if wide {
                let mut w: Vec<u16> = name.to_string_lossy().encode_utf16().collect();
                w.push(0);
                Box::leak(w.into_boxed_slice()).as_mut_ptr().cast()
            } else {
                name.to_owned().into_raw().cast()
            };
        }
        let raw = Box::into_raw(node);
        unsafe {
            *tail = raw;
            tail = &mut (*raw).next;
        }
        a = host.next;
    }
    unsafe {
        freeaddrinfo(list);
        *result = first;
    }
    0
}
unsafe fn narrow(p: *const c_char) -> Option<CString> {
    (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_owned())
}
unsafe fn from_wide(p: *const u16) -> Option<CString> {
    unsafe { text::string(p) }.and_then(|s| CString::new(s).ok())
}
pub unsafe extern "win64" fn get_addr_info(
    node: *const c_char,
    service: *const c_char,
    hints: *const i32,
    result: *mut *mut WinAddrInfo,
) -> i32 {
    resolve(
        unsafe { narrow(node) },
        unsafe { narrow(service) },
        hints,
        false,
        result,
    )
}
pub unsafe extern "win64" fn get_addr_info_w(
    node: *const u16,
    service: *const u16,
    hints: *const i32,
    result: *mut *mut WinAddrInfo,
) -> i32 {
    resolve(
        unsafe { from_wide(node) },
        unsafe { from_wide(service) },
        hints,
        true,
        result,
    )
}
pub unsafe extern "win64" fn free_addr_info(mut list: *mut WinAddrInfo) {
    while !list.is_null() {
        if unsafe { (*list).magic } != MAGIC {
            eprintln!("freeaddrinfo: not a list from getaddrinfo");
            return;
        }
        let node = unsafe { Box::from_raw(list) };
        list = node.next;
        if !node.canonname.is_null() {
            if node.wide {
                let w = node.canonname.cast::<u16>();
                let n = unsafe { text::units(w) }.len() + 1;
                drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(w, n)) });
            } else {
                drop(unsafe { CString::from_raw(node.canonname.cast()) });
            }
        }
    }
}
/// NI_ flags: Windows NOFQDN 1, NUMERICHOST 2, NAMEREQD 4, NUMERICSERV 8, DGRAM 16.
fn ni_flags(f: i32) -> i32 {
    (if f & 1 != 0 { 4 } else { 0 })
        | (if f & 2 != 0 { 1 } else { 0 })
        | (if f & 4 != 0 { 8 } else { 0 })
        | (if f & 8 != 0 { 2 } else { 0 })
        | (f & 16)
}
pub unsafe extern "win64" fn get_name_info(
    address: *const c_void,
    length: i32,
    host: *mut c_char,
    host_length: u32,
    service: *mut c_char,
    service_length: u32,
    flags: i32,
) -> i32 {
    let Some(a) = (unsafe { Address::from_windows(address, length) }) else {
        return 10014;
    };
    let rc = unsafe {
        getnameinfo(
            a.as_ptr(),
            a.length,
            host,
            if host.is_null() { 0 } else { host_length },
            service,
            if service.is_null() { 0 } else { service_length },
            ni_flags(flags),
        )
    };
    if rc != 0 {
        let e = eai_to_wsa(rc);
        error::set(e);
        return e as i32;
    }
    0
}
pub unsafe extern "win64" fn get_name_info_w(
    address: *const c_void,
    length: i32,
    host: *mut u16,
    host_length: u32,
    service: *mut u16,
    service_length: u32,
    flags: i32,
) -> i32 {
    let mut h = [0 as c_char; 1025];
    let mut s = [0 as c_char; 32];
    let rc = unsafe {
        get_name_info(
            address,
            length,
            if host.is_null() {
                std::ptr::null_mut()
            } else {
                h.as_mut_ptr()
            },
            h.len() as u32,
            if service.is_null() {
                std::ptr::null_mut()
            } else {
                s.as_mut_ptr()
            },
            s.len() as u32,
            flags,
        )
    };
    if rc != 0 {
        return rc;
    }
    for (out, capacity, value) in [
        (host, host_length, &h[..]),
        (service, service_length, &s[..]),
    ] {
        if out.is_null() {
            continue;
        }
        let value = unsafe { CStr::from_ptr(value.as_ptr()) }.to_string_lossy();
        let wide: Vec<u16> = value.encode_utf16().collect();
        if capacity <= wide.len() as u32 {
            error::set(10014);
            return 10014;
        }
        unsafe { text::copy_out(&wide, out, capacity) };
    }
    0
}
pub unsafe extern "win64" fn inet_addr(text: *const c_char) -> u32 {
    let mut a = 0u32;
    if text.is_null() || unsafe { inet_aton(text, &mut a) } == 0 {
        return u32::MAX; // INADDR_NONE
    }
    a
}
thread_local! {static NTOA: RefCell<[u8; 16]> = const { RefCell::new([0; 16]) };}
/// inet_ntoa: a per-thread buffer, as on Windows.
pub extern "win64" fn inet_ntoa(address: u32) -> *const c_char {
    let b = address.to_le_bytes();
    let text = format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
    NTOA.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.fill(0);
        buffer[..text.len()].copy_from_slice(text.as_bytes());
        buffer.as_ptr().cast()
    })
}
pub unsafe extern "win64" fn inet_pton_a(
    family: i32,
    text: *const c_char,
    out: *mut c_void,
) -> i32 {
    if text.is_null() || out.is_null() {
        error::set(10014);
        return -1;
    }
    let rc = unsafe { inet_pton(family_to_host(family), text, out) };
    if rc < 0 {
        error::set(10047);
        return -1;
    }
    rc
}
pub unsafe extern "win64" fn inet_pton_w(family: i32, text: *const u16, out: *mut c_void) -> i32 {
    let Some(narrow) = (unsafe { from_wide(text) }) else {
        error::set(10014);
        return -1;
    };
    unsafe { inet_pton_a(family, narrow.as_ptr(), out) }
}
pub unsafe extern "win64" fn inet_ntop_a(
    family: i32,
    address: *const c_void,
    out: *mut c_char,
    size: usize,
) -> *const c_char {
    if address.is_null() || out.is_null() {
        error::set(10014);
        return std::ptr::null();
    }
    let r = unsafe {
        inet_ntop(
            family_to_host(family),
            address,
            out,
            size.min(u32::MAX as usize) as u32,
        )
    };
    if r.is_null() {
        error::set(if super::errno() == 28 { 10022 } else { 10047 });
    }
    r
}
pub unsafe extern "win64" fn inet_ntop_w(
    family: i32,
    address: *const c_void,
    out: *mut u16,
    size: usize,
) -> *const u16 {
    let mut buffer = [0 as c_char; 64];
    if out.is_null()
        || unsafe { inet_ntop_a(family, address, buffer.as_mut_ptr(), buffer.len()) }.is_null()
    {
        if out.is_null() {
            error::set(10014);
        }
        return std::ptr::null();
    }
    let wide: Vec<u16> = unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .encode_utf16()
        .collect();
    if size <= wide.len() {
        error::set(10022);
        return std::ptr::null();
    }
    unsafe { text::copy_out(&wide, out, size as u32) };
    out
}

/// HOSTENT/PROTOENT/SERVENT with Windows layouts, rebuilt per thread (each call
/// overwrites the thread's previous result, as on Windows).
#[derive(Default)]
struct Owned {
    strings: Vec<CString>,
    pointers: Vec<Vec<*const c_char>>,
    addresses: Vec<Vec<u8>>,
    entry: Vec<u8>,
}
thread_local! {static RESULT: RefCell<Owned> = RefCell::new(Owned::default());}
fn list(owned: &mut Owned, items: Vec<*const c_char>) -> *const *const c_char {
    let mut items = items;
    items.push(std::ptr::null());
    owned.pointers.push(items);
    owned.pointers.last().unwrap().as_ptr()
}
unsafe fn aliases(owned: &mut Owned, mut p: *const *const c_char) -> *const *const c_char {
    let mut items = Vec::new();
    while !p.is_null() && unsafe { !(*p).is_null() } {
        owned.strings.push(unsafe { CStr::from_ptr(*p) }.to_owned());
        items.push(owned.strings.last().unwrap().as_ptr());
        p = unsafe { p.add(1) };
    }
    list(owned, items)
}
fn not_started<T>() -> Option<*const T> {
    if started() {
        return None;
    }
    error::set(10093);
    Some(std::ptr::null())
}
/// PROTOENT: char *name, char **aliases, short proto.
unsafe fn protocol_entry(entry: *const HostEntry) -> *const u8 {
    if entry.is_null() {
        error::set(11004); // WSANO_DATA
        return std::ptr::null();
    }
    RESULT.with(|r| {
        let mut owned = Owned::default();
        let e = unsafe { &*entry };
        owned
            .strings
            .push(unsafe { CStr::from_ptr(e.name) }.to_owned());
        let name = owned.strings.last().unwrap().as_ptr();
        let aliases = unsafe { aliases(&mut owned, e.aliases) };
        let mut bytes = vec![0u8; 24];
        bytes[..8].copy_from_slice(&(name as usize).to_le_bytes());
        bytes[8..16].copy_from_slice(&(aliases as usize).to_le_bytes());
        bytes[16..18].copy_from_slice(&(e.number as i16).to_le_bytes());
        owned.entry = bytes;
        *r.borrow_mut() = owned;
        r.borrow().entry.as_ptr()
    })
}
pub unsafe extern "win64" fn get_protocol_by_name(name: *const c_char) -> *const u8 {
    if let Some(p) = not_started() {
        return p;
    }
    if name.is_null() {
        error::set(10014);
        return std::ptr::null();
    }
    unsafe { protocol_entry(getprotobyname(name)) }
}
pub unsafe extern "win64" fn get_protocol_by_number(number: i32) -> *const u8 {
    if let Some(p) = not_started() {
        return p;
    }
    unsafe { protocol_entry(getprotobynumber(number)) }
}
/// SERVENT (x64): char *name, char **aliases, char *proto @16, short port @24.
unsafe fn service_entry(entry: *const HostService) -> *const u8 {
    if entry.is_null() {
        error::set(11004);
        return std::ptr::null();
    }
    RESULT.with(|r| {
        let mut owned = Owned::default();
        let e = unsafe { &*entry };
        owned
            .strings
            .push(unsafe { CStr::from_ptr(e.name) }.to_owned());
        let name = owned.strings.last().unwrap().as_ptr();
        owned
            .strings
            .push(unsafe { CStr::from_ptr(e.protocol) }.to_owned());
        let protocol = owned.strings.last().unwrap().as_ptr();
        let aliases = unsafe { aliases(&mut owned, e.aliases) };
        let mut bytes = vec![0u8; 32];
        bytes[..8].copy_from_slice(&(name as usize).to_le_bytes());
        bytes[8..16].copy_from_slice(&(aliases as usize).to_le_bytes());
        bytes[16..24].copy_from_slice(&(protocol as usize).to_le_bytes());
        bytes[24..26].copy_from_slice(&(e.port as u16).to_le_bytes());
        owned.entry = bytes;
        *r.borrow_mut() = owned;
        r.borrow().entry.as_ptr()
    })
}
pub unsafe extern "win64" fn get_service_by_name(
    name: *const c_char,
    protocol: *const c_char,
) -> *const u8 {
    if let Some(p) = not_started() {
        return p;
    }
    if name.is_null() {
        error::set(10014);
        return std::ptr::null();
    }
    unsafe { service_entry(getservbyname(name, protocol)) }
}
pub unsafe extern "win64" fn get_service_by_port(port: i32, protocol: *const c_char) -> *const u8 {
    if let Some(p) = not_started() {
        return p;
    }
    unsafe { service_entry(getservbyport(port, protocol)) }
}
/// gethostbyname through getaddrinfo (IPv4, as Windows returns). HOSTENT (x64):
/// char *h_name, char **h_aliases, short h_addrtype @16, short h_length @18,
/// char **h_addr_list @24.
pub unsafe extern "win64" fn get_host_by_name(name: *const c_char) -> *const u8 {
    if let Some(p) = not_started() {
        return p;
    }
    let host = if name.is_null() {
        let mut buffer = [0u8; 256];
        if unsafe { super::sockets::get_host_name(buffer.as_mut_ptr(), 256) } != 0 {
            return std::ptr::null();
        }
        unsafe { CStr::from_ptr(buffer.as_ptr().cast()) }.to_owned()
    } else {
        unsafe { CStr::from_ptr(name) }.to_owned()
    };
    let mut h: HostAddrInfo = unsafe { std::mem::zeroed() };
    h.family = 2;
    h.socktype = 1;
    h.flags = 2; // AI_CANONNAME
    let mut list = std::ptr::null_mut();
    let rc = unsafe { getaddrinfo(host.as_ptr(), std::ptr::null(), &h, &mut list) };
    if rc != 0 {
        error::set(eai_to_wsa(rc));
        return std::ptr::null();
    }
    RESULT.with(|r| {
        let mut owned = Owned::default();
        let mut addresses = Vec::new();
        let mut canonical = host.clone();
        let mut a = list;
        while !a.is_null() {
            let entry = unsafe { &*a };
            if !entry.canonname.is_null() {
                canonical = unsafe { CStr::from_ptr(entry.canonname) }.to_owned();
            }
            let bytes =
                unsafe { std::slice::from_raw_parts(entry.addr.cast::<u8>().add(4), 4) }.to_vec();
            if !addresses.contains(&bytes) {
                addresses.push(bytes);
            }
            a = entry.next;
        }
        unsafe { freeaddrinfo(list) };
        owned.strings.push(canonical);
        let name = owned.strings.last().unwrap().as_ptr();
        owned.addresses = addresses;
        let pointers: Vec<*const c_char> =
            owned.addresses.iter().map(|a| a.as_ptr().cast()).collect();
        let address_list = list_ptr(&mut owned, pointers);
        let alias_list = list_ptr(&mut owned, Vec::new());
        let mut bytes = vec![0u8; 32];
        bytes[..8].copy_from_slice(&(name as usize).to_le_bytes());
        bytes[8..16].copy_from_slice(&(alias_list as usize).to_le_bytes());
        bytes[16..18].copy_from_slice(&2i16.to_le_bytes());
        bytes[18..20].copy_from_slice(&4i16.to_le_bytes());
        bytes[24..32].copy_from_slice(&(address_list as usize).to_le_bytes());
        owned.entry = bytes;
        *r.borrow_mut() = owned;
        r.borrow().entry.as_ptr()
    })
}
fn list_ptr(owned: &mut Owned, items: Vec<*const c_char>) -> *const *const c_char {
    list(owned, items)
}
