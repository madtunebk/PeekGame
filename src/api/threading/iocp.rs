//! Process-local I/O completion queues. Registry locks only protect handle lookup;
//! blocking consumers use a port's own mutex. Packet pointers remain opaque.
use crate::api::system::error;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Packet {
    pub key: usize,
    pub overlapped: usize,
    pub internal: usize,
    pub bytes: u32,
}
struct State {
    queue: VecDeque<Packet>,
    closed: bool,
    active: usize,
}
pub struct Port {
    state: Mutex<State>,
    changed: Condvar,
    handles: AtomicUsize,
    _limit: usize,
}
#[derive(Clone)]
pub struct Binding {
    pub port: Arc<Port>,
    pub key: usize,
}
impl Binding {
    pub fn same_port(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.port, &other.port)
    }
    pub fn complete(&self, overlapped: usize, status: usize, bytes: u32) {
        self.port.post(Packet {
            key: self.key,
            overlapped,
            internal: status,
            bytes,
        });
    }
}
impl Port {
    pub fn post(&self, packet: Packet) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.closed {
            return false;
        }
        state.queue.push_back(packet);
        self.changed.notify_all();
        true
    }
}
struct Lease(Arc<Port>);
impl Drop for Lease {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.active -= 1;
        self.0.changed.notify_all();
    }
}
thread_local! {static WORKER:RefCell<Option<Lease>>=const{RefCell::new(None)};}
/// Called before Windows waits and at thread exit so another consumer can run.
pub fn release_worker() {
    WORKER.with(|worker| {
        worker.borrow_mut().take();
    });
}
static PORTS: OnceLock<Mutex<HashMap<usize, Arc<Port>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x69000000);
fn ports() -> &'static Mutex<HashMap<usize, Arc<Port>>> {
    PORTS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn get(handle: usize) -> Option<Arc<Port>> {
    ports().lock().unwrap().get(&handle).cloned()
}
fn insert(port: Arc<Port>) -> usize {
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    ports().lock().unwrap().insert(handle, port);
    handle
}
pub fn duplicate(handle: usize) -> Option<usize> {
    let registry = &mut *ports().lock().unwrap();
    let port = registry.get(&handle)?.clone();
    port.handles.fetch_add(1, Ordering::Relaxed);
    let target = NEXT.fetch_add(16, Ordering::Relaxed);
    registry.insert(target, port);
    Some(target)
}
pub fn close(handle: usize) -> bool {
    let Some(port) = ports().lock().unwrap().remove(&handle) else {
        return false;
    };
    if port.handles.fetch_sub(1, Ordering::AcqRel) == 1 {
        let mut state = port.state.lock().unwrap();
        state.closed = true;
        port.changed.notify_all();
    }
    true
}
pub extern "win64" fn create(file: usize, existing: usize, key: usize, threads: u32) -> usize {
    if file == usize::MAX && existing != 0 {
        error::set(87);
        return 0;
    }
    let (port, created) = if existing == 0 {
        (
            Arc::new(Port {
                state: Mutex::new(State {
                    queue: VecDeque::new(),
                    closed: false,
                    active: 0,
                }),
                changed: Condvar::new(),
                handles: AtomicUsize::new(1),
                _limit: if threads == 0 {
                    std::thread::available_parallelism().map_or(1, usize::from)
                } else {
                    threads as usize
                },
            }),
            true,
        )
    } else {
        match get(existing) {
            Some(p) => (p, false),
            None => {
                error::set(6);
                return 0;
            }
        }
    };
    if file != usize::MAX {
        if crate::api::network::afd::is_afd(file) {
            crate::api::network::afd::associate(file, port.clone(), key);
        } else {
            let binding = Binding {
                port: port.clone(),
                key,
            };
            if let Err(code) = crate::api::filesystem::files::associate_completion(file, binding) {
                error::set(code);
                return 0;
            }
        }
    }
    if created {
        insert(port)
    } else {
        existing
    }
}
pub extern "win64" fn post(handle: usize, bytes: u32, key: usize, overlapped: usize) -> i32 {
    let Some(port) = get(handle) else {
        error::set(6);
        return 0;
    };
    if !port.post(Packet {
        key,
        overlapped,
        internal: 0,
        bytes,
    }) {
        error::set(6);
        return 0;
    }
    1
}
fn dequeue(
    port: &Arc<Port>,
    out: &mut [Packet],
    timeout: u32,
    alertable: bool,
) -> Result<usize, u32> {
    release_worker();
    let start = Instant::now();
    let mut state = port.state.lock().unwrap();
    loop {
        if state.closed {
            return Err(735);
        }
        if alertable && super::apc::pending() {
            drop(state);
            super::apc::run();
            return Err(192);
        }
        if !state.queue.is_empty() {
            let n = out.len().min(state.queue.len());
            for item in &mut out[..n] {
                *item = state.queue.pop_front().unwrap();
            }
            state.active += 1;
            WORKER.with(|worker| *worker.borrow_mut() = Some(Lease(port.clone())));
            return Ok(n);
        }
        let remaining = if timeout == u32::MAX {
            Duration::MAX
        } else {
            Duration::from_millis(timeout as u64).saturating_sub(start.elapsed())
        };
        if remaining.is_zero() {
            return Err(258);
        }
        if alertable {
            state = port
                .changed
                .wait_timeout(state, remaining.min(Duration::from_millis(5)))
                .unwrap()
                .0;
        } else if timeout == u32::MAX {
            state = port.changed.wait(state).unwrap();
        } else {
            state = port.changed.wait_timeout(state, remaining).unwrap().0;
        }
    }
}
pub unsafe extern "win64" fn get_status(
    handle: usize,
    bytes: *mut u32,
    key: *mut usize,
    overlapped: *mut usize,
    timeout: u32,
) -> i32 {
    if bytes.is_null() || key.is_null() || overlapped.is_null() {
        error::set(87);
        return 0;
    }
    unsafe {
        *overlapped = 0;
    }
    let Some(port) = get(handle) else {
        error::set(6);
        return 0;
    };
    let mut packets = [Packet {
        key: 0,
        overlapped: 0,
        internal: 0,
        bytes: 0,
    }];
    match dequeue(&port, &mut packets, timeout, false) {
        Err(code) => {
            error::set(code);
            0
        }
        Ok(_) => {
            let packet = packets[0];
            unsafe {
                *bytes = packet.bytes;
                *key = packet.key;
                *overlapped = packet.overlapped;
            }
            if (packet.internal as u32 as i32) < 0 {
                error::set(status_error(packet.internal as u32));
                0
            } else {
                1
            }
        }
    }
}
pub unsafe extern "win64" fn get_status_ex(
    handle: usize,
    entries: *mut Packet,
    count: u32,
    removed: *mut u32,
    timeout: u32,
    alertable: i32,
) -> i32 {
    if entries.is_null() || removed.is_null() || count == 0 {
        error::set(87);
        return 0;
    }
    unsafe {
        *removed = 0;
    }
    let Some(port) = get(handle) else {
        error::set(6);
        return 0;
    };
    let out = unsafe { std::slice::from_raw_parts_mut(entries, count as usize) };
    match dequeue(&port, out, timeout, alertable != 0) {
        Ok(n) => {
            unsafe {
                *removed = n as u32;
            }
            1
        }
        Err(code) => {
            error::set(code);
            0
        }
    }
}
fn status_error(status: u32) -> u32 {
    match status {
        0xc0000120 => 995,
        0xc0000011 => 38,
        0xc0000008 => 6,
        0xc0000185 => 1117,
        0xc000014b => 109,
        _ => 317,
    }
}
pub extern "win64" fn notification_modes(file: usize, modes: u8) -> i32 {
    if modes & !3 != 0 {
        error::set(87);
        return 0;
    }
    if crate::api::network::afd::is_afd(file) {
        crate::api::network::afd::set_modes(file, modes);
        return 1;
    }
    match crate::api::filesystem::files::set_completion_modes(file, modes) {
        Ok(()) => 1,
        Err(code) => {
            error::set(code);
            0
        }
    }
}
