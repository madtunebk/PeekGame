//! Socket calls: creation, connection, data transfer, options, select.
use super::{errno, events, fail, family_to_host, set_errno, Address, INVALID_SOCKET};
use crate::api::system::error;
use std::ffi::c_void;

unsafe extern "C" {
    fn socket(domain: i32, kind: i32, protocol: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn bind(fd: i32, address: *const c_void, length: u32) -> i32;
    fn connect(fd: i32, address: *const c_void, length: u32) -> i32;
    fn listen(fd: i32, backlog: i32) -> i32;
    fn accept4(fd: i32, address: *mut c_void, length: *mut u32, flags: i32) -> i32;
    fn getsockname(fd: i32, address: *mut c_void, length: *mut u32) -> i32;
    fn getpeername(fd: i32, address: *mut c_void, length: *mut u32) -> i32;
    fn send(fd: i32, buffer: *const c_void, length: usize, flags: i32) -> isize;
    fn recv(fd: i32, buffer: *mut c_void, length: usize, flags: i32) -> isize;
    fn sendto(
        fd: i32,
        buffer: *const c_void,
        length: usize,
        flags: i32,
        to: *const c_void,
        to_length: u32,
    ) -> isize;
    fn recvfrom(
        fd: i32,
        buffer: *mut c_void,
        length: usize,
        flags: i32,
        from: *mut c_void,
        from_length: *mut u32,
    ) -> isize;
    fn sendmsg(fd: i32, message: *const MessageHeader, flags: i32) -> isize;
    fn recvmsg(fd: i32, message: *mut MessageHeader, flags: i32) -> isize;
    fn shutdown(fd: i32, how: i32) -> i32;
    fn setsockopt(fd: i32, level: i32, name: i32, value: *const c_void, length: u32) -> i32;
    fn getsockopt(fd: i32, level: i32, name: i32, value: *mut c_void, length: *mut u32) -> i32;
    fn fcntl(fd: i32, command: i32, ...) -> i32;
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, count: u64, timeout: i32) -> i32;
    fn gethostname(name: *mut u8, length: usize) -> i32;
}
#[repr(C)]
pub(crate) struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}
#[repr(C)]
struct IoVec {
    base: *mut c_void,
    length: usize,
}
#[repr(C)]
pub(super) struct MessageHeader {
    name: *mut c_void,
    name_length: u32,
    iov: *mut IoVec,
    iov_length: usize,
    control: *mut c_void,
    control_length: usize,
    flags: i32,
}
pub(super) const POLLIN: i16 = 1;
pub(super) const POLLPRI: i16 = 2;
pub(super) const POLLOUT: i16 = 4;
pub(super) const POLLERR: i16 = 8;
pub(super) const POLLHUP: i16 = 16;
pub(super) const POLLRDHUP: i16 = 0x2000;
const SOCK_CLOEXEC: i32 = 0x80000;
const MSG_WAITALL: i32 = 0x100;
const MSG_NOSIGNAL: i32 = 0x4000;
const MSG_TRUNC: i32 = 0x20;
pub(super) const F_GETFL: i32 = 3;
pub(super) const F_SETFL: i32 = 4;
pub(super) const O_NONBLOCK: i32 = 0x800;
pub(super) const FIONREAD: u64 = 0x541b;
pub(super) const SOL_SOCKET: i32 = 1;
pub(super) const SO_TYPE: i32 = 3;
pub(super) const SO_ERROR: i32 = 4;
pub(super) const SO_ACCEPTCONN: i32 = 30;

/// MSG_OOB 1, MSG_PEEK 2, MSG_DONTROUTE 4 match Linux; Windows MSG_WAITALL is 8.
fn message_flags(flags: i32) -> i32 {
    (flags & 7) | if flags & 8 != 0 { MSG_WAITALL } else { 0 }
}
fn fd(s: usize) -> i32 {
    s as i32
}
pub(crate) fn is_socket(s: usize) -> bool {
    let mut kind = 0i32;
    let mut length = 4u32;
    unsafe {
        getsockopt(
            fd(s),
            SOL_SOCKET,
            SO_TYPE,
            (&mut kind as *mut i32).cast(),
            &mut length,
        ) == 0
    }
}
/// Keeps errno across a bookkeeping call.
fn noting(f: impl FnOnce()) {
    let saved = errno();
    f();
    set_errno(saved);
}

pub extern "win64" fn create(family: i32, kind: i32, protocol: i32) -> usize {
    let s = unsafe { socket(family_to_host(family), kind | SOCK_CLOEXEC, protocol) };
    if s < 0 {
        fail();
        return INVALID_SOCKET;
    }
    s as usize
}
/// WSASocketW/A: protocol info and socket groups are not supported;
/// WSA_FLAG_OVERLAPPED (1) and WSA_FLAG_NO_HANDLE_INHERIT (0x80) are accepted.
pub extern "win64" fn wsa_socket(
    family: i32,
    kind: i32,
    protocol: i32,
    info: usize,
    group: u32,
    flags: u32,
) -> usize {
    if info != 0 || group != 0 || flags & !0x81 != 0 {
        eprintln!("WSASocket: protocol info, groups and flags {flags:#x} are not supported");
        error::set(10045);
        return INVALID_SOCKET;
    }
    create(family, kind, protocol)
}
pub extern "win64" fn close_socket(s: usize) -> i32 {
    events::note(fd(s), events::Note::Close);
    if unsafe { close(fd(s)) } != 0 {
        fail()
    } else {
        0
    }
}
pub unsafe extern "win64" fn bind_socket(s: usize, address: *const c_void, length: i32) -> i32 {
    let Some(a) = (unsafe { Address::from_windows(address, length) }) else {
        return -1;
    };
    if unsafe { bind(fd(s), a.as_ptr(), a.length) } != 0 {
        fail()
    } else {
        0
    }
}
pub unsafe extern "win64" fn connect_socket(s: usize, address: *const c_void, length: i32) -> i32 {
    let Some(a) = (unsafe { Address::from_windows(address, length) }) else {
        return -1;
    };
    let rc = unsafe { connect(fd(s), a.as_ptr(), a.length) };
    if rc == 0 || errno() == 115 {
        noting(|| events::note(fd(s), events::Note::Connect));
    }
    if rc != 0 {
        fail()
    } else {
        0
    }
}
pub unsafe extern "win64" fn wsa_connect(
    s: usize,
    address: *const c_void,
    length: i32,
    caller: usize,
    callee: usize,
    sqos: usize,
    gqos: usize,
) -> i32 {
    if caller | callee | sqos | gqos != 0 {
        eprintln!("WSAConnect: caller/callee data and QoS are not supported");
        error::set(10045);
        return -1;
    }
    unsafe { connect_socket(s, address, length) }
}
pub extern "win64" fn listen_socket(s: usize, backlog: i32) -> i32 {
    if unsafe { listen(fd(s), backlog) } != 0 {
        return fail();
    }
    events::note(fd(s), events::Note::Listen);
    0
}
pub unsafe extern "win64" fn accept_socket(
    s: usize,
    address: *mut c_void,
    length: *mut i32,
) -> usize {
    let mut a = Address::empty();
    let accepted = unsafe { accept4(fd(s), a.as_mut_ptr(), &mut a.length, SOCK_CLOEXEC) };
    noting(|| events::note(fd(s), events::Note::Accept(accepted)));
    if accepted < 0 {
        fail();
        return INVALID_SOCKET;
    }
    unsafe { a.to_windows(address, length) };
    accepted as usize
}
pub unsafe extern "win64" fn socket_name(s: usize, address: *mut c_void, length: *mut i32) -> i32 {
    let mut a = Address::empty();
    if unsafe { getsockname(fd(s), a.as_mut_ptr(), &mut a.length) } != 0 {
        return fail();
    }
    if address.is_null() || length.is_null() || unsafe { *length } < a.length as i32 {
        error::set(10014);
        return -1;
    }
    unsafe { a.to_windows(address, length) };
    0
}
pub unsafe extern "win64" fn peer_name(s: usize, address: *mut c_void, length: *mut i32) -> i32 {
    let mut a = Address::empty();
    if unsafe { getpeername(fd(s), a.as_mut_ptr(), &mut a.length) } != 0 {
        return fail();
    }
    if address.is_null() || length.is_null() || unsafe { *length } < a.length as i32 {
        error::set(10014);
        return -1;
    }
    unsafe { a.to_windows(address, length) };
    0
}
fn sent(s: usize, n: isize) -> i32 {
    if n < 0 {
        if errno() == 11 {
            noting(|| events::note(fd(s), events::Note::SendBlocked));
        }
        return fail();
    }
    n as i32
}
pub unsafe extern "win64" fn send_data(
    s: usize,
    buffer: *const c_void,
    length: i32,
    flags: i32,
) -> i32 {
    let n = unsafe {
        send(
            fd(s),
            buffer,
            length.max(0) as usize,
            message_flags(flags) | MSG_NOSIGNAL,
        )
    };
    sent(s, n)
}
pub unsafe extern "win64" fn recv_data(
    s: usize,
    buffer: *mut c_void,
    length: i32,
    flags: i32,
) -> i32 {
    let n = unsafe { recv(fd(s), buffer, length.max(0) as usize, message_flags(flags)) };
    noting(|| events::note(fd(s), events::Note::Receive));
    if n < 0 {
        fail()
    } else {
        n as i32
    }
}
pub unsafe extern "win64" fn send_to(
    s: usize,
    buffer: *const c_void,
    length: i32,
    flags: i32,
    to: *const c_void,
    to_length: i32,
) -> i32 {
    let a = if to.is_null() {
        None
    } else {
        match unsafe { Address::from_windows(to, to_length) } {
            Some(a) => Some(a),
            None => return -1,
        }
    };
    let (to, to_length) = a
        .as_ref()
        .map_or((std::ptr::null(), 0), |a| (a.as_ptr(), a.length));
    let n = unsafe {
        sendto(
            fd(s),
            buffer,
            length.max(0) as usize,
            message_flags(flags) | MSG_NOSIGNAL,
            to,
            to_length,
        )
    };
    sent(s, n)
}
pub unsafe extern "win64" fn recv_from(
    s: usize,
    buffer: *mut c_void,
    length: i32,
    flags: i32,
    from: *mut c_void,
    from_length: *mut i32,
) -> i32 {
    let mut a = Address::empty();
    let n = unsafe {
        recvfrom(
            fd(s),
            buffer,
            length.max(0) as usize,
            message_flags(flags),
            a.as_mut_ptr(),
            &mut a.length,
        )
    };
    noting(|| events::note(fd(s), events::Note::Receive));
    if n < 0 {
        return fail();
    }
    if !from.is_null() && !from_length.is_null() {
        unsafe { a.to_windows(from, from_length) };
    }
    n as i32
}
pub extern "win64" fn shutdown_socket(s: usize, how: i32) -> i32 {
    if unsafe { shutdown(fd(s), how) } != 0 {
        fail()
    } else {
        0
    }
}

/// WSABUF (x64): ULONG len, CHAR *buf at 8.
#[repr(C)]
pub struct WsaBuffer {
    length: u32,
    buffer: *mut c_void,
}
unsafe fn iov(buffers: *const WsaBuffer, count: u32) -> Option<Vec<IoVec>> {
    if buffers.is_null() || count == 0 || count > 1024 {
        error::set(10014);
        return None;
    }
    let list = unsafe { std::slice::from_raw_parts(buffers, count as usize) };
    Some(
        list.iter()
            .map(|b| IoVec {
                base: b.buffer,
                length: b.length as usize,
            })
            .collect(),
    )
}
fn no_overlapped(what: &str, overlapped: usize, routine: usize) -> bool {
    if overlapped != 0 || routine != 0 {
        eprintln!("{what}: overlapped operations are not supported");
        error::set(10045);
        return false;
    }
    true
}
unsafe fn send_message(
    s: usize,
    buffers: *const WsaBuffer,
    count: u32,
    done: *mut u32,
    flags: u32,
    to: Option<&Address>,
) -> i32 {
    let Some(mut vectors) = (unsafe { iov(buffers, count) }) else {
        return -1;
    };
    let message = MessageHeader {
        name: to.map_or(std::ptr::null_mut(), |a| a.as_ptr() as *mut c_void),
        name_length: to.map_or(0, |a| a.length),
        iov: vectors.as_mut_ptr(),
        iov_length: vectors.len(),
        control: std::ptr::null_mut(),
        control_length: 0,
        flags: 0,
    };
    let n = unsafe { sendmsg(fd(s), &message, message_flags(flags as i32) | MSG_NOSIGNAL) };
    if sent(s, n) < 0 {
        return -1;
    }
    if !done.is_null() {
        unsafe { *done = n as u32 };
    }
    0
}
unsafe fn receive_message(
    s: usize,
    buffers: *const WsaBuffer,
    count: u32,
    done: *mut u32,
    flags: *mut u32,
    from: Option<&mut Address>,
) -> i32 {
    if flags.is_null() {
        error::set(10014);
        return -1;
    }
    let Some(mut vectors) = (unsafe { iov(buffers, count) }) else {
        return -1;
    };
    let (name, name_length) = match &from {
        Some(a) => (a.bytes.as_ptr() as *mut c_void, a.bytes.len() as u32),
        None => (std::ptr::null_mut(), 0),
    };
    let mut message = MessageHeader {
        name,
        name_length,
        iov: vectors.as_mut_ptr(),
        iov_length: vectors.len(),
        control: std::ptr::null_mut(),
        control_length: 0,
        flags: 0,
    };
    let n = unsafe { recvmsg(fd(s), &mut message, message_flags(*flags as i32)) };
    noting(|| events::note(fd(s), events::Note::Receive));
    if n < 0 {
        return fail();
    }
    if let Some(a) = from {
        a.length = message.name_length;
    }
    if message.flags & MSG_TRUNC != 0 {
        error::set(10040); // WSAEMSGSIZE: a datagram did not fit
        return -1;
    }
    unsafe {
        *flags = 0;
        if !done.is_null() {
            *done = n as u32;
        }
    }
    0
}
pub unsafe extern "win64" fn wsa_send(
    s: usize,
    buffers: *const WsaBuffer,
    count: u32,
    done: *mut u32,
    flags: u32,
    overlapped: usize,
    routine: usize,
) -> i32 {
    if !no_overlapped("WSASend", overlapped, routine) {
        return -1;
    }
    unsafe { send_message(s, buffers, count, done, flags, None) }
}
pub unsafe extern "win64" fn wsa_send_to(
    s: usize,
    buffers: *const WsaBuffer,
    count: u32,
    done: *mut u32,
    flags: u32,
    to: *const c_void,
    to_length: i32,
    overlapped: usize,
    routine: usize,
) -> i32 {
    if !no_overlapped("WSASendTo", overlapped, routine) {
        return -1;
    }
    let a = if to.is_null() {
        None
    } else {
        match unsafe { Address::from_windows(to, to_length) } {
            Some(a) => Some(a),
            None => return -1,
        }
    };
    unsafe { send_message(s, buffers, count, done, flags, a.as_ref()) }
}
pub unsafe extern "win64" fn wsa_recv(
    s: usize,
    buffers: *const WsaBuffer,
    count: u32,
    done: *mut u32,
    flags: *mut u32,
    overlapped: usize,
    routine: usize,
) -> i32 {
    if !no_overlapped("WSARecv", overlapped, routine) {
        return -1;
    }
    unsafe { receive_message(s, buffers, count, done, flags, None) }
}
pub unsafe extern "win64" fn wsa_recv_from(
    s: usize,
    buffers: *const WsaBuffer,
    count: u32,
    done: *mut u32,
    flags: *mut u32,
    from: *mut c_void,
    from_length: *mut i32,
    overlapped: usize,
    routine: usize,
) -> i32 {
    if !no_overlapped("WSARecvFrom", overlapped, routine) {
        return -1;
    }
    let mut a = Address::empty();
    let rc = unsafe { receive_message(s, buffers, count, done, flags, Some(&mut a)) };
    if rc == 0 && !from.is_null() && !from_length.is_null() {
        unsafe { a.to_windows(from, from_length) };
    }
    rc
}

/// FIONBIO 0x8004667E (non-blocking), FIONREAD 0x4004667F (bytes available).
pub unsafe extern "win64" fn ioctl_socket(s: usize, command: i32, argument: *mut u32) -> i32 {
    if argument.is_null() {
        error::set(10014);
        return -1;
    }
    match command as u32 {
        0x8004_667e => {
            let flags = unsafe { fcntl(fd(s), F_GETFL) };
            if flags < 0 {
                return fail();
            }
            let flags = if unsafe { *argument } != 0 {
                flags | O_NONBLOCK
            } else {
                flags & !O_NONBLOCK
            };
            if unsafe { fcntl(fd(s), F_SETFL, flags) } != 0 {
                return fail();
            }
            0
        }
        0x4004_667f => {
            let mut n = 0i32;
            if unsafe { ioctl(fd(s), FIONREAD, &mut n) } != 0 {
                return fail();
            }
            unsafe { *argument = n as u32 };
            0
        }
        other => {
            eprintln!("ioctlsocket({other:#x}) is not supported");
            error::set(10045);
            -1
        }
    }
}

/// How a Windows option is stored on Linux.
enum Option_ {
    /// Same 32-bit value at another level/number.
    Int(i32, i32),
    /// SO_SNDTIMEO/SO_RCVTIMEO: DWORD milliseconds <-> struct timeval.
    Timeout(i32),
    /// SO_LINGER: two u_shorts <-> struct linger (two ints).
    Linger,
    /// SO_DONTLINGER: the inverse of linger's onoff.
    DontLinger,
    /// Accepted without effect (SO_EXCLUSIVEADDRUSE: Linux already refuses to share a
    /// port without SO_REUSEADDR).
    Ignored,
    /// IP_DONTFRAGMENT: IP_MTU_DISCOVER (DO when set, DONT when cleared).
    DontFragment,
    /// SO_PROTOCOL_INFOA: WSAPROTOCOL_INFOA
    ProtocolInfoA,
    /// SO_PROTOCOL_INFOW: WSAPROTOCOL_INFOW
    ProtocolInfoW,
}
fn option(level: i32, name: i32) -> Option<Option_> {
    use Option_::*;
    Some(match (level, name) {
        (0xffff, 1) => Int(SOL_SOCKET, 1), // SO_DEBUG
        (0xffff, 2) => Int(SOL_SOCKET, SO_ACCEPTCONN),
        (0xffff, 4) => Int(SOL_SOCKET, 2),       // SO_REUSEADDR
        (0xffff, 8) => Int(SOL_SOCKET, 9),       // SO_KEEPALIVE
        (0xffff, 0x10) => Int(SOL_SOCKET, 5),    // SO_DONTROUTE
        (0xffff, 0x20) => Int(SOL_SOCKET, 6),    // SO_BROADCAST
        (0xffff, 0x100) => Int(SOL_SOCKET, 10),  // SO_OOBINLINE
        (0xffff, 0x1001) => Int(SOL_SOCKET, 7),  // SO_SNDBUF
        (0xffff, 0x1002) => Int(SOL_SOCKET, 8),  // SO_RCVBUF
        (0xffff, 0x1003) => Int(SOL_SOCKET, 19), // SO_SNDLOWAT
        (0xffff, 0x1004) => Int(SOL_SOCKET, 18), // SO_RCVLOWAT
        (0xffff, 0x1005) => Timeout(21),         // SO_SNDTIMEO
        (0xffff, 0x1006) => Timeout(20),         // SO_RCVTIMEO
        (0xffff, 0x1007) => Int(SOL_SOCKET, SO_ERROR),
        (0xffff, 0x1008) => Int(SOL_SOCKET, SO_TYPE),
        (0xffff, 0x2004) => ProtocolInfoA,
        (0xffff, 0x2005) => ProtocolInfoW,
        (0xffff, 0x80) => Linger,
        (0xffff, n) if n == !0x80 => DontLinger,
        (0xffff, n) if n == !4 => Ignored,
        (0, 3) => Int(0, 1),   // IP_TOS
        (0, 4) => Int(0, 2),   // IP_TTL
        (0, 9) => Int(0, 32),  // IP_MULTICAST_IF
        (0, 10) => Int(0, 33), // IP_MULTICAST_TTL
        (0, 11) => Int(0, 34), // IP_MULTICAST_LOOP
        (0, 12) => Int(0, 35), // IP_ADD_MEMBERSHIP
        (0, 13) => Int(0, 36), // IP_DROP_MEMBERSHIP
        (0, 14) => DontFragment,
        (0, 19) => Int(0, 8),    // IP_PKTINFO
        (41, 4) => Int(41, 16),  // IPV6_UNICAST_HOPS
        (41, 9) => Int(41, 17),  // IPV6_MULTICAST_IF
        (41, 10) => Int(41, 18), // IPV6_MULTICAST_HOPS
        (41, 11) => Int(41, 19), // IPV6_MULTICAST_LOOP
        (41, 12) => Int(41, 20), // IPV6_ADD_MEMBERSHIP
        (41, 13) => Int(41, 21), // IPV6_DROP_MEMBERSHIP
        (41, 27) => Int(41, 26), // IPV6_V6ONLY
        (6, 1) => Int(6, 1),     // TCP_NODELAY
        (6, 3) => Int(6, 4),     // TCP_KEEPALIVE -> TCP_KEEPIDLE
        (6, 16) => Int(6, 6),    // TCP_KEEPCNT
        (6, 17) => Int(6, 5),    // TCP_KEEPINTVL
        _ => return None,
    })
}
#[repr(C)]
struct TimeVal {
    seconds: i64,
    microseconds: i64,
}
pub unsafe extern "win64" fn set_option(
    s: usize,
    level: i32,
    name: i32,
    value: *const c_void,
    length: i32,
) -> i32 {
    let Some(kind) = option(level, name) else {
        eprintln!("setsockopt(level {level:#x}, option {name:#x}) is not supported");
        error::set(10042);
        return -1;
    };
    let need = |n: i32| {
        if value.is_null() || length < n {
            error::set(10014);
            false
        } else {
            true
        }
    };
    let rc = match kind {
        Option_::Int(level, name) => {
            if !need(1) {
                return -1;
            }
            // Windows takes BOOL or DWORD; a 1-byte value (some programs pass a char) is widened.
            let v: i32 = if length >= 4 {
                unsafe { value.cast::<i32>().read_unaligned() }
            } else {
                (unsafe { *value.cast::<u8>() }) as i32
            };
            // Membership requests are structures, passed through unchanged.
            if (level == 0 && (35..=36).contains(&name))
                || (level == 41 && (20..=21).contains(&name))
            {
                unsafe { setsockopt(fd(s), level, name, value, length as u32) }
            } else {
                unsafe { setsockopt(fd(s), level, name, (&v as *const i32).cast(), 4) }
            }
        }
        Option_::Timeout(name) => {
            if !need(4) {
                return -1;
            }
            let ms = unsafe { value.cast::<u32>().read_unaligned() };
            let tv = TimeVal {
                seconds: (ms / 1000) as i64,
                microseconds: (ms % 1000) as i64 * 1000,
            };
            unsafe { setsockopt(fd(s), SOL_SOCKET, name, (&tv as *const TimeVal).cast(), 16) }
        }
        Option_::Linger => {
            if !need(4) {
                return -1;
            }
            let l = unsafe { value.cast::<[u16; 2]>().read_unaligned() };
            let linger = [l[0] as i32, l[1] as i32];
            unsafe { setsockopt(fd(s), SOL_SOCKET, 13, linger.as_ptr().cast(), 8) }
        }
        Option_::DontLinger => {
            if !need(4) {
                return -1;
            }
            let off = unsafe { value.cast::<i32>().read_unaligned() } != 0;
            let linger = [(!off) as i32, 0];
            unsafe { setsockopt(fd(s), SOL_SOCKET, 13, linger.as_ptr().cast(), 8) }
        }
        Option_::Ignored => 0,
        Option_::DontFragment => {
            if !need(4) {
                return -1;
            }
            let v: i32 = if unsafe { value.cast::<i32>().read_unaligned() } != 0 {
                2
            } else {
                0
            };
            unsafe { setsockopt(fd(s), 0, 10, (&v as *const i32).cast(), 4) }
        }
        Option_::ProtocolInfoA | Option_::ProtocolInfoW => {
            error::set(10042);
            return -1;
        }
    };
    if rc != 0 {
        fail()
    } else {
        0
    }
}
/// getsockopt with setsockopt's mapping. Results are 32-bit, as Windows writes them;
/// Linux doubles SO_SNDBUF/SO_RCVBUF internally, so they read back as set; SO_ERROR
/// is a WSA error.
pub unsafe extern "win64" fn get_option(
    s: usize,
    level: i32,
    name: i32,
    value: *mut c_void,
    length: *mut i32,
) -> i32 {
    if value.is_null() || length.is_null() || unsafe { *length } < 1 {
        error::set(10014);
        return -1;
    }
    let Some(kind) = option(level, name) else {
        eprintln!("getsockopt(level {level:#x}, option {name:#x}) is not supported");
        error::set(10042);
        return -1;
    };
    let out = |v: u32, bytes: i32| unsafe {
        let n = (*length).min(bytes).max(0) as usize;
        std::ptr::copy_nonoverlapping(v.to_le_bytes().as_ptr(), value.cast::<u8>(), n.min(4));
        *length = n as i32;
    };
    match kind {
        Option_::Int(host_level, host_name) => {
            let mut v = 0i32;
            let mut l = 4u32;
            if unsafe {
                getsockopt(
                    fd(s),
                    host_level,
                    host_name,
                    (&mut v as *mut i32).cast(),
                    &mut l,
                )
            } != 0
            {
                return fail();
            }
            if host_level == SOL_SOCKET && host_name == SO_ERROR && v != 0 {
                v = super::wsa_error(v) as i32;
            }
            if host_level == SOL_SOCKET && (host_name == 7 || host_name == 8) {
                v /= 2;
            }
            out(v as u32, 4);
        }
        Option_::Timeout(host_name) => {
            let mut tv = TimeVal {
                seconds: 0,
                microseconds: 0,
            };
            let mut l = 16u32;
            if unsafe {
                getsockopt(
                    fd(s),
                    SOL_SOCKET,
                    host_name,
                    (&mut tv as *mut TimeVal).cast(),
                    &mut l,
                )
            } != 0
            {
                return fail();
            }
            out((tv.seconds * 1000 + tv.microseconds / 1000) as u32, 4);
        }
        Option_::Linger | Option_::DontLinger => {
            let mut linger = [0i32; 2];
            let mut l = 8u32;
            if unsafe { getsockopt(fd(s), SOL_SOCKET, 13, linger.as_mut_ptr().cast(), &mut l) } != 0
            {
                return fail();
            }
            if matches!(kind, Option_::Linger) {
                out(
                    (linger[0] as u16 as u32) | (linger[1] as u16 as u32) << 16,
                    4,
                );
            } else {
                out((linger[0] == 0) as u32, 4);
            }
        }
        Option_::Ignored => out(0, 4),
        Option_::DontFragment => {
            let mut v = 0i32;
            let mut l = 4u32;
            if unsafe { getsockopt(fd(s), 0, 10, (&mut v as *mut i32).cast(), &mut l) } != 0 {
                return fail();
            }
            out((v == 2) as u32, 4);
        }
        Option_::ProtocolInfoW => {
            if unsafe { *length } < 628 {
                error::set(10014);
                return -1;
            }
            let mut sock_type = 0i32;
            let mut proto = 0i32;
            let mut domain = 0i32;
            let mut l = 4u32;
            unsafe {
                getsockopt(fd(s), SOL_SOCKET, 3, (&mut sock_type as *mut i32).cast(), &mut l);
                l = 4;
                getsockopt(fd(s), SOL_SOCKET, 38, (&mut proto as *mut i32).cast(), &mut l);
                l = 4;
                getsockopt(fd(s), SOL_SOCKET, 39, (&mut domain as *mut i32).cast(), &mut l);
            }
            let af = match domain {
                10 => 23, // AF_INET6 in Windows
                other => other,
            };
            let mut info = [0u8; 628];
            let sf1: u32 = if proto == 6 { 0x20066 } else { 0x600 };
            info[0..4].copy_from_slice(&sf1.to_le_bytes());
            info[24..28].copy_from_slice(&1001u32.to_le_bytes());
            info[28..32].copy_from_slice(&1u32.to_le_bytes()); // ChainLen = 1
            info[60..64].copy_from_slice(&2i32.to_le_bytes()); // Version = 2
            info[64..68].copy_from_slice(&af.to_le_bytes());
            let max_addr: i32 = if af == 23 { 28 } else { 16 };
            info[68..72].copy_from_slice(&max_addr.to_le_bytes());
            info[72..76].copy_from_slice(&16i32.to_le_bytes());
            info[76..80].copy_from_slice(&sock_type.to_le_bytes());
            info[80..84].copy_from_slice(&proto.to_le_bytes());
            let name_str = match proto {
                6 => "MSAFD Tcpip [TCP/IP]",
                17 => "MSAFD Tcpip [UDP/IP]",
                _ => "Unknown",
            };
            let utf16: Vec<u16> = name_str.encode_utf16().collect();
            for (idx, ch) in utf16.iter().enumerate() {
                if idx < 255 {
                    let off = 116 + idx * 2;
                    info[off..off + 2].copy_from_slice(&ch.to_le_bytes());
                }
            }
            unsafe {
                std::ptr::copy_nonoverlapping(info.as_ptr(), value.cast::<u8>(), 628);
                *length = 628;
            }
        }
        Option_::ProtocolInfoA => {
            if unsafe { *length } < 372 {
                error::set(10014);
                return -1;
            }
            let mut sock_type = 0i32;
            let mut proto = 0i32;
            let mut domain = 0i32;
            let mut l = 4u32;
            unsafe {
                getsockopt(fd(s), SOL_SOCKET, 3, (&mut sock_type as *mut i32).cast(), &mut l);
                l = 4;
                getsockopt(fd(s), SOL_SOCKET, 38, (&mut proto as *mut i32).cast(), &mut l);
                l = 4;
                getsockopt(fd(s), SOL_SOCKET, 39, (&mut domain as *mut i32).cast(), &mut l);
            }
            let af = match domain {
                10 => 23, // AF_INET6 in Windows
                other => other,
            };
            let mut info = [0u8; 372];
            let sf1: u32 = if proto == 6 { 0x20066 } else { 0x600 };
            info[0..4].copy_from_slice(&sf1.to_le_bytes());
            info[24..28].copy_from_slice(&1001u32.to_le_bytes());
            info[28..32].copy_from_slice(&1u32.to_le_bytes()); // ChainLen = 1
            info[60..64].copy_from_slice(&2i32.to_le_bytes()); // Version = 2
            info[64..68].copy_from_slice(&af.to_le_bytes());
            let max_addr: i32 = if af == 23 { 28 } else { 16 };
            info[68..72].copy_from_slice(&max_addr.to_le_bytes());
            info[72..76].copy_from_slice(&16i32.to_le_bytes());
            info[76..80].copy_from_slice(&sock_type.to_le_bytes());
            info[80..84].copy_from_slice(&proto.to_le_bytes());
            let name_str = match proto {
                6 => "MSAFD Tcpip [TCP/IP]",
                17 => "MSAFD Tcpip [UDP/IP]",
                _ => "Unknown",
            };
            for (idx, b) in name_str.bytes().enumerate() {
                if idx < 255 {
                    info[116 + idx] = b;
                }
            }
            unsafe {
                std::ptr::copy_nonoverlapping(info.as_ptr(), value.cast::<u8>(), 372);
                *length = 372;
            }
        }
    }
    0
}

/// fd_set (Windows): u_int fd_count @0, SOCKET fd_array[] @8.
unsafe fn set_count(set: *const u8) -> usize {
    if set.is_null() {
        0
    } else {
        unsafe { set.cast::<u32>().read_unaligned() as usize }
    }
}
unsafe fn set_item(set: *const u8, i: usize) -> usize {
    unsafe { set.add(8 + 8 * i).cast::<usize>().read_unaligned() }
}
/// select: poll() over the three sets; each set keeps its ready sockets.
/// timeval: 32-bit tv_sec and tv_usec; NULL waits for ever.
pub unsafe extern "win64" fn select(
    _nfds: i32,
    read: *mut u8,
    write: *mut u8,
    except: *mut u8,
    timeout: *const [i32; 2],
) -> i32 {
    let sets = [read, write, except];
    let wanted = [POLLIN, POLLOUT, POLLPRI];
    let mut fds = Vec::new();
    for (k, &set) in sets.iter().enumerate() {
        for i in 0..unsafe { set_count(set) } {
            fds.push(PollFd {
                fd: unsafe { set_item(set, i) } as i32,
                events: wanted[k],
                revents: 0,
            });
        }
    }
    if fds.is_empty() {
        error::set(10022); // WSAEINVAL: nothing to wait for
        return -1;
    }
    let ms = if timeout.is_null() {
        -1
    } else {
        let t = unsafe { *timeout };
        t[0].saturating_mul(1000).saturating_add(t[1] / 1000)
    };
    if unsafe { poll(fds.as_mut_ptr(), fds.len() as u64, ms) } < 0 {
        return fail();
    }
    let mut ready = 0;
    let mut k = 0;
    for (which, &set) in sets.iter().enumerate() {
        if set.is_null() {
            continue;
        }
        let n = unsafe { set_count(set) };
        let mut keep = 0usize;
        for i in 0..n {
            let got = fds[k].revents;
            k += 1;
            let hit = match which {
                0 => got & (POLLIN | POLLHUP | POLLERR) != 0,
                1 => got & (POLLOUT | POLLERR | POLLHUP) != 0,
                _ => got & (POLLPRI | POLLERR) != 0,
            };
            if hit {
                unsafe {
                    let v = set_item(set, i);
                    set.add(8 + 8 * keep).cast::<usize>().write_unaligned(v);
                }
                keep += 1;
                ready += 1;
            }
        }
        unsafe { set.cast::<u32>().write_unaligned(keep as u32) };
    }
    ready
}
pub unsafe extern "win64" fn fd_is_set(s: usize, set: *const u8) -> i32 {
    (0..unsafe { set_count(set) }).any(|i| unsafe { set_item(set, i) } == s) as i32
}
/// WSAIoctl: SIO_BASE_HANDLE and SIO_BSP_HANDLE* answer the socket itself (there are
/// no layered providers); other controls and overlapped calls are WSAEOPNOTSUPP.
pub unsafe extern "win64" fn wsa_ioctl(
    s: usize,
    code: u32,
    _input: *const c_void,
    _input_length: u32,
    output: *mut c_void,
    output_length: u32,
    returned: *mut u32,
    overlapped: usize,
    routine: usize,
) -> i32 {
    if overlapped != 0
        || routine != 0
        || ![0x4800_0022, 0x4800_001b, 0x4800_001c, 0x4800_001d].contains(&code)
    {
        eprintln!(
            "WSAIoctl({code:#x}){} is not supported",
            if overlapped | routine != 0 {
                " overlapped"
            } else {
                ""
            }
        );
        error::set(10045);
        return -1;
    }
    if !is_socket(s) {
        error::set(10038);
        return -1;
    }
    if output.is_null() || output_length < 8 {
        error::set(10014);
        return -1;
    }
    unsafe {
        output.cast::<usize>().write_unaligned(s);
        if !returned.is_null() {
            *returned = 8;
        }
    }
    0
}
fn host_name() -> Option<Vec<u8>> {
    let mut host = [0u8; 256];
    if unsafe { gethostname(host.as_mut_ptr(), host.len()) } != 0 {
        fail();
        return None;
    }
    host[255] = 0;
    let n = host.iter().position(|&b| b == 0).unwrap_or(255);
    Some(host[..n].to_vec())
}
pub unsafe extern "win64" fn get_host_name(out: *mut u8, size: i32) -> i32 {
    let Some(host) = host_name() else {
        return -1;
    };
    if out.is_null() || size <= host.len() as i32 {
        error::set(10014);
        return -1;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(host.as_ptr(), out, host.len());
        *out.add(host.len()) = 0;
    }
    0
}
pub unsafe extern "win64" fn get_host_name_w(out: *mut u16, size: i32) -> i32 {
    let Some(host) = host_name() else {
        return -1;
    };
    let wide: Vec<u16> = String::from_utf8_lossy(&host).encode_utf16().collect();
    if out.is_null() || size <= wide.len() as i32 {
        error::set(10014);
        return -1;
    }
    unsafe { crate::api::text::copy_out(&wide, out, size as u32) };
    0
}
