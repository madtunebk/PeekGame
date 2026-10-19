//! AFD (Ancillary Function Driver) socket poll device implementation.
//! Provides asynchronous socket event notification for Windows IOCP (used by mio / async-io / wepoll / Bevy).
use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    sync::{
        atomic::{AtomicI32, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, Once, OnceLock,
    },
};

unsafe extern "C" {
    fn eventfd(initval: u32, flags: i32) -> i32;
    fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    fn getsockopt(fd: i32, level: i32, name: i32, value: *mut c_void, length: *mut u32) -> i32;
    fn getpeername(fd: i32, address: *mut c_void, length: *mut u32) -> i32;
    fn poll(fds: *mut PollFd, count: u64, timeout: i32) -> i32;
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

const EFD_CLOEXEC: i32 = 0o2000000;
const EFD_NONBLOCK: i32 = 0o4000;

const POLLIN: i16 = 0x0001;
const POLLPRI: i16 = 0x0002;
const POLLOUT: i16 = 0x0004;
const POLLERR: i16 = 0x0008;
const POLLHUP: i16 = 0x0010;
const POLLNVAL: i16 = 0x0020;
const POLLRDHUP: i16 = 0x2000;

const SOL_SOCKET: i32 = 1;
const SO_ACCEPTCONN: i32 = 30;

pub const IOCTL_AFD_POLL: u32 = 0x00012024;

pub const AFD_RECEIVE: u32 = 0x1;
pub const AFD_RECEIVE_EXPEDITED: u32 = 0x2;
pub const AFD_SEND: u32 = 0x4;
pub const AFD_DISCONNECT: u32 = 0x8;
pub const AFD_ABORT: u32 = 0x10;
pub const AFD_LOCAL_CLOSE: u32 = 0x20;
pub const AFD_ACCEPT: u32 = 0x80;
pub const AFD_CONNECT_FAIL: u32 = 0x100;

pub const STATUS_SUCCESS: i32 = 0;
pub const STATUS_PENDING: i32 = 0x00000103;
pub const STATUS_NOT_IMPLEMENTED: i32 = 0xC0000002u32 as i32;
pub const STATUS_INVALID_HANDLE: i32 = 0xC0000008u32 as i32;
pub const STATUS_INVALID_PARAMETER: i32 = 0xC000000Du32 as i32;
pub const STATUS_CANCELLED: i32 = 0xC0000120u32 as i32;
pub const STATUS_NOT_FOUND: i32 = 0xC0000225u32 as i32;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AfdPollInfo {
    pub timeout: i64,
    pub count: u32,
    pub exclusive: u32,
    pub socket: u64,
    pub events: u32,
    pub status: i32,
}

#[derive(Clone, Debug)]
struct AfdRequest {
    id: u64,
    afd_handle: usize,
    iosb: usize,
    context: usize,
    info: usize,
    fd: i32,
    events: u32,
}

#[derive(Clone)]
struct AfdAssociation {
    port: Arc<crate::api::threading::iocp::Port>,
    key: usize,
    modes: u8,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(0x7200_0000);
static AFD_HANDLES: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();
static ASSOCIATIONS: OnceLock<Mutex<HashMap<usize, AfdAssociation>>> = OnceLock::new();
static REQUESTS: OnceLock<Mutex<Vec<AfdRequest>>> = OnceLock::new();
static WAKE_FD: AtomicI32 = AtomicI32::new(-1);
static POLLER_ONCE: Once = Once::new();

fn handles() -> &'static Mutex<HashSet<usize>> {
    AFD_HANDLES.get_or_init(|| Mutex::new(HashSet::new()))
}

fn associations() -> &'static Mutex<HashMap<usize, AfdAssociation>> {
    ASSOCIATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn requests() -> &'static Mutex<Vec<AfdRequest>> {
    REQUESTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn ensure_poller() {
    POLLER_ONCE.call_once(|| {
        let fd = unsafe { eventfd(0, EFD_CLOEXEC | EFD_NONBLOCK) };
        if fd < 0 {
            eprintln!("AFD: failed to create eventfd");
            return;
        }
        WAKE_FD.store(fd, Ordering::Relaxed);
        std::thread::Builder::new()
            .name("afd-poller".into())
            .spawn(poller_loop)
            .expect("failed to spawn afd poller thread");
    });
}

fn wake() {
    let wake_fd = WAKE_FD.load(Ordering::Relaxed);
    if wake_fd >= 0 {
        let one = 1u64;
        unsafe {
            write(wake_fd, &one as *const _ as *const c_void, std::mem::size_of::<u64>());
        }
    }
}

pub fn create() -> usize {
    ensure_poller();
    let handle = NEXT_HANDLE.fetch_add(16, Ordering::Relaxed);
    handles().lock().unwrap().insert(handle);
    handle
}

pub fn is_afd(handle: usize) -> bool {
    handles().lock().unwrap().contains(&handle)
}

pub fn associate(handle: usize, port: Arc<crate::api::threading::iocp::Port>, key: usize) {
    associations().lock().unwrap().insert(
        handle,
        AfdAssociation {
            port,
            key,
            modes: 0,
        },
    );
}

pub fn set_modes(handle: usize, modes: u8) {
    if let Some(assoc) = associations().lock().unwrap().get_mut(&handle) {
        assoc.modes = modes;
    }
}

pub fn close(handle: usize) -> bool {
    if !handles().lock().unwrap().remove(&handle) {
        return false;
    }
    associations().lock().unwrap().remove(&handle);
    let mut cancelled = Vec::new();
    {
        let mut reqs = requests().lock().unwrap();
        reqs.retain(|r| {
            if r.afd_handle == handle {
                cancelled.push(r.clone());
                false
            } else {
                true
            }
        });
    }
    for req in cancelled {
        complete_request(&req, STATUS_CANCELLED, 0);
    }
    wake();
    true
}

fn complete_request(req: &AfdRequest, status: i32, events: u32) {
    if status == 0 {
        let info = req.info as *mut AfdPollInfo;
        unsafe {
            (*info).count = 1;
            (*info).events = events;
            (*info).status = 0;
        }
    }
    let iosb = req.iosb as *mut crate::api::filesystem::nt_io::IoStatus;
    let info_len = if status == 0 {
        std::mem::size_of::<AfdPollInfo>()
    } else {
        0
    };
    unsafe {
        (*iosb).status = status as usize;
        (*iosb).information = info_len;
    }
    if let Some(assoc) = associations().lock().unwrap().get(&req.afd_handle) {
        assoc.port.post(crate::api::threading::iocp::Packet {
            key: assoc.key,
            overlapped: req.context,
            internal: status as usize,
            bytes: info_len as u32,
        });
    }
}

pub unsafe extern "win64" fn device_io_control(
    file: usize,
    event: usize,
    apc: usize,
    context: usize,
    io_status: *mut crate::api::filesystem::nt_io::IoStatus,
    code: u32,
    input: *mut c_void,
    input_len: u32,
    output: *mut c_void,
    output_len: u32,
) -> i32 {
    if !is_afd(file) {
        return STATUS_INVALID_HANDLE;
    }
    if code != IOCTL_AFD_POLL || event != 0 || apc != 0 {
        return STATUS_NOT_IMPLEMENTED;
    }
    if io_status.is_null()
        || input.is_null()
        || (input_len as usize) < std::mem::size_of::<AfdPollInfo>()
        || output != input
        || (output_len as usize) < std::mem::size_of::<AfdPollInfo>()
    {
        return STATUS_INVALID_PARAMETER;
    }
    let in_info = unsafe { &*input.cast::<AfdPollInfo>() };
    if in_info.count != 1 {
        return STATUS_NOT_IMPLEMENTED;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let req = AfdRequest {
        id,
        afd_handle: file,
        iosb: io_status as usize,
        context,
        info: input as usize,
        fd: in_info.socket as i32,
        events: in_info.events,
    };
    unsafe {
        (*io_status).status = STATUS_PENDING as usize;
        (*io_status).information = 0;
    }
    requests().lock().unwrap().push(req);
    wake();
    STATUS_PENDING
}

pub unsafe extern "win64" fn cancel_io(
    file_handle: usize,
    target_iosb: *const crate::api::filesystem::nt_io::IoStatus,
    result_iosb: *mut crate::api::filesystem::nt_io::IoStatus,
) -> i32 {
    let mut cancelled = Vec::new();
    {
        let mut reqs = requests().lock().unwrap();
        let target_addr = target_iosb as usize;
        reqs.retain(|r| {
            if r.afd_handle == file_handle && (target_addr == 0 || r.iosb == target_addr) {
                cancelled.push(r.clone());
                false
            } else {
                true
            }
        });
    }
    let found = !cancelled.is_empty();
    for req in cancelled {
        complete_request(&req, STATUS_CANCELLED, 0);
    }
    if found {
        wake();
    }
    let status = if found { STATUS_SUCCESS } else { STATUS_NOT_FOUND };
    if !result_iosb.is_null() {
        unsafe {
            (*result_iosb).status = status as usize;
            (*result_iosb).information = 0;
        }
    }
    status
}

fn poll_mask(events: u32) -> i16 {
    let mut m = 0i16;
    if events & (AFD_RECEIVE | AFD_ACCEPT) != 0 {
        m |= POLLIN;
    }
    if events & AFD_RECEIVE_EXPEDITED != 0 {
        m |= POLLPRI;
    }
    if events & (AFD_SEND | AFD_CONNECT_FAIL) != 0 {
        m |= POLLOUT;
    }
    if events & AFD_DISCONNECT != 0 {
        m |= POLLRDHUP;
    }
    m
}

fn ready_events(fd: i32, revents: i16) -> u32 {
    let mut ev = 0u32;
    let mut listening = 0i32;
    let mut len = std::mem::size_of::<i32>() as u32;
    unsafe {
        getsockopt(
            fd,
            SOL_SOCKET,
            SO_ACCEPTCONN,
            &mut listening as *mut _ as *mut c_void,
            &mut len,
        );
    }
    if revents & POLLIN != 0 {
        ev |= if listening != 0 { AFD_ACCEPT } else { AFD_RECEIVE };
    }
    if revents & POLLPRI != 0 {
        ev |= AFD_RECEIVE_EXPEDITED;
    }
    if revents & POLLOUT != 0 {
        ev |= AFD_SEND;
    }
    if revents & (POLLRDHUP | POLLHUP) != 0 {
        ev |= AFD_DISCONNECT;
    }
    if revents & POLLERR != 0 {
        let mut peer = [0u8; 128];
        let mut n = peer.len() as u32;
        ev |= if unsafe { getpeername(fd, peer.as_mut_ptr().cast(), &mut n) } != 0 {
            AFD_CONNECT_FAIL
        } else {
            AFD_ABORT
        };
    }
    if revents & POLLNVAL != 0 {
        ev |= AFD_LOCAL_CLOSE;
    }
    ev
}

fn poller_loop() {
    let wake_fd = WAKE_FD.load(Ordering::Relaxed);
    loop {
        let mut poll_fds = Vec::new();
        let mut owners = Vec::new();
        poll_fds.push(PollFd {
            fd: wake_fd,
            events: POLLIN,
            revents: 0,
        });
        owners.push(None);

        {
            let reqs = requests().lock().unwrap();
            for req in reqs.iter() {
                poll_fds.push(PollFd {
                    fd: req.fd,
                    events: poll_mask(req.events),
                    revents: 0,
                });
                owners.push(Some(req.id));
            }
        }

        let ret = unsafe { poll(poll_fds.as_mut_ptr(), poll_fds.len() as u64, -1) };
        if ret < 0 {
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if err == 4 { // EINTR
                continue;
            }
            break;
        }

        if poll_fds[0].revents & POLLIN != 0 {
            let mut drain = 0u64;
            unsafe {
                read(wake_fd, &mut drain as *mut _ as *mut c_void, std::mem::size_of::<u64>());
            }
        }

        let mut fired_ids = Vec::new();
        for i in 1..poll_fds.len() {
            let revents = poll_fds[i].revents;
            if revents == 0 {
                continue;
            }
            if let Some(id) = owners[i] {
                fired_ids.push((id, revents));
            }
        }

        let mut completed = Vec::new();
        if !fired_ids.is_empty() {
            let mut reqs = requests().lock().unwrap();
            for (id, revents) in fired_ids {
                if let Some(pos) = reqs.iter().position(|r| r.id == id) {
                    let req = &reqs[pos];
                    let ready = ready_events(req.fd, revents);
                    let wanted = ready & req.events;
                    if wanted == 0 && (revents & (POLLERR | POLLHUP | POLLNVAL)) == 0 {
                        continue;
                    }
                    let r = reqs.remove(pos);
                    let ev = if wanted != 0 { wanted } else { ready };
                    completed.push((r, 0, ev));
                }
            }
        }

        for (req, status, ev) in completed {
            complete_request(&req, status, ev);
        }
    }
}
