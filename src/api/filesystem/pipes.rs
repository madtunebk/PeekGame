//! Byte-stream named pipe clients backed by Unix sockets. The socket name lives
//! in PEEKGAME_PIPE_DIR (or XDG_RUNTIME_DIR). No protocol or peer is emulated.
use crate::api::{
    system::error,
    threading::{events, iocp::Binding},
};
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    time::Duration,
};
const PENDING: usize = 0x103;
const CANCELLED: usize = 0xc0000120;
struct Request {
    buffer: usize,
    len: u32,
    ov: usize,
    writing: bool,
    owner: u32,
}
struct State {
    closed: bool,
    requests: VecDeque<Request>,
    binding: Option<Binding>,
    modes: u8,
}
struct Pipe {
    stream: UnixStream,
    overlapped: bool,
    state: Mutex<State>,
    changed: Condvar,
    handles: AtomicUsize,
}
static TABLE: OnceLock<Mutex<HashMap<usize, Arc<Pipe>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x6a000000);
fn table() -> &'static Mutex<HashMap<usize, Arc<Pipe>>> {
    TABLE.get_or_init(|| Mutex::new(HashMap::new()))
}
fn get(handle: usize) -> Option<Arc<Pipe>> {
    table().lock().unwrap().get(&handle).cloned()
}
fn fail(code: u32) -> i32 {
    error::set(code);
    0
}
use crate::api::network::sockets::PollFd;
unsafe extern "C" {
    fn poll(fds: *mut PollFd, n: u64, timeout: i32) -> i32;
}
/// Recognizes only the local Win32 pipe namespace; other paths are untouched.
pub fn open(name: &str, access: u32, disposition: u32, flags: u32) -> Option<usize> {
    let normalized = name.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    if !lower.starts_with("//./pipe/") && !lower.starts_with("//?/pipe/") {
        return None;
    }
    let path = &normalized[9..];
    let opened = (|| -> Result<usize, u32> {
        if path.is_empty() || path.contains('/') || path == "." || path == ".." || disposition != 3
        {
            return Err(87);
        }
        if access & 0xc0000000 != 0xc0000000 || flags & !0x40000080 != 0 {
            return Err(50);
        }
        let directory = std::env::var_os("PEEKGAME_PIPE_DIR")
            .or_else(|| std::env::var_os("XDG_RUNTIME_DIR"))
            .ok_or(2u32)?;
        let stream =
            UnixStream::connect(PathBuf::from(directory).join(path)).map_err(|e| {
                match e.raw_os_error() {
                    Some(2 | 111) => 2u32,
                    Some(13) => 5,
                    _ => 50,
                }
            })?;
        stream.set_nonblocking(true).map_err(|_| 50u32)?;
        let pipe = Arc::new(Pipe {
            stream,
            overlapped: flags & 0x40000000 != 0,
            state: Mutex::new(State {
                closed: false,
                requests: VecDeque::new(),
                binding: None,
                modes: 0,
            }),
            changed: Condvar::new(),
            handles: AtomicUsize::new(1),
        });
        let worker = pipe.clone();
        std::thread::Builder::new()
            .name("peek-pipe".into())
            .spawn(move || run(worker))
            .map_err(|_| 8u32)?;
        let handle = NEXT.fetch_add(16, Ordering::Relaxed);
        table().lock().unwrap().insert(handle, pipe);
        Ok(handle)
    })();
    Some(match opened {
        Ok(handle) => handle,
        Err(code) => {
            error::set(code);
            usize::MAX
        }
    })
}
pub fn duplicate(handle: usize) -> Option<usize> {
    let mut table = table().lock().unwrap();
    let pipe = table.get(&handle)?.clone();
    pipe.handles.fetch_add(1, Ordering::Relaxed);
    let target = NEXT.fetch_add(16, Ordering::Relaxed);
    table.insert(target, pipe);
    Some(target)
}
pub fn close(handle: usize) -> bool {
    let Some(pipe) = table().lock().unwrap().remove(&handle) else {
        return false;
    };
    if pipe.handles.fetch_sub(1, Ordering::AcqRel) == 1 {
        let mut state = pipe.state.lock().unwrap();
        state.closed = true;
        while let Some(request) = state.requests.pop_front() {
            complete(&state, &request, CANCELLED, 0);
        }
        pipe.changed.notify_all();
    }
    true
}
pub fn known(handle: usize) -> bool {
    get(handle).is_some()
}
pub fn associate(handle: usize, binding: Binding) -> Option<Result<(), u32>> {
    let pipe = get(handle)?;
    let mut state = pipe.state.lock().unwrap();
    Some(if !pipe.overlapped {
        Err(87)
    } else if let Some(old) = state.binding.as_ref() {
        if old.same_port(&binding) {
            Ok(())
        } else {
            Err(87)
        }
    } else {
        state.binding = Some(binding);
        Ok(())
    })
}
pub fn modes(handle: usize, modes: u8) -> Option<Result<(), u32>> {
    let pipe = get(handle)?;
    if !pipe.overlapped {
        return Some(Err(87));
    }
    pipe.state.lock().unwrap().modes |= modes;
    Some(Ok(()))
}
fn complete(state: &State, request: &Request, status: usize, bytes: u32) {
    complete_with(state, request, status, bytes, false);
}
fn complete_with(state: &State, request: &Request, status: usize, bytes: u32, immediate: bool) {
    unsafe {
        let ov = request.ov as *mut usize;
        ov.add(1).write(bytes as usize);
        AtomicUsize::from_ptr(ov).store(status, Ordering::Release);
        let event = ov.add(3).read_unaligned();
        if event & !1 != 0 {
            events::set(event & !1);
        }
        if event & 1 == 0 && !(immediate && status == 0 && state.modes & 1 != 0) {
            if let Some(binding) = state.binding.as_ref() {
                binding.complete(request.ov, status, bytes);
            }
        }
    }
}
fn transfer(pipe: &Pipe, request: &Request) -> Result<usize, std::io::Error> {
    if request.len == 0 {
        return Ok(0);
    }
    if request.writing {
        (&pipe.stream).write(unsafe {
            std::slice::from_raw_parts(request.buffer as *const u8, request.len as usize)
        })
    } else {
        (&pipe.stream).read(unsafe {
            std::slice::from_raw_parts_mut(request.buffer as *mut u8, request.len as usize)
        })
    }
}
fn run(pipe: Arc<Pipe>) {
    loop {
        let mut state = pipe.state.lock().unwrap();
        while state.requests.is_empty() && !state.closed {
            state = pipe.changed.wait(state).unwrap();
        }
        if state.closed {
            return;
        }
        let mask = state
            .requests
            .iter()
            .fold(0, |mask, r| mask | if r.writing { 4 } else { 1 });
        drop(state);
        let mut fd = PollFd {
            fd: pipe.stream.as_raw_fd(),
            events: mask,
            revents: 0,
        };
        let result = unsafe { poll(&mut fd, 1, 20) };
        let mut state = pipe.state.lock().unwrap();
        if state.closed {
            return;
        }
        let mut index = 0;
        while index < state.requests.len() {
            let request = &state.requests[index];
            if result >= 0 && fd.revents & (if request.writing { 4 } else { 1 } | 8 | 16 | 32) == 0
            {
                index += 1;
                continue;
            }
            let completed = match transfer(&pipe, request) {
                Ok(0) if request.len > 0 && !request.writing => Some((0xc000014b, 0)),
                Ok(n) => Some((0, n as u32)),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                Err(_) => Some((0xc000014b, 0)),
            };
            if let Some((status, bytes)) = completed {
                let request = state.requests.remove(index).unwrap();
                complete(&state, &request, status, bytes);
            } else {
                index += 1;
            }
        }
    }
}
/// Pending buffers belong to the caller until event/IOCP completion or cancellation.
pub unsafe fn io(
    handle: usize,
    buffer: *mut u8,
    len: u32,
    count: *mut u32,
    ov: *mut u8,
    writing: bool,
) -> Option<i32> {
    let pipe = get(handle)?;
    if !count.is_null() {
        unsafe {
            *count = 0;
        }
    }
    if buffer.is_null() && len != 0 {
        return Some(fail(87));
    }
    if !ov.is_null() {
        if !pipe.overlapped {
            return Some(fail(87));
        }
        let mut state = pipe.state.lock().unwrap();
        if state.closed {
            return Some(fail(6));
        }
        if state.requests.iter().any(|r| r.ov == ov as usize) {
            return Some(fail(87));
        }
        unsafe {
            AtomicUsize::from_ptr(ov.cast::<usize>()).store(PENDING, Ordering::Release);
            ov.add(8).cast::<usize>().write_volatile(0);
            let event = ov.add(24).cast::<usize>().read_unaligned() & !1;
            if event != 0 {
                events::reset(event);
            }
        }
        let request = Request {
            buffer: buffer as usize,
            len,
            ov: ov as usize,
            writing,
            owner: crate::api::system::startup::thread_id(),
        };
        if len == 0 {
            complete_with(&state, &request, 0, 0, true);
            return Some(1);
        }
        state.requests.push_back(request);
        pipe.changed.notify_one();
        error::set(997);
        return Some(0);
    }
    if pipe.overlapped || count.is_null() {
        return Some(fail(87));
    }
    crate::api::threading::iocp::release_worker();
    let request = Request {
        buffer: buffer as usize,
        len,
        ov: 0,
        writing,
        owner: crate::api::system::startup::thread_id(),
    };
    loop {
        let state = pipe.state.lock().unwrap();
        if state.closed {
            return Some(fail(6));
        }
        match transfer(&pipe, &request) {
            Ok(0) if len > 0 && !writing => return Some(fail(109)),
            Ok(n) => {
                unsafe {
                    *count = n as u32;
                }
                return Some(1);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Some(fail(109)),
        }
        drop(state);
        std::thread::sleep(Duration::from_millis(2));
    }
}
pub fn cancel(handle: usize, ov: usize) -> Option<i32> {
    cancel_requests(handle, ov, false)
}
pub fn cancel_current(handle: usize) -> Option<i32> {
    cancel_requests(handle, 0, true)
}
fn cancel_requests(handle: usize, ov: usize, current_only: bool) -> Option<i32> {
    let pipe = get(handle)?;
    let mut state = pipe.state.lock().unwrap();
    let mut found = false;
    let mut index = 0;
    while index < state.requests.len() {
        if (ov == 0 || state.requests[index].ov == ov)
            && (!current_only
                || state.requests[index].owner == crate::api::system::startup::thread_id())
        {
            let request = state.requests.remove(index).unwrap();
            complete(&state, &request, CANCELLED, 0);
            found = true;
        } else {
            index += 1;
        }
    }
    Some(if found || current_only { 1 } else { fail(1168) })
}
pub unsafe extern "win64" fn set_state(
    handle: usize,
    mode: *const u32,
    collection: *const u32,
    timeout: *const u32,
) -> i32 {
    if !known(handle) {
        return fail(6);
    }
    if !collection.is_null() || !timeout.is_null() || (!mode.is_null() && unsafe { *mode } != 0) {
        return fail(50);
    }
    1
}

pub unsafe fn result(handle: usize, ov: *const u8, transferred: *mut u32, wait: i32) -> i32 {
    if !known(handle) {
        return fail(6);
    }
    if ov.is_null() || transferred.is_null() {
        return fail(87);
    }
    if wait != 0 {
        crate::api::threading::iocp::release_worker();
    }
    loop {
        let status = unsafe { AtomicUsize::from_ptr(ov as *mut usize) }.load(Ordering::Acquire);
        if status != PENDING {
            unsafe {
                *transferred = ov.add(8).cast::<usize>().read() as u32;
            }
            return match status as u32 {
                0 => 1,
                0xc0000120 => fail(995),
                0xc000014b => fail(109),
                _ => fail(31),
            };
        }
        if wait == 0 {
            return fail(996);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
