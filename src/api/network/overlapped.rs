//! Overlapped WSASend/WSASendTo/WSARecv/WSARecvFrom completed through OVERLAPPED.hEvent,
//! and WSAGetOverlappedResult. An operation that can finish at once completes
//! synchronously (return 0, OVERLAPPED filled, event signalled), as on Windows.
//! Otherwise the call returns WSA_IO_PENDING and a poll thread performs it into the
//! caller's buffers when the socket becomes ready, fills OVERLAPPED and signals the
//! event. Completion routines and I/O completion ports are not supported for sockets.
//! closesocket cancels pending operations with WSA_OPERATION_ABORTED.
use super::{errno, wsa_error, Address};
use crate::api::{system::error, threading::events as win_events};
use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicI32, AtomicUsize, Ordering},
        Mutex, Once,
    },
};
unsafe extern "C" {
    fn sendmsg(fd: i32, message: *const Message, flags: i32) -> isize;
    fn recvmsg(fd: i32, message: *mut Message, flags: i32) -> isize;
    fn eventfd(initial: u32, flags: i32) -> i32;
    fn read(fd: i32, buffer: *mut c_void, length: usize) -> isize;
    fn write(fd: i32, buffer: *const c_void, length: usize) -> isize;
    fn poll(fds: *mut super::sockets::PollFd, count: u64, timeout: i32) -> i32;
}
#[repr(C)]
struct IoVec {
    base: *mut c_void,
    length: usize,
}
#[repr(C)]
struct Message {
    name: *mut c_void,
    name_length: u32,
    iov: *mut IoVec,
    iov_length: usize,
    control: *mut c_void,
    control_length: usize,
    flags: i32,
}
const MSG_DONTWAIT: i32 = 0x40;
const MSG_NOSIGNAL: i32 = 0x4000;
const MSG_TRUNC: i32 = 0x20;
const EAGAIN: i32 = 11;
const STATUS_PENDING: usize = 0x103;
const STATUS_BUFFER_OVERFLOW: usize = 0x8000_0005;
const STATUS_CANCELLED: usize = 0xC000_0120;
const WSA_IO_PENDING: u32 = 997;
const WSA_IO_INCOMPLETE: u32 = 996;
const WSA_OPERATION_ABORTED: u32 = 995;
const WSAEMSGSIZE: u32 = 10040;
/// Other failures keep their WSA code in OVERLAPPED.Internal under this tag (a
/// customer-defined NTSTATUS); WSAGetOverlappedResult reports the code back.
const WSA_TAG: usize = 0xE000_0000;

/// OVERLAPPED (x64): Internal @0, InternalHigh @8, Offset @16, OffsetHigh @20, hEvent @24.
struct Overlapped(usize);
impl Overlapped {
    fn internal(&self) -> &AtomicUsize {
        unsafe { &*(self.0 as *const AtomicUsize) }
    }
    fn event(&self) -> usize {
        // The low bit only asks Windows not to queue a completion packet.
        unsafe { *((self.0 + 24) as *const usize) & !1 }
    }
    fn complete(&self, status: usize, bytes: usize, flags: u32) {
        unsafe {
            *((self.0 + 8) as *mut usize) = bytes;
            *((self.0 + 16) as *mut u32) = flags;
        }
        self.internal().store(status, Ordering::Release);
        let event = self.event();
        if event != 0 {
            win_events::set(event);
        }
    }
}

pub(super) enum Kind {
    Receive { from: usize, from_length: usize },
    Send { to: Option<Box<Address>> },
}
struct Pending {
    fd: i32,
    overlapped: usize,
    buffers: Vec<(usize, u32)>,
    flags: i32,
    kind: Kind,
}
// The raw addresses refer to caller memory that Windows requires to stay valid
// until the operation completes.
unsafe impl Send for Pending {}

static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());
static WAKE: AtomicI32 = AtomicI32::new(-1);

fn wake() {
    let fd = WAKE.load(Ordering::Acquire);
    if fd >= 0 {
        let _ = unsafe { write(fd, 1u64.to_le_bytes().as_ptr().cast(), 8) };
    }
}
fn start() {
    static START: Once = Once::new();
    START.call_once(|| {
        WAKE.store(unsafe { eventfd(0, 0x80000 | 0x800) }, Ordering::Release);
        let _ = std::thread::Builder::new().name("wsa-overlapped".into()).spawn(reactor);
    });
}

/// One attempt without blocking: Ok(Some(status, bytes, flags)) when finished,
/// Ok(None) when it would block.
fn attempt(p: &Pending) -> Option<(usize, usize, u32)> {
    let mut vectors: Vec<IoVec> =
        p.buffers.iter().map(|&(base, length)| IoVec { base: base as *mut c_void, length: length as usize }).collect();
    match &p.kind {
        Kind::Receive { from, from_length } => {
            let mut address = Address::empty();
            let mut message = Message {
                name: if *from != 0 { address.bytes.as_mut_ptr().cast() } else { std::ptr::null_mut() },
                name_length: if *from != 0 { address.bytes.len() as u32 } else { 0 },
                iov: vectors.as_mut_ptr(),
                iov_length: vectors.len(),
                control: std::ptr::null_mut(),
                control_length: 0,
                flags: 0,
            };
            let n = unsafe { recvmsg(p.fd, &mut message, p.flags | MSG_DONTWAIT) };
            if n < 0 {
                let e = errno();
                return (e != EAGAIN).then(|| (WSA_TAG | wsa_error(e) as usize, 0, 0));
            }
            super::events::note(p.fd, super::events::Note::Receive);
            if *from != 0 && *from_length != 0 {
                address.length = message.name_length;
                unsafe { address.to_windows(*from as *mut c_void, *from_length as *mut i32) };
            }
            let status = if message.flags & MSG_TRUNC != 0 { STATUS_BUFFER_OVERFLOW } else { 0 };
            Some((status, n as usize, 0))
        }
        Kind::Send { to } => {
            let message = Message {
                name: to.as_ref().map_or(std::ptr::null_mut(), |a| a.as_ptr() as *mut c_void),
                name_length: to.as_ref().map_or(0, |a| a.length),
                iov: vectors.as_mut_ptr(),
                iov_length: vectors.len(),
                control: std::ptr::null_mut(),
                control_length: 0,
                flags: 0,
            };
            let n = unsafe { sendmsg(p.fd, &message, p.flags | MSG_DONTWAIT | MSG_NOSIGNAL) };
            if n < 0 {
                let e = errno();
                return (e != EAGAIN).then(|| (WSA_TAG | wsa_error(e) as usize, 0, 0));
            }
            Some((0, n as usize, 0))
        }
    }
}
fn reactor() {
    use super::sockets::{PollFd, POLLERR, POLLHUP, POLLIN, POLLOUT};
    loop {
        let mut fds = vec![PollFd { fd: WAKE.load(Ordering::Acquire), events: POLLIN, revents: 0 }];
        for p in PENDING.lock().unwrap().iter() {
            let events = match p.kind { Kind::Receive { .. } => POLLIN, Kind::Send { .. } => POLLOUT };
            match fds.iter_mut().skip(1).find(|f| f.fd == p.fd) {
                Some(f) => f.events |= events,
                None => fds.push(PollFd { fd: p.fd, events, revents: 0 }),
            }
        }
        if unsafe { poll(fds.as_mut_ptr(), fds.len() as u64, -1) } < 0 {
            continue;
        }
        if fds[0].revents != 0 {
            let mut counter = [0u8; 8];
            let _ = unsafe { read(fds[0].fd, counter.as_mut_ptr().cast(), 8) };
        }
        let ready: Vec<(i32, i16)> = fds[1..].iter().filter(|f| f.revents != 0).map(|f| (f.fd, f.revents)).collect();
        if ready.is_empty() {
            continue;
        }
        let mut done = Vec::new();
        {
            let mut list = PENDING.lock().unwrap();
            // Operations on one socket finish in the order they were issued.
            let mut blocked: Vec<(i32, bool)> = Vec::new();
            list.retain(|p| {
                let Some(&(_, revents)) = ready.iter().find(|(fd, _)| *fd == p.fd) else { return true };
                let receive = matches!(p.kind, Kind::Receive { .. });
                let wanted = if receive { POLLIN | POLLERR | POLLHUP } else { POLLOUT | POLLERR | POLLHUP };
                if revents & wanted == 0 || blocked.contains(&(p.fd, receive)) {
                    return true;
                }
                match attempt(p) {
                    Some(result) => {
                        done.push((p.overlapped, result));
                        false
                    }
                    None => {
                        blocked.push((p.fd, receive));
                        true
                    }
                }
            });
        }
        for (overlapped, (status, bytes, flags)) in done {
            Overlapped(overlapped).complete(status, bytes, flags);
        }
    }
}

/// Starts an overlapped operation. Returns 0 (completed now) or -1 with the WSA error
/// set (WSA_IO_PENDING when queued).
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn start_operation(
    fd: i32,
    buffers: *const u8,
    count: u32,
    done: *mut u32,
    flags: *mut u32,
    send_flags: u32,
    kind: Kind,
    overlapped: usize,
) -> i32 {
    if buffers.is_null() || count == 0 || count > 1024 {
        error::set(10014);
        return -1;
    }
    // WSABUF (x64): len @0, buf @8.
    let buffers: Vec<(usize, u32)> = (0..count as usize)
        .map(|i| unsafe {
            let entry = buffers.add(i * 16);
            (*(entry.add(8) as *const usize), *(entry as *const u32))
        })
        .collect();
    let receive = matches!(kind, Kind::Receive { .. });
    let windows_flags = if receive {
        if flags.is_null() {
            error::set(10014);
            return -1;
        }
        unsafe { *flags }
    } else {
        send_flags
    };
    // MSG_OOB 1, MSG_PEEK 2, MSG_DONTROUTE 4 match Linux; MSG_PARTIAL and others do not.
    if windows_flags & !7 != 0 {
        error::set(10045);
        return -1;
    }
    let pending = Pending { fd, overlapped, buffers, flags: windows_flags as i32, kind };
    let ov = Overlapped(overlapped);
    ov.internal().store(STATUS_PENDING, Ordering::Release);
    unsafe { *((overlapped + 8) as *mut usize) = 0 };
    start();
    let mut list = PENDING.lock().unwrap();
    // Only try at once when nothing of the same direction is queued ahead of it.
    let queued = list.iter().any(|p| p.fd == fd && matches!(p.kind, Kind::Receive { .. }) == receive);
    if !queued {
        if let Some((status, bytes, out_flags)) = attempt(&pending) {
            drop(list);
            ov.complete(status, bytes, out_flags);
            if status == 0 {
                unsafe {
                    if !done.is_null() {
                        *done = bytes as u32;
                    }
                    if receive {
                        *flags = out_flags;
                    }
                }
                error::set(0);
                return 0;
            }
            error::set(status_error(status));
            return -1;
        }
    }
    list.push(pending);
    drop(list);
    wake();
    error::set(WSA_IO_PENDING);
    -1
}
pub(super) fn receive(from: usize, from_length: usize) -> Kind {
    Kind::Receive { from, from_length }
}
pub(super) fn send(to: Option<Address>) -> Kind {
    Kind::Send { to: to.map(Box::new) }
}
/// The WSA error for a completed operation's status.
fn status_error(status: usize) -> u32 {
    match status {
        STATUS_BUFFER_OVERFLOW => WSAEMSGSIZE,
        STATUS_CANCELLED => WSA_OPERATION_ABORTED,
        s if s & 0xF000_0000 == WSA_TAG => (s & 0xffff) as u32,
        _ => 10022,
    }
}
/// closesocket: pending operations on the socket complete with WSA_OPERATION_ABORTED.
pub(super) fn cancel(fd: i32) {
    let cancelled: Vec<usize> = {
        let mut list = PENDING.lock().unwrap();
        let mut out = Vec::new();
        list.retain(|p| {
            if p.fd == fd {
                out.push(p.overlapped);
                false
            } else {
                true
            }
        });
        out
    };
    if !cancelled.is_empty() {
        for overlapped in cancelled {
            Overlapped(overlapped).complete(STATUS_CANCELLED, 0, 0);
        }
        wake();
    }
}

/// WSAGetOverlappedResult. Waiting needs the operation's event; without one the call
/// fails with WSA_IO_INCOMPLETE while the operation is pending.
pub unsafe extern "win64" fn get_overlapped_result(
    _s: usize, overlapped: usize, bytes: *mut u32, wait: i32, flags: *mut u32,
) -> i32 {
    if overlapped == 0 || bytes.is_null() || flags.is_null() {
        error::set(10014);
        return 0;
    }
    let ov = Overlapped(overlapped);
    if ov.internal().load(Ordering::Acquire) == STATUS_PENDING {
        let event = ov.event();
        if wait == 0 || event == 0 {
            error::set(WSA_IO_INCOMPLETE);
            return 0;
        }
        while ov.internal().load(Ordering::Acquire) == STATUS_PENDING {
            win_events::wait(event, u32::MAX);
        }
    }
    let status = ov.internal().load(Ordering::Acquire);
    unsafe {
        *bytes = *((overlapped + 8) as *const usize) as u32;
        *flags = *((overlapped + 16) as *const u32);
    }
    if status == 0 {
        error::set(0);
        return 1;
    }
    error::set(status_error(status));
    0
}
