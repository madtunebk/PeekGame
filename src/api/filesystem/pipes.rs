//! Byte-stream named pipes (servers and clients) backed by Unix domain sockets.
//! The socket name lives in PEEKGAME_PIPE_DIR (or XDG_RUNTIME_DIR, fallback /tmp).
use crate::api::{
    system::error,
    text,
    threading::{events, iocp::Binding},
};
use std::{
    collections::{HashMap, VecDeque},
    ffi::{c_char, CStr},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::net::{UnixListener, UnixStream},
    },
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex, OnceLock, Weak,
    },
    time::Duration,
};
const PENDING: usize = 0x103;
const CANCELLED: usize = 0xc0000120;
const ERROR_PIPE_CONNECTED: u32 = 535;
const ERROR_IO_PENDING: u32 = 997;

struct BoundListener {
    listener: UnixListener,
    path: PathBuf,
}
impl Drop for BoundListener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

struct Request {
    buffer: usize,
    len: u32,
    ov: usize,
    writing: bool,
    connecting: bool,
    owner: u32,
}
struct State {
    closed: bool,
    stream: Option<UnixStream>,
    requests: VecDeque<Request>,
    binding: Option<Binding>,
    modes: u8,
}
struct Pipe {
    listener: Option<Arc<BoundListener>>,
    overlapped: bool,
    state: Mutex<State>,
    changed: Condvar,
    handles: AtomicUsize,
}
static TABLE: OnceLock<Mutex<HashMap<usize, Arc<Pipe>>>> = OnceLock::new();
static LISTENERS: OnceLock<Mutex<HashMap<PathBuf, Weak<BoundListener>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x6a000000);

fn table() -> &'static Mutex<HashMap<usize, Arc<Pipe>>> {
    TABLE.get_or_init(|| {
        unsafe { prctl(0x59616d61, usize::MAX, 0, 0, 0) };
        let base_h = 0x6a00_0000usize + ((std::process::id() as usize & 0xfff) << 12);
        NEXT.fetch_max(base_h, Ordering::Relaxed);
        let mut map = HashMap::new();
        if let Ok(inherited) = std::env::var("PEEKGAME_INHERITED_PIPES") {
            let mut max_h = base_h;
            for item in inherited.split(',') {
                let mut parts = item.split(':');
                if let (Some(h_s), Some(fd_s), Some(ov_s)) =
                    (parts.next(), parts.next(), parts.next())
                {
                    if let (Ok(h), Ok(fd), Ok(ov)) = (
                        usize::from_str_radix(h_s, 16),
                        fd_s.parse::<i32>(),
                        ov_s.parse::<u8>(),
                    ) {
                        let duped = unsafe { dup(fd) };
                        if duped >= 0 {
                            use std::os::fd::FromRawFd;
                            let stream = unsafe { UnixStream::from_raw_fd(duped) };
                            let _ = stream.set_nonblocking(true);
                            let pipe = Arc::new(Pipe {
                                listener: None,
                                overlapped: ov != 0,
                                state: Mutex::new(State {
                                    closed: false,
                                    stream: Some(stream),
                                    requests: VecDeque::new(),
                                    binding: None,
                                    modes: 0,
                                }),
                                changed: Condvar::new(),
                                handles: AtomicUsize::new(1),
                            });
                            let worker = pipe.clone();
                            let _ = std::thread::Builder::new()
                                .name("peek-pipe".into())
                                .spawn(move || run(worker));
                            map.insert(h, pipe);
                            if h >= max_h {
                                max_h = h + 16;
                            }
                        }
                    }
                }
            }
            NEXT.fetch_max(max_h, Ordering::Relaxed);
        }
        Mutex::new(map)
    })
}
pub fn export_inherited() -> String {
    let map = table().lock().unwrap();
    let mut out = Vec::new();
    for (&h, pipe) in map.iter() {
        let state = pipe.state.lock().unwrap();
        if let Some(stream) = state.stream.as_ref() {
            let fd = stream.as_raw_fd();
            unsafe { fcntl(fd, 2, 0) }; // F_SETFD = 2, clear FD_CLOEXEC
            out.push(format!("{h:x}:{fd}:{}", pipe.overlapped as u8));
        }
    }
    out.join(",")
}
fn listeners() -> &'static Mutex<HashMap<PathBuf, Weak<BoundListener>>> {
    LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn get(handle: usize) -> Option<Arc<Pipe>> {
    if let Some(p) = table().lock().unwrap().get(&handle).cloned() {
        return Some(p);
    }
    if !(0x6a00_0000..0x6c00_0000).contains(&handle) {
        return None;
    }
    let text = std::fs::read_to_string("/tmp/peekgame_handles.tsv").ok()?;
    let prefix = format!("P\t{handle:x}\t");
    for line in text.lines().rev() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            let mut parts = rest.split('\t');
            let pid = parts.next()?.parse::<i32>().ok()?;
            let remote_fd = parts.next()?.parse::<i32>().ok()?;
            let ov = parts.next()?.parse::<u8>().ok()?;
            let pidfd = unsafe { syscall(434, pid, 0usize) } as i32;
            if pidfd < 0 {
                continue;
            }
            let local_fd = unsafe { syscall(438, pidfd, remote_fd, 0usize) } as i32;
            unsafe { close_fd(pidfd) };
            if local_fd >= 0 {
                use std::os::fd::FromRawFd;
                let stream = unsafe { UnixStream::from_raw_fd(local_fd) };
                let _ = stream.set_nonblocking(true);
                let pipe = Arc::new(Pipe {
                    listener: None,
                    overlapped: ov != 0,
                    state: Mutex::new(State {
                        closed: false,
                        stream: Some(stream),
                        requests: VecDeque::new(),
                        binding: None,
                        modes: 0,
                    }),
                    changed: Condvar::new(),
                    handles: AtomicUsize::new(1),
                });
                let worker = pipe.clone();
                let _ = std::thread::Builder::new()
                    .name("peek-pipe".into())
                    .spawn(move || run(worker));
                table().lock().unwrap().insert(handle, pipe.clone());
                return Some(pipe);
            }
        }
    }
    None
}
fn fail(code: u32) -> i32 {
    error::set(code);
    0
}
use crate::api::network::sockets::PollFd;
unsafe extern "C" {
    fn dup(fd: i32) -> i32;
    #[link_name = "close"]
    fn close_fd(fd: i32) -> i32;
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    fn poll(fds: *mut PollFd, n: u64, timeout: i32) -> i32;
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn prctl(option: i32, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32;
    fn syscall(num: i64, ...) -> i64;
}

fn pipe_socket_path(name: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();
    if !lower.starts_with("//./pipe/") && !lower.starts_with("//?/pipe/") {
        return None;
    }
    let subpath = &normalized[9..];
    if subpath.is_empty() || subpath == "." || subpath == ".." {
        return None;
    }
    let sanitized = subpath.replace('/', "_");
    let directory = std::env::var_os("PEEKGAME_PIPE_DIR")
        .or_else(|| std::env::var_os("XDG_RUNTIME_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    // Keep Unix socket path within SUN_LEN (< 108 bytes).
    let leaf = if sanitized.len() > 64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        sanitized.hash(&mut h);
        format!("pg_{:016x}", h.finish())
    } else {
        sanitized
    };
    Some(directory.join(leaf))
}

fn create_named_pipe(name: &str, open_mode: u32) -> usize {
    let Some(socket_path) = pipe_socket_path(name) else {
        error::set(123); // ERROR_INVALID_NAME
        return usize::MAX;
    };
    let mut map = listeners().lock().unwrap();
    let existing = map.get(&socket_path).and_then(Weak::upgrade);
    let bound = if let Some(b) = existing {
        // FILE_FLAG_FIRST_PIPE_INSTANCE = 0x00080000
        if open_mode & 0x0008_0000 != 0 {
            error::set(5); // ERROR_ACCESS_DENIED
            return usize::MAX;
        }
        b
    } else {
        let _ = std::fs::remove_file(&socket_path);
        let Ok(listener) = UnixListener::bind(&socket_path) else {
            error::set(5);
            return usize::MAX;
        };
        let _ = listener.set_nonblocking(true);
        let b = Arc::new(BoundListener {
            listener,
            path: socket_path.clone(),
        });
        map.insert(socket_path, Arc::downgrade(&b));
        b
    };
    drop(map);

    let pipe = Arc::new(Pipe {
        listener: Some(bound),
        overlapped: open_mode & 0x4000_0000 != 0,
        state: Mutex::new(State {
            closed: false,
            stream: None,
            requests: VecDeque::new(),
            binding: None,
            modes: 0,
        }),
        changed: Condvar::new(),
        handles: AtomicUsize::new(1),
    });
    let worker = pipe.clone();
    if std::thread::Builder::new()
        .name("peek-pipe".into())
        .spawn(move || run(worker))
        .is_err()
    {
        error::set(8);
        return usize::MAX;
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    table().lock().unwrap().insert(handle, pipe);
    error::set(0);
    handle
}

pub unsafe extern "win64" fn create_named_pipe_w(
    name: *const u16,
    open_mode: u32,
    _pipe_mode: u32,
    _max_instances: u32,
    _out_buffer_size: u32,
    _in_buffer_size: u32,
    _default_timeout: u32,
    _security: usize,
) -> usize {
    let Some(name_str) = (unsafe { text::string(name) }) else {
        error::set(123);
        return usize::MAX;
    };
    create_named_pipe(&name_str, open_mode)
}

pub unsafe extern "win64" fn create_named_pipe_a(
    name: *const c_char,
    open_mode: u32,
    _pipe_mode: u32,
    _max_instances: u32,
    _out_buffer_size: u32,
    _in_buffer_size: u32,
    _default_timeout: u32,
    _security: usize,
) -> usize {
    if name.is_null() {
        error::set(123);
        return usize::MAX;
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    create_named_pipe(&name_str, open_mode)
}

pub unsafe extern "win64" fn connect_named_pipe(handle: usize, ov: *mut u8) -> i32 {
    let Some(pipe) = get(handle) else {
        return fail(6);
    };
    let mut state = pipe.state.lock().unwrap();
    if state.closed {
        return fail(6);
    }
    if state.stream.is_none() {
        if let Some(bound) = pipe.listener.as_ref() {
            if let Ok((stream, _)) = bound.listener.accept() {
                let _ = stream.set_nonblocking(true);
                state.stream = Some(stream);
            }
        }
    }
    if !ov.is_null() {
        if !pipe.overlapped {
            return fail(87);
        }
        if state.stream.is_some() {
            error::set(ERROR_PIPE_CONNECTED);
            return 0;
        }
        unsafe {
            AtomicUsize::from_ptr(ov.cast::<usize>()).store(PENDING, Ordering::Release);
            ov.add(8).cast::<usize>().write_volatile(0);
            let event = ov.add(24).cast::<usize>().read_unaligned() & !1;
            if event != 0 {
                events::reset(event);
            }
        }
        state.requests.push_back(Request {
            buffer: 0,
            len: 0,
            ov: ov as usize,
            writing: false,
            connecting: true,
            owner: crate::api::system::startup::thread_id(),
        });
        pipe.changed.notify_one();
        error::set(ERROR_IO_PENDING);
        return 0;
    }
    if state.stream.is_some() {
        error::set(ERROR_PIPE_CONNECTED);
        return 0;
    }
    drop(state);
    crate::api::threading::iocp::release_worker();
    loop {
        let mut state = pipe.state.lock().unwrap();
        if state.closed {
            return fail(6);
        }
        if state.stream.is_some() {
            error::set(0);
            return 1;
        }
        if let Some(bound) = pipe.listener.as_ref() {
            match bound.listener.accept() {
                Ok((stream, _)) => {
                    let _ = stream.set_nonblocking(true);
                    state.stream = Some(stream);
                    error::set(0);
                    return 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return fail(109),
            }
        } else {
            return fail(6);
        }
        drop(state);
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub extern "win64" fn disconnect_named_pipe(handle: usize) -> i32 {
    let Some(pipe) = get(handle) else {
        return fail(6);
    };
    let mut state = pipe.state.lock().unwrap();
    state.stream = None;
    while let Some(request) = state.requests.pop_front() {
        complete(&state, &request, CANCELLED, 0);
    }
    pipe.changed.notify_all();
    1
}

pub unsafe extern "win64" fn wait_named_pipe_w(name: *const u16, _timeout: u32) -> i32 {
    let Some(name_str) = (unsafe { text::string(name) }) else {
        return fail(123);
    };
    let Some(path) = pipe_socket_path(&name_str) else {
        return fail(123);
    };
    if path.exists() {
        1
    } else {
        fail(2) // ERROR_FILE_NOT_FOUND
    }
}

pub unsafe extern "win64" fn wait_named_pipe_a(name: *const c_char, _timeout: u32) -> i32 {
    if name.is_null() {
        return fail(123);
    }
    let name_str = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    let Some(path) = pipe_socket_path(&name_str) else {
        return fail(123);
    };
    if path.exists() {
        1
    } else {
        fail(2)
    }
}

pub unsafe extern "win64" fn get_pipe_process_id(handle: usize, pid_out: *mut u32) -> i32 {
    if !known(handle) {
        return fail(6);
    }
    if pid_out.is_null() {
        return fail(87);
    }
    unsafe { *pid_out = std::process::id() };
    1
}

/// Recognizes only the local Win32 pipe namespace; other paths are untouched.
pub fn open(name: &str, _access: u32, disposition: u32, flags: u32) -> Option<usize> {
    let socket_path = pipe_socket_path(name)?;
    let opened = (|| -> Result<usize, u32> {
        if disposition != 3 {
            return Err(87);
        }
        let stream = UnixStream::connect(&socket_path).map_err(|e| match e.raw_os_error() {
            Some(2 | 111) => 2u32,
            Some(13) => 5,
            _ => 50,
        })?;
        stream.set_nonblocking(true).map_err(|_| 50u32)?;
        let pipe = Arc::new(Pipe {
            listener: None,
            overlapped: flags & 0x40000000 != 0,
            state: Mutex::new(State {
                closed: false,
                stream: Some(stream),
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
    let pipe = get(handle)?;
    pipe.handles.fetch_add(1, Ordering::Relaxed);
    let target = NEXT.fetch_add(16, Ordering::Relaxed);
    table().lock().unwrap().insert(target, pipe.clone());
    if let Some(stream) = pipe.state.lock().unwrap().stream.as_ref() {
        let export_fd = unsafe { dup(stream.as_raw_fd()) };
        if export_fd >= 0 {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/peekgame_handles.tsv")
            {
                let _ = writeln!(
                    f,
                    "P\t{target:x}\t{}\t{export_fd}\t{}",
                    std::process::id(),
                    pipe.overlapped as u8
                );
            }
        }
    }
    Some(target)
}

pub fn close(handle: usize) -> bool {
    let Some(pipe) = table().lock().unwrap().remove(&handle) else {
        return false;
    };
    if pipe.handles.fetch_sub(1, Ordering::AcqRel) == 1 {
        let mut state = pipe.state.lock().unwrap();
        state.closed = true;
        state.stream = None;
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

pub unsafe fn peek(
    handle: usize,
    _buffer: usize,
    _size: u32,
    read: *mut u32,
    available: *mut u32,
    left: *mut u32,
) -> Option<i32> {
    let pipe = get(handle)?;
    let state = pipe.state.lock().unwrap();
    let Some(stream) = state.stream.as_ref() else {
        return Some(fail(230)); // ERROR_BAD_PIPE
    };
    let mut n: i32 = 0;
    if unsafe { ioctl(stream.as_raw_fd(), 0x541B, &mut n) } != 0 {
        return Some(fail(109));
    }
    unsafe {
        for (p, v) in [(read, 0), (available, n.max(0) as u32), (left, 0)] {
            if !p.is_null() {
                *p = v;
            }
        }
    }
    Some(1)
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

fn transfer(stream: &UnixStream, request: &Request) -> Result<usize, std::io::Error> {
    if request.len == 0 {
        return Ok(0);
    }
    if request.writing {
        (&*stream).write(unsafe {
            std::slice::from_raw_parts(request.buffer as *const u8, request.len as usize)
        })
    } else {
        (&*stream).read(unsafe {
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
        if state.stream.is_none() {
            let Some(bound) = pipe.listener.clone() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            drop(state);
            let mut fd = PollFd {
                fd: bound.listener.as_raw_fd(),
                events: 1,
                revents: 0,
            };
            let _ = unsafe { poll(&mut fd, 1, 20) };
            let mut state = pipe.state.lock().unwrap();
            if state.closed {
                return;
            }
            if state.stream.is_none() {
                if let Ok((stream, _)) = bound.listener.accept() {
                    let _ = stream.set_nonblocking(true);
                    state.stream = Some(stream);
                }
            }
            if state.stream.is_some() {
                let mut i = 0;
                while i < state.requests.len() {
                    if state.requests[i].connecting {
                        let req = state.requests.remove(i).unwrap();
                        complete(&state, &req, 0, 0);
                    } else {
                        i += 1;
                    }
                }
            }
            continue;
        }

        // Complete any leftover connecting requests if stream is already connected.
        let mut i = 0;
        while i < state.requests.len() {
            if state.requests[i].connecting {
                let req = state.requests.remove(i).unwrap();
                complete(&state, &req, 0, 0);
            } else {
                i += 1;
            }
        }
        if state.requests.is_empty() {
            continue;
        }

        let raw_fd = state.stream.as_ref().unwrap().as_raw_fd();
        let mask = state
            .requests
            .iter()
            .fold(0, |mask, r| mask | if r.writing { 4 } else { 1 });
        drop(state);
        let mut fd = PollFd {
            fd: raw_fd,
            events: mask,
            revents: 0,
        };
        let result = unsafe { poll(&mut fd, 1, 20) };
        let mut state = pipe.state.lock().unwrap();
        if state.closed || state.stream.is_none() {
            continue;
        }
        let mut index = 0;
        while index < state.requests.len() {
            let request = &state.requests[index];
            if result >= 0 && fd.revents & (if request.writing { 4 } else { 1 } | 8 | 16 | 32) == 0
            {
                index += 1;
                continue;
            }
            let stream = state.stream.as_ref().unwrap();
            let completed = match transfer(stream, request) {
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
            connecting: false,
            owner: crate::api::system::startup::thread_id(),
        };
        if len == 0 && state.stream.is_some() {
            complete_with(&state, &request, 0, 0, true);
            return Some(1);
        }
        state.requests.push_back(request);
        pipe.changed.notify_one();
        error::set(997);
        return Some(0);
    }
    if count.is_null() {
        return Some(fail(87));
    }
    crate::api::threading::iocp::release_worker();
    let request = Request {
        buffer: buffer as usize,
        len,
        ov: 0,
        writing,
        connecting: false,
        owner: crate::api::system::startup::thread_id(),
    };
    loop {
        let state = pipe.state.lock().unwrap();
        if state.closed {
            return Some(fail(6));
        }
        let Some(stream) = state.stream.as_ref() else {
            return Some(fail(230));
        };
        match transfer(stream, &request) {
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
    _mode: *const u32,
    _collection: *const u32,
    _timeout: *const u32,
) -> i32 {
    if !known(handle) {
        return fail(6);
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
