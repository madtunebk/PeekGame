//! IP Helper adapter enumeration from Linux interfaces. The first Windows
//! IP_ADAPTER_ADDRESSES_LH and unicast record use offsets checked against
//! mingw-w64 iptypes.h for x64 (448 and 64 bytes respectively).
use std::{ffi::{c_char, c_int, c_uint, CStr}, ptr};

const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_BUFFER_OVERFLOW: u32 = 111;
const ERROR_NO_DATA: u32 = 232;
const AF_UNSPEC: u32 = 0;
const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
const GAA_FLAG_SKIP_UNICAST: u32 = 1;
const GAA_FLAG_SKIP_FRIENDLY_NAME: u32 = 0x20;
const ADAPTER_SIZE: usize = 448;
const UNICAST_SIZE: usize = 64;

#[repr(C)]
struct SockAddr { family: u16, data: [u8; 14] }
#[repr(C)]
struct IfAddr {
    next: *mut IfAddr,
    name: *const c_char,
    flags: c_uint,
    address: *const SockAddr,
    netmask: *const SockAddr,
    ifu: *const SockAddr,
    data: *mut u8,
}
unsafe extern "C" {
    fn getifaddrs(out: *mut *mut IfAddr) -> c_int;
    fn freeifaddrs(head: *mut IfAddr);
}
struct Interface { name: String, ipv4: Option<([u8; 4], u8)>, ipv6: Option<([u8; 16], u8)>, mac: Vec<u8>, index: u32, up: bool, mtu: u32 }
fn interfaces() -> Vec<Interface> {
    let mut head = ptr::null_mut();
    if unsafe { getifaddrs(&mut head) } != 0 { return Vec::new(); }
    let mut out: Vec<Interface> = Vec::new();
    let mut cursor = head;
    while !cursor.is_null() {
        let item = unsafe { &*cursor };
        if !item.name.is_null() {
            let name = unsafe { CStr::from_ptr(item.name) }.to_string_lossy().into_owned();
            if name != "lo" && !name.contains('/') {
                let idx = match out.iter().position(|entry| entry.name == name) {
                    Some(idx) => idx,
                    None => {
                        let sys = std::path::Path::new("/sys/class/net").join(&name);
                        let index = std::fs::read_to_string(sys.join("ifindex")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                        let mtu = std::fs::read_to_string(sys.join("mtu")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                        let mac = std::fs::read_to_string(sys.join("address")).ok().map(|s| s.trim().split(':').filter_map(|part| u8::from_str_radix(part, 16).ok()).collect()).unwrap_or_default();
                        out.push(Interface { name: name.clone(), ipv4: None, ipv6: None, mac, index, up: item.flags & 1 != 0, mtu });
                        out.len() - 1
                    }
                };
                if !item.address.is_null() {
                    let family = unsafe { (*item.address).family as u32 };
                    if family == 2 {
                        // Linux sockaddr_in: family, port, 4 address bytes.
                        let bytes = unsafe { std::slice::from_raw_parts(item.address.cast::<u8>().add(4), 4) };
                        let mut addr = [0; 4]; addr.copy_from_slice(bytes);
                        let prefix = if item.netmask.is_null() { 0 } else {
                            unsafe { std::slice::from_raw_parts(item.netmask.cast::<u8>().add(4), 4) }.iter().map(|b| b.count_ones() as u8).sum()
                        };
                        out[idx].ipv4 = Some((addr, prefix));
                    } else if family == 10 {
                        // Linux sockaddr_in6: family, port, flowinfo, 16 address bytes.
                        let bytes = unsafe { std::slice::from_raw_parts(item.address.cast::<u8>().add(8), 16) };
                        let mut addr = [0; 16]; addr.copy_from_slice(bytes);
                        let prefix = if item.netmask.is_null() { 0 } else {
                            unsafe { std::slice::from_raw_parts(item.netmask.cast::<u8>().add(8), 16) }.iter().map(|b| b.count_ones() as u8).sum()
                        };
                        out[idx].ipv6 = Some((addr, prefix));
                    }
                }
            }
        }
        cursor = item.next;
    }
    unsafe { freeifaddrs(head) };
    out.retain(|entry| entry.index != 0 && (entry.ipv4.is_some() || entry.ipv6.is_some()));
    out
}
fn put32(out: *mut u8, offset: usize, value: u32) { unsafe { out.add(offset).cast::<u32>().write_unaligned(value) } }
fn put64(out: *mut u8, offset: usize, value: usize) { unsafe { out.add(offset).cast::<u64>().write_unaligned(value as u64) } }
fn align8(n: usize) -> usize { (n + 7) & !7 }

pub unsafe extern "win64" fn get_adapters_addresses(
    family: u32, flags: u32, reserved: usize, addresses: *mut u8, size: *mut u32,
) -> u32 {
    if size.is_null() || reserved != 0 || !matches!(family, AF_UNSPEC | AF_INET | AF_INET6) {
        return ERROR_INVALID_PARAMETER;
    }
    let entries = interfaces();
    if entries.is_empty() { return ERROR_NO_DATA; }
    // One record per real host adapter, plus its first address of the requested family.
    let selected: Vec<_> = entries.into_iter().filter_map(|entry| {
        let address = match family {
            AF_INET => entry.ipv4.map(|(a, p)| (2u16, a.to_vec(), p)),
            AF_INET6 => entry.ipv6.map(|(a, p)| (23u16, a.to_vec(), p)),
            _ => entry.ipv4.map(|(a, p)| (2u16, a.to_vec(), p))
                .or_else(|| entry.ipv6.map(|(a, p)| (23u16, a.to_vec(), p))),
        };
        address.map(|address| (entry, address))
    }).collect();
    if selected.is_empty() { return ERROR_NO_DATA; }
    let mut layouts = Vec::new();
    let mut needed = 0usize;
    for (entry, (family, addr, _)) in &selected {
        let name = std::ffi::CString::new(entry.name.as_str()).unwrap();
        let friendly: Vec<u16> = entry.name.encode_utf16().chain(std::iter::once(0)).collect();
        let unicast = flags & GAA_FLAG_SKIP_UNICAST == 0;
        let base = align8(needed);
        let mut end = base + ADAPTER_SIZE;
        let uni = if unicast { let at = align8(end); end = at + UNICAST_SIZE; Some(at) } else { None };
        let socket = if unicast { let at = align8(end); end = at + if *family == 2 { 16 } else { 28 }; Some(at) } else { None };
        let name_at = end; end += name.as_bytes_with_nul().len();
        let friendly_at = if flags & GAA_FLAG_SKIP_FRIENDLY_NAME == 0 { let at = align8(end); end = at + friendly.len()*2; Some(at) } else { None };
        layouts.push((base, uni, socket, name_at, friendly_at, name, friendly, addr.len()));
        needed = end;
    }
    let needed = align8(needed);
    if needed > u32::MAX as usize { return ERROR_BUFFER_OVERFLOW; }
    let supplied = unsafe { size.read_unaligned() } as usize;
    unsafe { size.write_unaligned(needed as u32) };
    if addresses.is_null() || supplied < needed { return ERROR_BUFFER_OVERFLOW; }
    unsafe { addresses.write_bytes(0, needed) };
    for (i, (entry, (family, addr, prefix))) in selected.iter().enumerate() {
        let (base, uni, socket, name_at, friendly_at, name, friendly, _) = &layouts[i];
        let record = unsafe { addresses.add(*base) };
        put32(record, 0, ADAPTER_SIZE as u32);
        put32(record, 4, entry.index);
        if let Some(next) = layouts.get(i+1) { put64(record, 8, unsafe { addresses.add(next.0) } as usize); }
        put64(record, 16, unsafe { addresses.add(*name_at) } as usize);
        if let Some(uni) = uni { put64(record, 24, unsafe { addresses.add(*uni) } as usize); }
        if let Some(friendly_at) = friendly_at {
            let ptr = unsafe { addresses.add(*friendly_at) };
            put64(record, 64, ptr as usize); // Description
            put64(record, 72, ptr as usize); // FriendlyName
            unsafe { ptr::copy_nonoverlapping(friendly.as_ptr(), ptr.cast::<u16>(), friendly.len()) };
        }
        unsafe { ptr::copy_nonoverlapping(name.as_ptr().cast::<u8>(), addresses.add(*name_at), name.as_bytes_with_nul().len()) };
        let mac_length = entry.mac.len().min(8);
        if mac_length > 0 { unsafe { ptr::copy_nonoverlapping(entry.mac.as_ptr(), record.add(80), mac_length) }; }
        put32(record, 88, mac_length as u32);
        put32(record, 92, if *family == 2 { 0x80 } else { 0x100 });
        put32(record, 96, entry.mtu);
        put32(record, 100, if entry.mac.len() == 6 { 6 } else { 71 }); // Ethernet or generic
        put32(record, 104, if entry.up { 1 } else { 2 });
        if *family == 23 { put32(record, 108, entry.index); }
        if let (Some(uni), Some(socket)) = (uni, socket) {
            let unicast = unsafe { addresses.add(*uni) };
            let sockaddr = unsafe { addresses.add(*socket) };
            put32(unicast, 0, UNICAST_SIZE as u32);
            put64(unicast, 16, sockaddr as usize);
            put32(unicast, 24, if *family == 2 { 16 } else { 28 });
            put32(unicast, 56, *prefix as u32); // OnLinkPrefixLength
            unsafe { sockaddr.cast::<u16>().write_unaligned(*family) };
            let addr_offset = if *family == 2 { 4 } else { 8 };
            unsafe { ptr::copy_nonoverlapping(addr.as_ptr(), sockaddr.add(addr_offset), addr.len()) };
        }
    }
    0
}
