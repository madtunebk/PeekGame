//! The default Windows thread pool (Vista API): work, timer and wait objects. Every
//! callback runs on a Windows thread. Work items share a small pool of reusable
//! threads; each timer and each wait object has its own thread for as long as it is
//! open. Callback environments are accepted only for the default pool: custom pools,
//! cleanup groups and RaceDll fail with ERROR_NOT_SUPPORTED.
use crate::api::{system::error, threading::threads};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, Instant},
};

type WorkCallback = unsafe extern "win64" fn(usize, usize, usize);
type WaitCallback = unsafe extern "win64" fn(usize, usize, usize, u32);
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 0x102;

/// PTP_CALLBACK_INSTANCE: what to do when the callback returns.
struct Instance {
    free_module: AtomicUsize,
}
fn with_instance(f: impl FnOnce(usize)) {
    let instance = Instance { free_module: AtomicUsize::new(0) };
    f(&instance as *const Instance as usize);
    let module = instance.free_module.load(Ordering::Acquire);
    if module != 0 {
        crate::api::system::dynamic::free_library(module);
    }
}
/// FreeLibraryWhenCallbackReturns.
pub extern "win64" fn free_library_when_callback_returns(instance: usize, module: usize) {
    if instance != 0 {
        unsafe { &*(instance as *const Instance) }.free_module.store(module, Ordering::Release);
    }
}
/// TP_CALLBACK_ENVIRON (x64): Version @0, Pool @8, CleanupGroup @16, cancel callback
/// @24, RaceDll @32. Only the default pool without cleanup group is supported.
fn environment_supported(environment: usize) -> bool {
    if environment == 0 {
        return true;
    }
    let field = |at: usize| unsafe { *((environment + at) as *const usize) };
    if field(8) != 0 || field(16) != 0 || field(32) != 0 {
        eprintln!("thread pool: custom pools, cleanup groups and RaceDll are not supported");
        error::set(50);
        return false;
    }
    true
}
/// Starts `f` on a new Windows thread.
fn spawn(f: Box<dyn FnOnce() + Send>) -> bool {
    extern "win64" fn start(parameter: usize) -> u32 {
        let f = unsafe { Box::from_raw(parameter as *mut Box<dyn FnOnce() + Send>) };
        f();
        0
    }
    let parameter = Box::into_raw(Box::new(f)) as usize;
    let handle = unsafe { threads::create(std::ptr::null(), 0, Some(start), parameter, 0, std::ptr::null_mut()) };
    if handle == 0 {
        drop(unsafe { Box::from_raw(parameter as *mut Box<dyn FnOnce() + Send>) });
        return false;
    }
    threads::close(handle);
    true
}

// ---- shared workers for work items ----------------------------------------------
type Job = Box<dyn FnOnce() + Send>;
struct Workers {
    queue: Mutex<(VecDeque<Job>, usize)>, // jobs, idle workers
    ready: Condvar,
}
static WORKERS: Workers = Workers { queue: Mutex::new((VecDeque::new(), 0)), ready: Condvar::new() };
fn run(job: Job) {
    let mut q = WORKERS.queue.lock().unwrap();
    q.0.push_back(job);
    // Enough idle workers for every queued job: wake one; otherwise add a worker.
    if q.1 >= q.0.len() {
        WORKERS.ready.notify_one();
        return;
    }
    drop(q);
    if !spawn(Box::new(worker)) {
        eprintln!("thread pool: a worker thread could not start");
    }
}
/// Runs queued jobs; exits after 30 s without work.
fn worker() {
    let mut q = WORKERS.queue.lock().unwrap();
    loop {
        if let Some(job) = q.0.pop_front() {
            drop(q);
            job();
            q = WORKERS.queue.lock().unwrap();
            continue;
        }
        q.1 += 1;
        let (next, timeout) = WORKERS.ready.wait_timeout(q, Duration::from_secs(30)).unwrap();
        q = next;
        q.1 -= 1;
        if timeout.timed_out() && q.0.is_empty() {
            return;
        }
    }
}

// ---- work -----------------------------------------------------------------------
struct Work {
    callback: usize,
    context: usize,
    outstanding: Mutex<usize>,
    idle: Condvar,
}
pub extern "win64" fn create_work(callback: usize, context: usize, environment: usize) -> usize {
    if callback == 0 {
        error::set(87);
        return 0;
    }
    if !environment_supported(environment) {
        return 0;
    }
    Arc::into_raw(Arc::new(Work { callback, context, outstanding: Mutex::new(0), idle: Condvar::new() })) as usize
}
fn work_ref(handle: usize) -> Arc<Work> {
    let work = unsafe { Arc::from_raw(handle as *const Work) };
    let clone = work.clone();
    std::mem::forget(work);
    clone
}
pub extern "win64" fn submit_work(handle: usize) {
    if handle == 0 {
        return;
    }
    let work = work_ref(handle);
    *work.outstanding.lock().unwrap() += 1;
    run(Box::new(move || {
        let callback: WorkCallback = unsafe { std::mem::transmute(work.callback) };
        with_instance(|instance| unsafe { callback(instance, work.context, handle) });
        let mut n = work.outstanding.lock().unwrap();
        *n -= 1;
        work.idle.notify_all();
    }));
}
/// WaitForThreadpoolWorkCallbacks: `cancel` drops callbacks that have not started.
pub extern "win64" fn wait_work_callbacks(handle: usize, _cancel: i32) {
    if handle == 0 {
        return;
    }
    let work = work_ref(handle);
    let mut n = work.outstanding.lock().unwrap();
    while *n != 0 {
        n = work.idle.wait(n).unwrap();
    }
}
/// CloseThreadpoolWork: the object goes away once its queued callbacks have run.
pub extern "win64" fn close_work(handle: usize) {
    if handle != 0 {
        drop(unsafe { Arc::from_raw(handle as *const Work) });
    }
}

// ---- due times ------------------------------------------------------------------
/// A FILETIME due time: negative = relative (100 ns units), otherwise absolute UTC.
fn due(filetime: *const i64) -> Instant {
    let value = unsafe { filetime.read_unaligned() };
    let now = Instant::now();
    if value < 0 {
        return now + Duration::from_nanos(value.unsigned_abs().saturating_mul(100));
    }
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let current = since.as_nanos() as i128 / 100 + 116_444_736_000_000_000;
    let delta = value as i128 - current;
    if delta <= 0 { now } else { now + Duration::from_nanos((delta as u64).saturating_mul(100)) }
}

// ---- timers ---------------------------------------------------------------------
struct TimerState {
    due: Option<Instant>,
    period: u32,
    running: bool,
    closed: bool,
}
struct Timer {
    callback: usize,
    context: usize,
    state: Mutex<TimerState>,
    changed: Condvar,
}
pub extern "win64" fn create_timer(callback: usize, context: usize, environment: usize) -> usize {
    if callback == 0 {
        error::set(87);
        return 0;
    }
    if !environment_supported(environment) {
        return 0;
    }
    let timer = Arc::new(Timer {
        callback,
        context,
        state: Mutex::new(TimerState { due: None, period: 0, running: false, closed: false }),
        changed: Condvar::new(),
    });
    let handle = Arc::into_raw(timer.clone()) as usize;
    if !spawn(Box::new(move || timer_thread(timer, handle))) {
        drop(unsafe { Arc::from_raw(handle as *const Timer) });
        error::set(8);
        return 0;
    }
    handle
}
fn timer_thread(timer: Arc<Timer>, handle: usize) {
    let mut s = timer.state.lock().unwrap();
    loop {
        if s.closed {
            return;
        }
        let Some(at) = s.due else {
            s = timer.changed.wait(s).unwrap();
            continue;
        };
        let now = Instant::now();
        if now < at {
            s = timer.changed.wait_timeout(s, at - now).unwrap().0;
            continue;
        }
        s.due = (s.period != 0).then(|| at + Duration::from_millis(s.period as u64));
        s.running = true;
        drop(s);
        let callback: WorkCallback = unsafe { std::mem::transmute(timer.callback) };
        with_instance(|instance| unsafe { callback(instance, timer.context, handle) });
        s = timer.state.lock().unwrap();
        s.running = false;
        timer.changed.notify_all();
    }
}
fn timer_ref(handle: usize) -> Arc<Timer> {
    let timer = unsafe { Arc::from_raw(handle as *const Timer) };
    let clone = timer.clone();
    std::mem::forget(timer);
    clone
}
/// SetThreadpoolTimer: NULL due time cancels; the window length is not used.
pub unsafe extern "win64" fn set_timer(handle: usize, due_time: *const i64, period: u32, _window: u32) {
    if handle == 0 {
        return;
    }
    let timer = timer_ref(handle);
    let mut s = timer.state.lock().unwrap();
    s.due = (!due_time.is_null()).then(|| due(due_time));
    s.period = period;
    timer.changed.notify_all();
}
pub unsafe extern "win64" fn is_timer_set(handle: usize) -> i32 {
    (handle != 0 && timer_ref(handle).state.lock().unwrap().due.is_some()) as i32
}
/// WaitForThreadpoolTimerCallbacks: waits for a running callback; `cancel` also
/// cancels the pending expiration.
pub extern "win64" fn wait_timer_callbacks(handle: usize, cancel: i32) {
    if handle == 0 {
        return;
    }
    let timer = timer_ref(handle);
    let mut s = timer.state.lock().unwrap();
    if cancel != 0 {
        s.due = None;
        timer.changed.notify_all();
    }
    while s.running {
        s = timer.changed.wait(s).unwrap();
    }
}
pub extern "win64" fn close_timer(handle: usize) {
    if handle == 0 {
        return;
    }
    let timer = unsafe { Arc::from_raw(handle as *const Timer) };
    let mut s = timer.state.lock().unwrap();
    s.closed = true;
    s.due = None;
    timer.changed.notify_all();
}

// ---- waits ----------------------------------------------------------------------
struct WaitState {
    handle: usize,
    deadline: Option<Instant>,
    armed: bool,
    generation: u64,
    running: bool,
    closed: bool,
}
struct Wait {
    callback: usize,
    context: usize,
    state: Mutex<WaitState>,
    changed: Condvar,
}
pub extern "win64" fn create_wait(callback: usize, context: usize, environment: usize) -> usize {
    if callback == 0 {
        error::set(87);
        return 0;
    }
    if !environment_supported(environment) {
        return 0;
    }
    let wait = Arc::new(Wait {
        callback,
        context,
        state: Mutex::new(WaitState { handle: 0, deadline: None, armed: false, generation: 0, running: false, closed: false }),
        changed: Condvar::new(),
    });
    let handle = Arc::into_raw(wait.clone()) as usize;
    if !spawn(Box::new(move || wait_thread(wait, handle))) {
        drop(unsafe { Arc::from_raw(handle as *const Wait) });
        error::set(8);
        return 0;
    }
    handle
}
/// Waits on the armed handle in short slices so SetThreadpoolWait/Close take effect.
fn wait_thread(wait: Arc<Wait>, handle: usize) {
    const SLICE: Duration = Duration::from_millis(20);
    loop {
        let (object, deadline, generation) = {
            let mut s = wait.state.lock().unwrap();
            while !s.armed && !s.closed {
                s = wait.changed.wait(s).unwrap();
            }
            if s.closed {
                return;
            }
            (s.handle, s.deadline, s.generation)
        };
        let slice = deadline.map_or(SLICE, |d| d.saturating_duration_since(Instant::now()).min(SLICE));
        let result = crate::api::threading::events::wait(object, slice.as_millis() as u32);
        let mut s = wait.state.lock().unwrap();
        if s.closed {
            return;
        }
        if s.generation != generation || !s.armed {
            continue; // re-armed or cancelled meanwhile
        }
        let outcome = if result == WAIT_OBJECT_0 {
            WAIT_OBJECT_0
        } else if deadline.is_some_and(|d| Instant::now() >= d) {
            WAIT_TIMEOUT
        } else if result == WAIT_TIMEOUT {
            continue;
        } else {
            eprintln!("thread pool: waiting on handle {object:#x} failed ({result:#x}); the wait is disarmed");
            s.armed = false;
            continue;
        };
        s.armed = false;
        s.running = true;
        drop(s);
        let callback: WaitCallback = unsafe { std::mem::transmute(wait.callback) };
        with_instance(|instance| unsafe { callback(instance, wait.context, handle, outcome) });
        let mut s = wait.state.lock().unwrap();
        s.running = false;
        wait.changed.notify_all();
    }
}
fn wait_ref(handle: usize) -> Arc<Wait> {
    let wait = unsafe { Arc::from_raw(handle as *const Wait) };
    let clone = wait.clone();
    std::mem::forget(wait);
    clone
}
/// SetThreadpoolWait: arms one wait on `object` (NULL disarms); NULL timeout = infinite.
pub unsafe extern "win64" fn set_wait(handle: usize, object: usize, timeout: *const i64) {
    if handle == 0 {
        return;
    }
    let wait = wait_ref(handle);
    let mut s = wait.state.lock().unwrap();
    s.generation += 1;
    s.handle = object;
    s.armed = object != 0;
    s.deadline = (!timeout.is_null()).then(|| due(timeout));
    wait.changed.notify_all();
}
pub extern "win64" fn wait_wait_callbacks(handle: usize, cancel: i32) {
    if handle == 0 {
        return;
    }
    let wait = wait_ref(handle);
    let mut s = wait.state.lock().unwrap();
    if cancel != 0 {
        s.armed = false;
        s.generation += 1;
    }
    while s.running {
        s = wait.changed.wait(s).unwrap();
    }
}
pub extern "win64" fn close_wait(handle: usize) {
    if handle == 0 {
        return;
    }
    let wait = unsafe { Arc::from_raw(handle as *const Wait) };
    let mut s = wait.state.lock().unwrap();
    s.closed = true;
    s.armed = false;
    wait.changed.notify_all();
}
