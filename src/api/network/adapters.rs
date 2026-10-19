//! IP Helper adapter enumeration from Linux interfaces. IP_ADAPTER_ADDRESSES_LH and
//! IP_ADAPTER_UNICAST_ADDRESS_LH use offsets checked against mingw-w64 iptypes.h for
//! x64 (448 and 64 bytes respectively).
use std::{ffi::{c_char, c_int, c_uint, CStr}, ptr};

const ERROR_INVALID_PARAMETER: u32 = 87;
const ERROR_BUFFER_OVERFLOW: u32 = 111;
const ERROR_NO_DATA: u32 = 232;
const AF_UNSPEC: u32 = 0;
const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
const GAA_FLAG_SKIP_UNICAST: u32 = 1;
const GAA_FLAG_SKIP_DNS_SERVER: u32 = 0x8;
const GAA_FLAG_SKIP_FRIENDLY_NAME: u32 = 0x20;
const ADAPTER_SIZE: usize = 448;
const UNICAST_SIZE: usize = 64;
const DNS_SERVER_SIZE: usize = 32;

pub fn dns_servers() -> Vec<[u8; 4]> {
    let mut servers: Vec<[u8; 4]> = Vec::new();
    for path in ["/run/systemd/resolve/resolv.conf", "/etc/resolv.conf"] {
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("nameserver") {
                    let ip_str = rest.trim();
                    let parts: Vec<u8> = ip_str
                        .split('.')
                        .filter_map(|p| p.parse::<u8>().ok())
                        .collect();
                    if parts.len() == 4 {
                        let ip = [parts[0], parts[1], parts[2], parts[3]];
                        if ip != [127, 0, 0, 53] && !servers.contains(&ip) {
                            servers.push(ip);
                        }
                    }
                }
            }
        }
    }
    for fallback in [[8, 8, 8, 8], [8, 8, 4, 4]] {
        if !servers.contains(&fallback) {
            servers.push(fallback);
        }
    }
    servers
}

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
struct Interface { name: String, ipv4: Vec<([u8; 4], u8)>, ipv6: Vec<([u8; 16], u8)>, mac: Vec<u8>, index: u32, up: bool, mtu: u32 }
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
                        out.push(Interface { name: name.clone(), ipv4: Vec::new(), ipv6: Vec::new(), mac, index, up: item.flags & 1 != 0, mtu });
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
                        out[idx].ipv4.push((addr, prefix));
                    } else if family == 10 {
                        // Linux sockaddr_in6: family, port, flowinfo, 16 address bytes.
                        let bytes = unsafe { std::slice::from_raw_parts(item.address.cast::<u8>().add(8), 16) };
                        let mut addr = [0; 16]; addr.copy_from_slice(bytes);
                        let prefix = if item.netmask.is_null() { 0 } else {
                            unsafe { std::slice::from_raw_parts(item.netmask.cast::<u8>().add(8), 16) }.iter().map(|b| b.count_ones() as u8).sum()
                        };
                        out[idx].ipv6.push((addr, prefix));
                    }
                }
            }
        }
        cursor = item.next;
    }
    unsafe { freeifaddrs(head) };
    out.retain(|entry| entry.index != 0 && (!entry.ipv4.is_empty() || !entry.ipv6.is_empty()));
    out
}
fn put32(out: *mut u8, offset: usize, value: u32) { unsafe { out.add(offset).cast::<u32>().write_unaligned(value) } }
fn put64(out: *mut u8, offset: usize, value: usize) { unsafe { out.add(offset).cast::<u64>().write_unaligned(value as u64) } }
fn align8(n: usize) -> usize { (n + 7) & !7 }

const IP_ADAPTER_INFO_SIZE: usize = 704;
/// Legacy IPv4 adapter list. The offsets follow mingw-w64 IP_ADAPTER_INFO on x64.
pub unsafe extern "win64" fn get_adapters_info(out: *mut u8, size: *mut u32) -> u32 {
    if size.is_null() { return ERROR_INVALID_PARAMETER; }
    let entries: Vec<_> = interfaces().into_iter().filter(|entry| !entry.ipv4.is_empty()).collect();
    if entries.is_empty() { return ERROR_NO_DATA; }
    let needed = entries.len() * IP_ADAPTER_INFO_SIZE;
    let supplied = unsafe { size.read_unaligned() } as usize;
    unsafe { size.write_unaligned(needed as u32) };
    if out.is_null() || supplied < needed { return ERROR_BUFFER_OVERFLOW; }
    unsafe { out.write_bytes(0, needed) };
    for (position, entry) in entries.iter().enumerate() {
        let record = unsafe { out.add(position * IP_ADAPTER_INFO_SIZE) };
        if position + 1 < entries.len() {
            put64(record, 0, unsafe { record.add(IP_ADAPTER_INFO_SIZE) } as usize);
        }
        let name = entry.name.as_bytes();
        let name_len = name.len().min(259);
        unsafe {
            ptr::copy_nonoverlapping(name.as_ptr(), record.add(12), name_len);
            ptr::copy_nonoverlapping(name.as_ptr(), record.add(272), name_len);
        }
        let mac_len = entry.mac.len().min(8);
        if mac_len > 0 {
            unsafe { ptr::copy_nonoverlapping(entry.mac.as_ptr(), record.add(408), mac_len) };
        }
        put32(record, 404, mac_len as u32);
        put32(record, 416, entry.index);
        put32(record, 420, if entry.mac.len() == 6 { 6 } else { 71 });
        let (ip, prefix) = entry.ipv4[0];
        let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix.min(32)) };
        let address = ip.map(|part| part.to_string()).join(".");
        let mask = std::net::Ipv4Addr::from(mask).to_string();
        put64(record, 432, unsafe { record.add(440) } as usize); // CurrentIpAddress
        unsafe {
            ptr::copy_nonoverlapping(address.as_ptr(), record.add(448), address.len());
            ptr::copy_nonoverlapping(mask.as_ptr(), record.add(464), mask.len());
        }
    }
    0
}

/// Ask the host routing table which source address would reach the destination,
/// then return that source adapter's real interface index. UDP connect sends no data.
pub unsafe extern "win64" fn get_best_interface(destination: u32, out: *mut u32) -> u32 {
    if out.is_null() { return ERROR_INVALID_PARAMETER; }
    let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(_) => return 1231, // ERROR_NETWORK_UNREACHABLE
    };
    let destination = std::net::Ipv4Addr::from(destination.to_ne_bytes());
    if socket.connect((destination, 9)).is_err() { return 1231; }
    let source = match socket.local_addr() {
        Ok(std::net::SocketAddr::V4(address)) => address.ip().octets(),
        _ => return 1231,
    };
    let Some(entry) = interfaces().into_iter().find(|entry| entry.ipv4.iter().any(|(ip, _)| *ip == source)) else {
        return 1231;
    };
    unsafe { out.write_unaligned(entry.index) };
    0
}

pub unsafe extern "win64" fn get_best_route2(
    interface_luid: *mut u64,
    mut interface_index: u32,
    _source_address: *const u8,
    destination_address: *const u8,
    _address_sort_options: u32,
    best_route: *mut u8,
    best_source_address: *mut u8,
) -> u32 {
    let entries = interfaces();
    if entries.is_empty() {
        return 1168; // ERROR_NOT_FOUND
    }

    let mut resolved_source: Option<[u8; 4]> = None;
    if !destination_address.is_null() {
        let family = unsafe { (destination_address as *const u16).read_unaligned() };
        if family == 2 {
            // AF_INET
            let dest_bytes = unsafe {
                let p = destination_address.add(4) as *const [u8; 4];
                *p
            };
            let dest_ip = std::net::Ipv4Addr::from(dest_bytes);
            if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
                if socket.connect((dest_ip, 9)).is_ok() {
                    if let Ok(std::net::SocketAddr::V4(addr)) = socket.local_addr() {
                        let src = addr.ip().octets();
                        if let Some(entry) = entries.iter().find(|e| e.ipv4.iter().any(|(ip, _)| *ip == src)) {
                            interface_index = entry.index;
                            resolved_source = Some(src);
                        }
                    }
                }
            }
        }
    }

    let matched_entry = entries
        .iter()
        .find(|e| interface_index != 0 && e.index == interface_index)
        .or_else(|| entries.iter().find(|e| !e.name.starts_with("lo")))
        .or_else(|| entries.first());

    let Some(entry) = matched_entry else {
        return 1168;
    };
    let if_index = entry.index;
    let src_ip = resolved_source.or_else(|| entry.ipv4.first().map(|(ip, _)| *ip)).unwrap_or([127, 0, 0, 1]);

    if !interface_luid.is_null() {
        unsafe { interface_luid.write_unaligned(if_index as u64) };
    }

    if !best_route.is_null() {
        unsafe {
            // MIB_IPFORWARD_ROW2 size is ~112 bytes
            best_route.write_bytes(0, 112);
            // InterfaceLuid at offset 0 (8 bytes)
            (best_route as *mut u64).write_unaligned(if_index as u64);
            // InterfaceIndex at offset 8 (4 bytes)
            (best_route.add(8) as *mut u32).write_unaligned(if_index);
            // Metric at offset 80 (4 bytes)
            (best_route.add(80) as *mut u32).write_unaligned(1);
        }
    }

    if !best_source_address.is_null() {
        unsafe {
            best_source_address.write_bytes(0, 28);
            (best_source_address as *mut u16).write_unaligned(2); // AF_INET
            std::ptr::copy_nonoverlapping(src_ip.as_ptr(), best_source_address.add(4), 4);
        }
    }

    0 // NO_ERROR
}

/// (AF_UNSPEC: IPv4 then IPv6) as unicast entries. Every address is reported usable:
/// DadState IpDadStatePreferred, infinite lifetimes, origins "other" (Linux does not
/// say whether an address came from DHCP or was set manually).
pub unsafe extern "win64" fn get_adapters_addresses(
    family: u32, flags: u32, reserved: usize, addresses: *mut u8, size: *mut u32,
) -> u32 {
    const IP_DAD_STATE_PREFERRED: u32 = 4;
    const INFINITE_LIFETIME: u32 = u32::MAX;
    if size.is_null() || reserved != 0 || !matches!(family, AF_UNSPEC | AF_INET | AF_INET6) {
        return ERROR_INVALID_PARAMETER;
    }
    let entries = interfaces();
    if entries.is_empty() { return ERROR_NO_DATA; }
    // (family, address bytes, prefix length) per adapter, in Windows order.
    type Addresses = Vec<(u16, Vec<u8>, u8)>;
    let selected: Vec<(Interface, Addresses)> = entries.into_iter().filter_map(|entry| {
        let mut list: Addresses = Vec::new();
        if family != AF_INET6 {
            list.extend(entry.ipv4.iter().map(|(a, p)| (2u16, a.to_vec(), *p)));
        }
        if family != AF_INET {
            list.extend(entry.ipv6.iter().map(|(a, p)| (23u16, a.to_vec(), *p)));
        }
        (!list.is_empty()).then_some((entry, list))
    }).collect();
    if selected.is_empty() { return ERROR_NO_DATA; }
    let unicast = flags & GAA_FLAG_SKIP_UNICAST == 0;
    let include_dns = flags & GAA_FLAG_SKIP_DNS_SERVER == 0 && family != AF_INET6;
    let dns_list = if include_dns { dns_servers() } else { Vec::new() };
    struct Layout {
        base: usize,
        unicast: Vec<(usize, usize)>,
        dns: Vec<(usize, usize)>,
        dns_suffix_at: usize,
        name_at: usize,
        friendly_at: Option<usize>,
        name: std::ffi::CString,
        friendly: Vec<u16>,
    }
    let mut layouts = Vec::new();
    let mut needed = 0usize;
    for (entry, list) in &selected {
        let name = std::ffi::CString::new(entry.name.as_str()).unwrap();
        let friendly: Vec<u16> = entry.name.encode_utf16().chain(std::iter::once(0)).collect();
        let base = align8(needed);
        let mut end = base + ADAPTER_SIZE;
        let mut records = Vec::new();
        if unicast {
            for (family, _, _) in list {
                let record = align8(end);
                let sockaddr = align8(record + UNICAST_SIZE);
                end = sockaddr + if *family == 2 { 16 } else { 28 };
                records.push((record, sockaddr));
            }
        }
        let mut dns_records = Vec::new();
        for _ in &dns_list {
            let record = align8(end);
            let sockaddr = align8(record + DNS_SERVER_SIZE);
            end = sockaddr + 16;
            dns_records.push((record, sockaddr));
        }
        let dns_suffix_at = align8(end);
        end = dns_suffix_at + 2; // L"" (single NUL u16)
        let name_at = end; end += name.as_bytes_with_nul().len();
        let friendly_at = if flags & GAA_FLAG_SKIP_FRIENDLY_NAME == 0 { let at = align8(end); end = at + friendly.len()*2; Some(at) } else { None };
        layouts.push(Layout { base, unicast: records, dns: dns_records, dns_suffix_at, name_at, friendly_at, name, friendly });
        needed = end;
    }
    let needed = align8(needed);
    if needed > u32::MAX as usize { return ERROR_BUFFER_OVERFLOW; }
    let supplied = unsafe { size.read_unaligned() } as usize;
    unsafe { size.write_unaligned(needed as u32) };
    if addresses.is_null() || supplied < needed { return ERROR_BUFFER_OVERFLOW; }
    unsafe { addresses.write_bytes(0, needed) };
    for (i, (entry, list)) in selected.iter().enumerate() {
        let layout = &layouts[i];
        let record = unsafe { addresses.add(layout.base) };
        put32(record, 0, ADAPTER_SIZE as u32);
        put32(record, 4, entry.index);
        if let Some(next) = layouts.get(i+1) { put64(record, 8, unsafe { addresses.add(next.base) } as usize); }
        put64(record, 16, unsafe { addresses.add(layout.name_at) } as usize);
        if let Some(&(first, _)) = layout.unicast.first() { put64(record, 24, unsafe { addresses.add(first) } as usize); }
        if let Some(&(first_dns, _)) = layout.dns.first() { put64(record, 48, unsafe { addresses.add(first_dns) } as usize); }
        put64(record, 56, unsafe { addresses.add(layout.dns_suffix_at) } as usize); // DnsSuffix (non-NULL L"")
        if let Some(friendly_at) = layout.friendly_at {
            let ptr = unsafe { addresses.add(friendly_at) };
            put64(record, 64, ptr as usize); // Description
            put64(record, 72, ptr as usize); // FriendlyName
            unsafe { ptr::copy_nonoverlapping(layout.friendly.as_ptr(), ptr.cast::<u16>(), layout.friendly.len()) };
        }
        unsafe { ptr::copy_nonoverlapping(layout.name.as_ptr().cast::<u8>(), addresses.add(layout.name_at), layout.name.as_bytes_with_nul().len()) };
        let mac_length = entry.mac.len().min(8);
        if mac_length > 0 { unsafe { ptr::copy_nonoverlapping(entry.mac.as_ptr(), record.add(80), mac_length) }; }
        put32(record, 88, mac_length as u32);
        let has_v4 = list.iter().any(|(f, _, _)| *f == 2);
        let has_v6 = list.iter().any(|(f, _, _)| *f == 23);
        put32(record, 92, ((has_v4 as u32) * 0x80) | ((has_v6 as u32) * 0x100)); // Ipv4Enabled / Ipv6Enabled
        put32(record, 96, entry.mtu);
        put32(record, 100, if entry.mac.len() == 6 { 6 } else { 71 }); // Ethernet or generic
        put32(record, 104, if entry.up { 1 } else { 2 });
        if has_v6 { put32(record, 108, entry.index); }
        for (j, ((family, addr, prefix), &(at, sockaddr_at))) in list.iter().zip(&layout.unicast).enumerate() {
            let unicast = unsafe { addresses.add(at) };
            let sockaddr = unsafe { addresses.add(sockaddr_at) };
            put32(unicast, 0, UNICAST_SIZE as u32);
            if let Some(&(next, _)) = layout.unicast.get(j + 1) { put64(unicast, 8, unsafe { addresses.add(next) } as usize); }
            put64(unicast, 16, sockaddr as usize);
            put32(unicast, 24, if *family == 2 { 16 } else { 28 });
            put32(unicast, 40, IP_DAD_STATE_PREFERRED);
            put32(unicast, 44, INFINITE_LIFETIME); // ValidLifetime
            put32(unicast, 48, INFINITE_LIFETIME); // PreferredLifetime
            put32(unicast, 52, INFINITE_LIFETIME); // LeaseLifetime
            unsafe { *unicast.add(56) = *prefix }; // OnLinkPrefixLength
            unsafe { sockaddr.cast::<u16>().write_unaligned(*family) };
            let addr_offset = if *family == 2 { 4 } else { 8 };
            unsafe { ptr::copy_nonoverlapping(addr.as_ptr(), sockaddr.add(addr_offset), addr.len()) };
            // Link-local IPv6 (fe80::/10) carries its interface as the scope ID.
            if *family == 23 && addr[0] == 0xfe && addr[1] & 0xc0 == 0x80 {
                put32(sockaddr, 24, entry.index);
            }
        }
        for (j, (dns_ip, &(at, sockaddr_at))) in dns_list.iter().zip(&layout.dns).enumerate() {
            let dns = unsafe { addresses.add(at) };
            let sockaddr = unsafe { addresses.add(sockaddr_at) };
            put32(dns, 0, DNS_SERVER_SIZE as u32);
            if let Some(&(next, _)) = layout.dns.get(j + 1) {
                put64(dns, 8, unsafe { addresses.add(next) } as usize);
            }
            put64(dns, 16, sockaddr as usize);
            put32(dns, 24, 16); // sizeof(sockaddr_in)
            unsafe {
                sockaddr.cast::<u16>().write_unaligned(2u16); // AF_INET
                ptr::copy_nonoverlapping(dns_ip.as_ptr(), sockaddr.add(4), 4);
            }
        }
    }
    0
}

const ERROR_IO_PENDING: u32 = 997;

pub unsafe extern "win64" fn notify_addr_change(
    handle: *mut usize,
    overlapped: usize,
) -> u32 {
    if !handle.is_null() {
        unsafe { *handle = 0x5555_0001 };
    }
    if overlapped != 0 {
        ERROR_IO_PENDING
    } else {
        0
    }
}

pub unsafe extern "win64" fn notify_route_change(
    handle: *mut usize,
    overlapped: usize,
) -> u32 {
    if !handle.is_null() {
        unsafe { *handle = 0x5555_0002 };
    }
    if overlapped != 0 {
        ERROR_IO_PENDING
    } else {
        0
    }
}

pub unsafe extern "win64" fn cancel_ip_change_notify(_overlapped: usize) -> i32 {
    1
}

#[cfg(test)]
mod legacy_tests {
    #[test]
    fn adapters_info_uses_host_ipv4_and_index() {
        let entries: Vec<_> = super::interfaces().into_iter().filter(|entry| !entry.ipv4.is_empty()).collect();
        if entries.is_empty() { return; }
        let mut needed = 0;
        assert_eq!(unsafe { super::get_adapters_info(std::ptr::null_mut(), &mut needed) }, 111);
        assert_eq!(needed as usize, entries.len() * super::IP_ADAPTER_INFO_SIZE);
        let mut buffer = vec![0u8; needed as usize];
        assert_eq!(unsafe { super::get_adapters_info(buffer.as_mut_ptr(), &mut needed) }, 0);
        for (position, entry) in entries.iter().enumerate() {
            let record = &buffer[position * super::IP_ADAPTER_INFO_SIZE..][..super::IP_ADAPTER_INFO_SIZE];
            assert_eq!(u32::from_ne_bytes(record[416..420].try_into().unwrap()), entry.index);
            let ip = entry.ipv4[0].0.map(|part| part.to_string()).join(".");
            assert_eq!(&record[448..448 + ip.len()], ip.as_bytes());
        }
    }
}
