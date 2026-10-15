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
    changed: Condvar,
}
thread_local! {static CURRENT:RefCell<Option<Arc<Thread>>>=const{RefCell::new(None)};}
static THREADS: OnceLock<Mutex<HashMap<usize, Arc<Thread>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x66000000);
fn threads() -> &'static Mutex<HashMap<usize, Arc<Thread>>> {
    THREADS.get_or_init(|| Mutex::new(HashMap::new()))
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
    if !security.is_null() || flags != 0 {
        error::set(50);
        return 0;
    }
    let Some(start) = start else {
        error::set(87);
        return 0;
    };
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let mut builder = std::thread::Builder::new();
    if stack > 0 {
        builder = builder.stack_size(stack);
    }
    let spawned = builder.spawn(move || {
        let thread = Arc::new(Thread {
            id: startup::thread_id(),
            exit: Mutex::new(None),
            changed: Condvar::new(),
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
        let result = start(parameter);
        crate::host::thread_template::detach();
        *thread.exit.lock().unwrap() = Some(result);
        thread.changed.notify_all();
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
    if source_process != usize::MAX
        || target_process != usize::MAX
        || inherit != 0
        || options & !3 != 0
        || options & 2 == 0
        || access != 0
    {
        error::set(50);
        return 0;
    }
    let existing = threads().lock().unwrap().get(&source).cloned();
    let target = if source == usize::MAX - 1 {
        Some(CURRENT.with(|current| {
            let mut current = current.borrow_mut();
            let thread = current
                .get_or_insert_with(|| {
                    Arc::new(Thread {
                        id: startup::thread_id(),
                        exit: Mutex::new(None),
                        changed: Condvar::new(),
                    })
                })
                .clone();
            insert(thread)
        }))
    } else if let Some(thread) = existing {
        Some(insert(thread))
    } else {
        super::events::duplicate(source)
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
