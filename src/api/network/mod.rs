//! Winsock (WS2_32/WSOCK32) on Linux sockets, ported from peek's winsock.inc and
//! wsa_events.inc. A SOCKET is the Linux descriptor; INVALID_SOCKET is !0 and
//! SOCKET_ERROR -1. The WSA error is the thread's last error (WSAGetLastError is
//! GetLastError on Windows). What differs from Linux is translated here: address
//! family numbers (AF_INET6 23 <-> 10), ADDRINFO layout, AI_/NI_ flags, option
//! numbers, SO_RCVTIMEO/SNDTIMEO in milliseconds, fd_set as count + SOCKET array,
//! 32-bit timeval, WSAEWOULDBLOCK for connects in progress, no SIGPIPE.
pub(crate) mod events;
pub(crate) mod certificates;
pub(crate) mod resolve;
pub(crate) mod sockets;
pub(crate) mod sspi;
pub(crate) mod winhttp;
use crate::api::system::error;
use std::ffi::c_void;

pub const INVALID_SOCKET: usize = !0;
pub const AF_INET6_WINDOWS: u16 = 23;
pub const AF_INET6: u16 = 10;
const SOCKADDR_STORAGE: usize = 128;

#[allow(non_camel_case_types)]
type socklen_t = u32;
unsafe extern "C" {
    fn __errno_location() -> *mut i32;
}
pub(crate) fn errno() -> i32 {
    unsafe { *__errno_location() }
}
pub(crate) fn set_errno(value: i32) {
    unsafe { *__errno_location() = value };
}
/// The WSA error for a Linux errno.
pub(crate) fn wsa_error(e: i32) -> u32 {
    match e {
        11 | 115 => 10035, // WSAEWOULDBLOCK (also a connect in progress)
        4 => 10004,
        9 | 88 => 10038,
        13 => 10013,
        14 => 10014,
        22 => 10022,
        24 => 10024,
        114 => 10037,
        89 => 10039,
        90 => 10040,
        91 => 10041,
        92 => 10042,
        93 => 10043,
        95 => 10045,
        97 => 10047,
        98 => 10048,
        99 => 10049,
        100 => 10050,
        101 => 10051,
        102 => 10052,
        103 => 10053,
        104 | 32 => 10054,
        105 => 10055,
        106 => 10056,
        107 => 10057,
        108 => 10058,
        110 => 10060,
        111 => 10061,
        113 => 10065,
        _ => 10022,
    }
}
/// Records the WSA error for the current errno; SOCKET_ERROR.
pub(crate) fn fail() -> i32 {
    error::set(wsa_error(errno()));
    -1
}
pub(crate) fn family_to_host(f: i32) -> i32 {
    if f == AF_INET6_WINDOWS as i32 {
        AF_INET6 as i32
    } else {
        f
    }
}
pub(crate) fn family_to_windows(f: i32) -> i32 {
    if f == AF_INET6 as i32 {
        AF_INET6_WINDOWS as i32
    } else {
        f
    }
}
/// A sockaddr in host form (only the family field differs from Windows).
#[repr(C, align(8))]
pub(crate) struct Address {
    pub bytes: [u8; SOCKADDR_STORAGE],
    pub length: socklen_t,
}
impl Address {
    pub fn empty() -> Self {
        Address {
            bytes: [0; SOCKADDR_STORAGE],
            length: SOCKADDR_STORAGE as socklen_t,
        }
    }
    /// From a Windows sockaddr; None (WSAEFAULT set) if it is not one.
    pub unsafe fn from_windows(address: *const c_void, length: i32) -> Option<Self> {
        if address.is_null() || !(2..=SOCKADDR_STORAGE as i32).contains(&length) {
            error::set(10014);
            return None;
        }
        let mut out = Address::empty();
        unsafe {
            std::ptr::copy_nonoverlapping(
                address.cast::<u8>(),
                out.bytes.as_mut_ptr(),
                length as usize,
            )
        };
        let family = family_to_host(u16::from_le_bytes([out.bytes[0], out.bytes[1]]) as i32) as u16;
        out.bytes[..2].copy_from_slice(&family.to_le_bytes());
        out.length = length as socklen_t;
        Some(out)
    }
    pub fn as_ptr(&self) -> *const c_void {
        self.bytes.as_ptr().cast()
    }
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.bytes.as_mut_ptr().cast()
    }
    /// Copies into a Windows sockaddr buffer (truncated to `*length`), storing the
    /// full length.
    pub unsafe fn to_windows(&self, out: *mut c_void, length: *mut i32) {
        if out.is_null() || length.is_null() {
            return;
        }
        let n = (self.length as usize).min(unsafe { *length }.max(0) as usize);
        unsafe { std::ptr::copy_nonoverlapping(self.bytes.as_ptr(), out.cast::<u8>(), n) };
        if n >= 2 {
            let family =
                family_to_windows(u16::from_le_bytes([self.bytes[0], self.bytes[1]]) as i32) as u16;
            unsafe {
                std::ptr::copy_nonoverlapping(family.to_le_bytes().as_ptr(), out.cast::<u8>(), 2)
            };
        }
        unsafe { *length = self.length as i32 };
    }
}

static STARTED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub(crate) fn started() -> bool {
    STARTED.load(std::sync::atomic::Ordering::Acquire) > 0
}
/// WSAStartup. WSADATA (x64): wVersion, wHighVersion, iMaxSockets, iMaxUdpDg,
/// lpVendorInfo @8, szDescription[257] @16, szSystemStatus[129] @273; 408 bytes.
pub unsafe extern "win64" fn startup(version: u16, data: *mut u8) -> i32 {
    if data.is_null() {
        return 10014;
    }
    let (major, minor) = (version & 0xff, version >> 8);
    if major < 1 || (major == 1 && minor < 1) {
        return 10092; // WSAVERNOTSUPPORTED
    }
    let used: u16 = if major > 2 || (major == 2 && minor >= 2) {
        0x0202
    } else {
        version
    };
    unsafe {
        data.write_bytes(0, 408);
        data.cast::<u16>().write_unaligned(used);
        data.add(2).cast::<u16>().write_unaligned(0x0202);
        let description = b"PeekGame Winsock 2.2 (Linux sockets)";
        std::ptr::copy_nonoverlapping(description.as_ptr(), data.add(16), description.len());
        std::ptr::copy_nonoverlapping(b"Running".as_ptr(), data.add(273), 7);
    }
    STARTED.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    0
}
pub extern "win64" fn cleanup() -> i32 {
    let ok = STARTED
        .fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |n| n.checked_sub(1),
        )
        .is_ok();
    if ok {
        0
    } else {
        error::set(10093); // WSANOTINITIALISED
        -1
    }
}
pub extern "win64" fn get_last_error() -> i32 {
    error::get_last_error() as i32
}
pub extern "win64" fn set_last_error(code: i32) {
    error::set(code as u32);
}
pub extern "win64" fn htonl(v: u32) -> u32 {
    v.to_be()
}
pub extern "win64" fn htons(v: u16) -> u16 {
    v.to_be()
}
pub extern "win64" fn ntohl(v: u32) -> u32 {
    u32::from_be(v)
}
pub extern "win64" fn ntohs(v: u16) -> u16 {
    u16::from_be(v)
}
