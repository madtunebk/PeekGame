use crate::api::system::{error, startup};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
pub type Start = extern "win64" fn(usize) -> u32;
struct Thread {
    id: u32,
    exit: Mutex<Option<u32>>,
    times: Mutex<Option<[u64; 4]>>,
    changed: Condvar,
    /// CREATE_SUSPENDED: the start routine waits until ResumeThread clears this.
    suspended: Mutex<bool>,
}
const CREATE_SUSPENDED: u32 = 0x4;
const STACK_SIZE_PARAM_IS_A_RESERVATION: u32 = 0x10000;
thread_local! {static CURRENT:RefCell<Option<Arc<Thread>>>=const{RefCell::new(None)};}
static THREADS: OnceLock<Mutex<HashMap<usize, Arc<Thread>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x66000000);
fn threads() -> &'static Mutex<HashMap<usize, Arc<Thread>>> {
    THREADS.get_or_init(|| Mutex::new(HashMap::new()))
}
/// Has the thread exited? None: not a thread handle.
pub(crate) fn exited(handle: usize) -> Option<bool> {
    let thread = threads().lock().unwrap().get(&handle).cloned()?;
    let done = thread.exit.lock().unwrap().is_some();
    Some(done)
}
/// The calling thread's record (made on first use for threads PeekGame did not
/// create, such as the main thread).
fn current_thread() -> Arc<Thread> {
    CURRENT.with(|current| {
        current
            .borrow_mut()
            .get_or_insert_with(|| record(startup::thread_id()))
            .clone()
    })
}
fn record(id: u32) -> Arc<Thread> {
    Arc::new(Thread {
        id,
        exit: Mutex::new(None),
        times: Mutex::new(None),
        changed: Condvar::new(),
        suspended: Mutex::new(false),
    })
}
/// The thread ID behind a thread handle (GetCurrentThread's pseudo handle included).
pub(crate) fn id_of(handle: usize) -> Option<u32> {
    if handle == usize::MAX - 1 {
        return Some(startup::thread_id());
    }
    threads().lock().unwrap().get(&handle).map(|t| t.id)
}
/// OpenThread: a new handle to a thread of this process, by ID.
pub extern "win64" fn open(_access: u32, _inherit: i32, id: u32) -> usize {
    let known = threads()
        .lock()
        .unwrap()
        .values()
        .find(|t| t.id == id)
        .cloned();
    let thread = match known {
        Some(thread) => thread,
        None if id == startup::thread_id() => current_thread(),
        // A thread PeekGame did not create (its exit is not observed).
        None if std::path::Path::new(&format!("/proc/self/task/{id}")).exists() => record(id),
        None => {
            error::set(87);
            return 0;
        }
    };
    insert(thread)
}
/// GetThreadId.
pub extern "win64" fn thread_id_of(handle: usize) -> u32 {
    id_of(handle).unwrap_or_else(|| {
        error::set(6);
        0
    })
}
fn insert(thread: Arc<Thread>) -> usize {
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    threads().lock().unwrap().insert(handle, thread);
    handle
}
pub unsafe extern "win64" fn create(
    security: *const c_void,
    stack: usize,
    start: Option<Start>,
    parameter: usize,
    flags: u32,
    id: *mut u32,
) -> usize {
    // Security attributes only matter for inheritance by child processes.
    let _ = security;
    if flags & !(CREATE_SUSPENDED | STACK_SIZE_PARAM_IS_A_RESERVATION) != 0 {
        error::set(50);
        return 0;
    }
    let suspended = flags & CREATE_SUSPENDED != 0;
    let Some(start) = start else {
        error::set(87);
        return 0;
    };
    let inherited_locale = crate::api::system::locale::thread::get();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let mut builder = std::thread::Builder::new();
    if stack > 0 {
        builder = builder.stack_size(stack);
    }
    let spawned = builder.spawn(move || {
        crate::api::system::locale::thread::inherit(inherited_locale);
        let thread = Arc::new(Thread {
            id: startup::thread_id(),
            exit: Mutex::new(None),
            times: Mutex::new(None),
            changed: Condvar::new(),
            suspended: Mutex::new(suspended),
        });
        if let Err(e) = crate::host::thread_template::attach() {
            eprintln!("Windows thread initialization failed: {e}");
            let _ = send.send(None);
            return;
        }
        if send.send(Some(thread.clone())).is_err() {
            return;
        }
        CURRENT.with(|current| *current.borrow_mut() = Some(thread.clone()));
        {
            let mut held = thread.suspended.lock().unwrap();
            while *held {
                held = thread.changed.wait(held).unwrap();
            }
        }
        let result = crate::host::thread_entry::run(start, parameter);
        super::objects::thread_exited(std::thread::current().id());
        crate::api::graphics::opengl::release_thread();
        super::iocp::release_worker();
        crate::host::thread_template::detach();
        *thread.times.lock().unwrap() = super::thread_times::snapshot(thread.id);
        *thread.exit.lock().unwrap() = Some(result);
        thread.changed.notify_all();
        super::objects::changed();
    });
    if spawned.is_err() {
        error::set(8);
        return 0;
    }
    let Ok(Some(thread)) = receive.recv() else {
        error::set(8);
        return 0;
    };
    if !id.is_null() {
        unsafe {
            *id = thread.id;
        }
    }
    insert(thread)
}
pub unsafe extern "win64" fn duplicate(
    source_process: usize,
    source: usize,
    target_process: usize,
    out: *mut usize,
    access: u32,
    inherit: i32,
    options: u32,
) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    unsafe {
        *out = 0;
    }
    // Only this process exists. Inheritance concerns child processes, so the flag is
    // accepted; access must be "the same" (DUPLICATE_SAME_ACCESS or no extra rights).
    let _ = inherit;
    if source_process != usize::MAX
        || target_process != usize::MAX
        || options & !3 != 0
        || (options & 2 == 0 && access != 0)
    {
        error::set(50);
        return 0;
    }
    let existing = threads().lock().unwrap().get(&source).cloned();
    let target = if source == usize::MAX - 1 {
        Some(insert(current_thread()))
    } else if let Some(thread) = existing {
        Some(insert(thread))
    } else {
        super::iocp::duplicate(source)
            .or_else(|| super::events::duplicate(source))
            .or_else(|| super::objects::duplicate(source))
            .or_else(|| crate::api::filesystem::files::duplicate(source))
    };
    let Some(target) = target else {
        error::set(6);
        return 0;
    };
    unsafe {
        *out = target;
    }
    if options & 1 != 0 {
        super::events::close(source);
    }
    1
}
pub fn close(handle: usize) -> bool {
    threads().lock().unwrap().remove(&handle).is_some()
}
pub fn wait(handle: usize, milliseconds: u32) -> Option<u32> {
    let thread = threads().lock().unwrap().get(&handle).cloned()?;
    let mut exit = thread.exit.lock().unwrap();
    let start = Instant::now();
    while exit.is_none() {
        if milliseconds == u32::MAX {
            exit = thread.changed.wait(exit).unwrap();
        } else {
            let duration =
                Duration::from_millis(milliseconds as u64).saturating_sub(start.elapsed());
            if duration.is_zero() {
                return Some(258);
            }
            exit = thread.changed.wait_timeout(exit, duration).unwrap().0;
        }
    }
    Some(0)
}
pub unsafe extern "win64" fn get_exit_code(handle: usize, out: *mut u32) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let thread = threads().lock().unwrap().get(&handle).cloned();
    let Some(thread) = thread else {
        error::set(6);
        return 0;
    };
    unsafe {
        *out = thread.exit.lock().unwrap().unwrap_or(259);
    }
    1
}

pub extern "win64" fn priority(handle: usize) -> i32 {
    let id = if handle == usize::MAX - 1 {
        Some(startup::thread_id())
    } else {
        threads().lock().unwrap().get(&handle).map(|t| t.id)
    };
    let Some(id) = id else {
        error::set(6);
        return i32::MAX;
    };
    unsafe extern "C" {
        fn getpriority(which: i32, who: u32) -> i32;
    }
    let nice = unsafe { getpriority(0, id) };
    if nice == 0 {
        0
    } else if nice > 0 {
        -((nice + 4) / 5).min(2)
    } else {
        ((-nice + 4) / 5).min(2)
    }
}
pub unsafe extern "win64" fn begin_thread(
    security: *const c_void,
    stack: u32,
    start: Option<Start>,
    parameter: usize,
    flags: u32,
    id: *mut u32,
) -> usize {
    unsafe { create(security, stack as usize, start, parameter, flags, id) }
}

/// ResumeThread: starts a thread created suspended. Suspending running threads
/// (SuspendThread) is not implemented yet, so their count is always 0.
/// Is this a thread created with CREATE_SUSPENDED that has not started yet?
pub(crate) fn created_suspended(handle: usize) -> bool {
    threads()
        .lock()
        .unwrap()
        .get(&handle)
        .is_some_and(|t| *t.suspended.lock().unwrap())
}
pub extern "win64" fn resume(handle: usize) -> u32 {
    let Some(thread) = threads().lock().unwrap().get(&handle).cloned() else {
        error::set(6);
        return u32::MAX;
    };
    let mut suspended = thread.suspended.lock().unwrap();
    if *suspended {
        *suspended = false;
        thread.changed.notify_all();
        1
    } else {
        0
    }
}

/// Saved exit accounting remains available through duplicated/open thread handles.
pub(super) fn accounting(handle: usize) -> Option<[u64; 4]> {
    let thread = if handle == usize::MAX - 1 {
        current_thread()
    } else {
        threads().lock().unwrap().get(&handle).cloned()?
    };
    let saved = thread.times.lock().unwrap();
    if let Some(times) = *saved {
        return Some(times);
    }
    let mut times = super::thread_times::snapshot(thread.id)?;
    times[1] = 0; // the exit FILETIME is undefined while running; choose zero.
    Some(times)
}

pub unsafe extern "win64" fn convert_thread_to_fiber(param: usize) -> usize {
    unsafe { convert_thread_to_fiber_ex(param, 0) }
}

pub unsafe extern "win64" fn convert_thread_to_fiber_ex(param: usize, _flags: u32) -> usize {
    let existing = crate::host::teb::get_fiber_data();
    if existing != 0 {
        crate::api::system::error::set(0x5002); // ERROR_ALREADY_FIBER
        return 0;
    }
    let fiber = Box::into_raw(Box::new(param)) as usize;
    crate::host::teb::set_fiber_data(fiber);
    fiber
}

pub unsafe extern "win64" fn convert_fiber_to_thread() -> i32 {
    let existing = crate::host::teb::get_fiber_data();
    if existing == 0 {
        crate::api::system::error::set(0x5003); // ERROR_NOT_FIBER
        return 0;
    }
    drop(unsafe { Box::from_raw(existing as *mut usize) });
    crate::host::teb::set_fiber_data(0);
    1
}

/// FlushProcessWriteBuffers: every CPU running this process passes a full barrier
/// (Windows uses an IPI; .NET/Mono GCs and asymmetric locks rely on it). Linux
/// membarrier(PRIVATE_EXPEDITED) is the same operation; GLOBAL is the slower fallback,
/// and a local fence is all that is left without membarrier.
pub extern "win64" fn flush_process_write_buffers() {
    unsafe extern "C" {
        fn syscall(number: i64, ...) -> i64;
    }
    const SYS_MEMBARRIER: i64 = 324;
    const QUERY: i64 = 0;
    const GLOBAL: i64 = 1;
    const PRIVATE_EXPEDITED: i64 = 8;
    const REGISTER_PRIVATE_EXPEDITED: i64 = 16;
    static MODE: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
    let mode = *MODE.get_or_init(|| unsafe {
        let supported = syscall(SYS_MEMBARRIER, QUERY, 0, 0);
        if supported > 0
            && supported & PRIVATE_EXPEDITED != 0
            && syscall(SYS_MEMBARRIER, REGISTER_PRIVATE_EXPEDITED, 0, 0) == 0
        {
            PRIVATE_EXPEDITED
        } else if supported > 0 && supported & GLOBAL != 0 {
            GLOBAL
        } else {
            0
        }
    });
    std::sync::atomic::fence(Ordering::SeqCst);
    if mode != 0 {
        unsafe { syscall(SYS_MEMBARRIER, mode, 0, 0) };
    }
}

pub extern "win64" fn current_processor_number() -> u32 {
    unsafe extern "C" {
        fn syscall(number: i64, ...) -> i64;
    }
    let mut cpu: u32 = 0;
    unsafe {
        syscall(309, &mut cpu as *mut u32, std::ptr::null_mut::<c_void>(), std::ptr::null_mut::<c_void>());
    }
    cpu
}
