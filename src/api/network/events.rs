//! Winsock's event model (peek's wsa_events.inc): WSAEventSelect,
//! WSAEnumNetworkEvents and WSA event objects (manual-reset events). A poller
//! thread watches every socket registered with WSAEventSelect and, when a requested
//! condition occurs, records the network event (with its error code) and signals
//! the socket's event. As on Windows, each network event is reported once and then
//! waits for the call that re-enables it: recv/recvfrom/WSARecv for FD_READ, accept
//! for FD_ACCEPT, a send that would block for FD_WRITE; FD_CONNECT and FD_CLOSE are
//! reported once. Registered sockets become non-blocking, and an accepted socket
//! inherits the listening socket's registration.
use super::sockets::{
    is_socket, PollFd, FIONREAD, F_GETFL, F_SETFL, O_NONBLOCK, POLLERR, POLLHUP, POLLIN, POLLOUT,
    POLLRDHUP, SOL_SOCKET as SOL_SOCKET_LEVEL, SO_ACCEPTCONN as SOCKET_ACCEPTCONN,
    SO_ERROR as SOCKET_ERROR_OPTION, SO_TYPE as SOCKET_TYPE,
};
use super::{fail, wsa_error};
use crate::api::{
    system::error,
    threading::{events as win_events, objects},
};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    Mutex, Once,
};
const FD_READ: u32 = 0x01;
const FD_WRITE: u32 = 0x02;
const FD_OOB: u32 = 0x04;
const FD_ACCEPT: u32 = 0x08;
const FD_CONNECT: u32 = 0x10;
const FD_CLOSE: u32 = 0x20;
const FD_ALL: u32 = 0x3f;
#[derive(Clone, Copy, PartialEq)]
enum State {
    Fresh,
    Connecting,
    Connected,
    Listening,
    Closed,
}
struct Registration {
    fd: i32,
    event: usize,
    mask: u32,
    armed: u32,
    pending: u32,
    errors: [i32; 10],
    state: State,
}
static REGISTRATIONS: Mutex<Vec<Registration>> = Mutex::new(Vec::new());
static ANY: AtomicBool = AtomicBool::new(false);
static WAKE: AtomicI32 = AtomicI32::new(-1);
unsafe extern "C" {
    fn eventfd(initial: u32, flags: i32) -> i32;
    fn read(fd: i32, buffer: *mut std::ffi::c_void, length: usize) -> isize;
    fn write(fd: i32, buffer: *const std::ffi::c_void, length: usize) -> isize;
    fn poll(fds: *mut PollFd, count: u64, timeout: i32) -> i32;
    fn getsockopt(
        fd: i32,
        level: i32,
        name: i32,
        value: *mut std::ffi::c_void,
        length: *mut u32,
    ) -> i32;
    fn getpeername(fd: i32, address: *mut std::ffi::c_void, length: *mut u32) -> i32;
    fn fcntl(fd: i32, command: i32, ...) -> i32;
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}
fn option(fd: i32, name: i32) -> Option<i32> {
    let mut v = 0;
    let mut l = 4u32;
    (unsafe {
        getsockopt(
            fd,
            SOL_SOCKET_LEVEL,
            name,
            (&mut v as *mut i32).cast(),
            &mut l,
        )
    } == 0)
        .then_some(v)
}
fn wake() {
    let fd = WAKE.load(Ordering::Acquire);
    if fd >= 0 {
        let one = 1u64.to_le_bytes();
        // A full counter means a wake-up is pending anyway.
        let _ = unsafe { write(fd, one.as_ptr().cast(), 8) };
    }
}
fn state_of(fd: i32) -> State {
    if option(fd, SOCKET_ACCEPTCONN) == Some(1) {
        return State::Listening;
    }
    if option(fd, SOCKET_TYPE).is_some_and(|t| t != 1) {
        return State::Connected; // datagrams: always ready to use
    }
    let mut peer = [0u8; 128];
    let mut length = 128u32;
    if unsafe { getpeername(fd, peer.as_mut_ptr().cast(), &mut length) } == 0 {
        State::Connected
    } else {
        State::Fresh
    }
}
fn report(r: &mut Registration, fired: u32) {
    let fired = fired & r.mask;
    if fired == 0 {
        return;
    }
    r.pending |= fired;
    r.armed &= !fired;
    win_events::set(r.event);
}
fn bit(flag: u32) -> usize {
    flag.trailing_zeros() as usize
}
fn poller() {
    loop {
        let mut fds = vec![PollFd {
            fd: WAKE.load(Ordering::Acquire),
            events: POLLIN,
            revents: 0,
        }];
        {
            let list = REGISTRATIONS.lock().unwrap();
            for r in list.iter() {
                let mut events = 0;
                match r.state {
                    State::Listening if r.armed & FD_ACCEPT != 0 => events = POLLIN,
                    State::Connecting => events = POLLOUT,
                    State::Connected => {
                        if r.armed & (FD_READ | FD_OOB) != 0 {
                            events |= POLLIN;
                        }
                        if r.armed & FD_WRITE != 0 {
                            events |= POLLOUT;
                        }
                        if r.armed & FD_CLOSE != 0 {
                            events |= POLLRDHUP;
                        }
                    }
                    _ => {}
                }
                if events != 0 {
                    fds.push(PollFd {
                        fd: r.fd,
                        events,
                        revents: 0,
                    });
                }
            }
        }
        if unsafe { poll(fds.as_mut_ptr(), fds.len() as u64, -1) } < 0 {
            continue;
        }
        if fds[0].revents & POLLIN != 0 {
            let mut drain = [0u8; 8];
            let _ = unsafe { read(fds[0].fd, drain.as_mut_ptr().cast(), 8) };
        }
        let mut list = REGISTRATIONS.lock().unwrap();
        for p in &fds[1..] {
            if p.revents == 0 {
                continue;
            }
            let Some(r) = list.iter_mut().find(|r| r.fd == p.fd) else {
                continue; // unregistered meanwhile
            };
            let got = p.revents;
            let mut error = 0;
            if got & (POLLERR | POLLHUP) != 0 {
                error = option(r.fd, SOCKET_ERROR_OPTION).unwrap_or(0);
            }
            if r.state == State::Connecting {
                let error = option(r.fd, SOCKET_ERROR_OPTION).unwrap_or(0);
                r.errors[bit(FD_CONNECT)] = if error != 0 {
                    wsa_error(error) as i32
                } else {
                    0
                };
                r.state = if error != 0 {
                    State::Closed
                } else {
                    State::Connected
                };
                let write = if error != 0 { 0 } else { r.armed & FD_WRITE };
                report(r, FD_CONNECT | write);
                continue;
            }
            if r.state == State::Listening {
                if got & POLLIN != 0 {
                    report(r, FD_ACCEPT);
                }
                continue;
            }
            let mut fired = 0;
            if got & POLLIN != 0 && r.armed & FD_READ != 0 {
                let mut available = 0i32;
                if unsafe { ioctl(r.fd, FIONREAD, &mut available) } == 0 && available > 0 {
                    fired |= FD_READ;
                }
            }
            if got & POLLOUT != 0 && r.armed & FD_WRITE != 0 {
                fired |= FD_WRITE;
            }
            if got & (POLLRDHUP | POLLHUP | POLLERR) != 0 {
                if r.armed & FD_CLOSE != 0 {
                    r.errors[bit(FD_CLOSE)] = if error != 0 {
                        wsa_error(error) as i32
                    } else {
                        0
                    };
                    fired |= FD_CLOSE;
                }
                r.state = State::Closed; // a hang-up stays readable for ever: stop watching
            }
            report(r, fired);
        }
    }
}
/// What a socket call did, for the registrations.
pub(crate) enum Note {
    Receive,
    Accept(i32),
    SendBlocked,
    Connect,
    Listen,
    Close,
}
pub(crate) fn note(fd: i32, what: Note) {
    if !ANY.load(Ordering::Acquire) {
        return; // nothing registered: the common case costs one load
    }
    let mut list = REGISTRATIONS.lock().unwrap();
    let Some(index) = list.iter().position(|r| r.fd == fd) else {
        return;
    };
    match what {
        Note::Receive => {
            let r = &mut list[index];
            r.armed |= r.mask & (FD_READ | FD_OOB);
        }
        Note::Accept(child) => {
            let r = &mut list[index];
            r.armed |= r.mask & FD_ACCEPT;
            if child >= 0 {
                // The accepted socket gets the listening socket's event and network events.
                let (event, mask) = (r.event, r.mask);
                list.push(Registration {
                    fd: child,
                    event,
                    mask,
                    armed: mask,
                    pending: 0,
                    errors: [0; 10],
                    state: State::Connected,
                });
            }
        }
        Note::SendBlocked => {
            let r = &mut list[index];
            r.armed |= r.mask & FD_WRITE;
        }
        Note::Connect => list[index].state = State::Connecting,
        Note::Listen => list[index].state = State::Listening,
        Note::Close => {
            list.remove(index);
        }
    }
    drop(list);
    wake();
}
/// WSAEventSelect(s, hEventObject, lNetworkEvents); 0 events cancels.
pub extern "win64" fn event_select(s: usize, event: usize, events: i32) -> i32 {
    let fd = s as i32;
    let flags = unsafe { fcntl(fd, F_GETFL) };
    if flags < 0 || !is_socket(s) {
        error::set(10038);
        return -1;
    }
    let events = events as u32;
    if events & !FD_ALL != 0 {
        eprintln!("WSAEventSelect: network events {events:#x} are not supported");
        error::set(10022);
        return -1;
    }
    if events == 0 {
        REGISTRATIONS.lock().unwrap().retain(|r| r.fd != fd);
        wake();
        return 0;
    }
    if win_events::check(event, false).is_none() {
        error::set(10022);
        return -1;
    }
    if unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } != 0 {
        return fail();
    }
    static START: Once = Once::new();
    START.call_once(|| {
        let fd = unsafe { eventfd(0, 0x80000 | 0x800) };
        WAKE.store(fd, Ordering::Release);
        let _ = std::thread::Builder::new()
            .name("wsa-events".into())
            .spawn(poller);
    });
    let state = state_of(fd);
    let mut list = REGISTRATIONS.lock().unwrap();
    list.retain(|r| r.fd != fd);
    list.push(Registration {
        fd,
        event,
        mask: events,
        armed: events,
        pending: 0,
        errors: [0; 10],
        state,
    });
    ANY.store(true, Ordering::Release);
    drop(list);
    wake();
    0
}
/// WSANETWORKEVENTS: lNetworkEvents, iErrorCode[10] (FD_*_BIT order). Clears what it
/// returns and, with hEventObject, resets that event.
pub unsafe extern "win64" fn enum_network_events(s: usize, event: usize, out: *mut u8) -> i32 {
    if out.is_null() {
        error::set(10014);
        return -1;
    }
    if !is_socket(s) {
        error::set(10038);
        return -1;
    }
    let (pending, errors) = {
        let mut list = REGISTRATIONS.lock().unwrap();
        match list.iter_mut().find(|r| r.fd == s as i32) {
            Some(r) => {
                let result = (r.pending, r.errors);
                r.pending = 0;
                r.errors = [0; 10];
                result
            }
            None => (0, [0; 10]),
        }
    };
    if event != 0 && win_events::reset(event) == 0 {
        error::set(10022);
        return -1;
    }
    unsafe {
        out.cast::<u32>().write_unaligned(pending);
        std::ptr::copy_nonoverlapping(errors.as_ptr().cast::<u8>(), out.add(4), 40);
    }
    0
}
/// WSA event objects are manual-reset events; WSA_INVALID_EVENT is NULL.
pub extern "win64" fn create_event() -> usize {
    let e = unsafe { win_events::create_w(std::ptr::null(), 1, 0, std::ptr::null()) };
    if e == 0 {
        error::set(8);
    }
    e
}
pub extern "win64" fn close_event(event: usize) -> i32 {
    if win_events::check(event, false).is_none() {
        error::set(6);
        return 0;
    }
    win_events::close(event)
}
pub unsafe extern "win64" fn wait_for_multiple_events(
    count: u32,
    events: *const usize,
    all: i32,
    timeout: u32,
    alertable: i32,
) -> u32 {
    unsafe { objects::wait_multiple_ex(count, events, all, timeout, alertable) }
}
