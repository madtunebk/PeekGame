//! Semaphores, mutexes and multi-object waits (ported from peek's sync_objects.inc
//! and kernel32_ext.inc). Every operation that can make an object signalled calls
//! [`changed`], which wakes waiters on several objects; they then re-check their
//! handles. Names are a process-local namespace.
use super::{apc, events, threads};
use crate::api::{system::error, text};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Condvar, Mutex,
    },
    thread::ThreadId,
    time::{Duration, Instant},
};
const WAIT_OBJECT_0: u32 = 0;
const WAIT_ABANDONED_0: u32 = 0x80;
const WAIT_TIMEOUT: u32 = 258;
const WAIT_FAILED: u32 = u32::MAX;
const ERROR_ALREADY_EXISTS: u32 = 183;
const ERROR_TOO_MANY_POSTS: u32 = 298;
const ERROR_NOT_OWNER: u32 = 288;

enum Object {
    Semaphore { count: i32, maximum: i32 },
    /// `abandoned`: the owner ended without releasing it; the next acquirer is told
    /// with WAIT_ABANDONED.
    Mutex { owner: Option<ThreadId>, depth: u32, abandoned: bool },
}
struct Named {
    name: Option<String>,
    object: Object,
}
static OBJECTS: Mutex<Option<HashMap<usize, Named>>> = Mutex::new(None);
static GENERATION: Mutex<u64> = Mutex::new(0);
static CHANGED: Condvar = Condvar::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x6700_0000);

/// Is `name` used by a semaphore or mutex? (Kernel object names share one namespace.)
pub fn name_taken(name: &str) -> bool {
    with(|t| t.values().any(|n| n.name.as_deref() == Some(name)))
}
/// Wakes multi-object waiters: something may have become signalled.
pub fn changed() {
    *GENERATION.lock().unwrap() += 1;
    CHANGED.notify_all();
}
fn with<T>(f: impl FnOnce(&mut HashMap<usize, Named>) -> T) -> T {
    f(OBJECTS.lock().unwrap().get_or_insert_with(HashMap::new))
}
fn create(name: Option<String>, object: Object) -> usize {
    with(|table| {
        if let Some(name) = &name {
            if let Some((&handle, _)) = table.iter().find(|(_, o)| o.name.as_ref() == Some(name)) {
                let alias = NEXT.fetch_add(16, Ordering::Relaxed);
                ALIASES
                    .lock()
                    .unwrap()
                    .get_or_insert_with(HashMap::new)
                    .insert(alias, handle);
                error::set(ERROR_ALREADY_EXISTS);
                return alias;
            }
        }
        error::set(0);
        let handle = NEXT.fetch_add(16, Ordering::Relaxed);
        table.insert(handle, Named { name, object });
        handle
    })
}
/// DuplicateHandle: a second handle naming the same semaphore or mutex.
static ALIASES: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);
fn resolve(handle: usize) -> usize {
    ALIASES
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|a| a.get(&handle).copied())
        .unwrap_or(handle)
}
pub fn duplicate(handle: usize) -> Option<usize> {
    with(|table| {
        let mut aliases = ALIASES.lock().unwrap();
        let aliases = aliases.get_or_insert_with(HashMap::new);
        let original = aliases.get(&handle).copied().unwrap_or(handle);
        if !table.contains_key(&original) {
            return None;
        }
        let alias = NEXT.fetch_add(16, Ordering::Relaxed);
        aliases.insert(alias, original);
        Some(alias)
    })
}

pub fn close(handle: usize) -> bool {
    with(|table| {
        let mut aliases = ALIASES.lock().unwrap();
        let aliases = aliases.get_or_insert_with(HashMap::new);
        if let Some(&original) = aliases.get(&handle) {
            if original == 0 {
                return false;
            }
            aliases.remove(&handle);
            if !aliases.values().any(|&value| value == original)
                && aliases.get(&original) == Some(&0)
            {
                aliases.remove(&original);
                table.remove(&original);
            }
            return true;
        }
        if !table.contains_key(&handle) {
            return false;
        }
        if aliases.values().any(|&value| value == handle) {
            aliases.insert(handle, 0);
        } else {
            table.remove(&handle);
        }
        true
    })
}
/// Names are process-local, shared with CreateMutex. Open returns an independent
/// handle: closing the creating handle must not destroy an opened mutex.
pub unsafe extern "win64" fn open_mutex_w(_access: u32, _inherit: i32, name: *const u16) -> usize {
    let Some(name) = (unsafe { text::string(name) }) else {
        error::set(87);
        return 0;
    };
    with(|table| {
        let found = table
            .iter()
            .find(|(_, entry)| entry.name.as_deref() == Some(name.as_str()));
        match found {
            Some((
                &original,
                Named {
                    object: Object::Mutex { .. },
                    ..
                },
            )) => {
                let handle = NEXT.fetch_add(16, Ordering::Relaxed);
                ALIASES
                    .lock()
                    .unwrap()
                    .get_or_insert_with(HashMap::new)
                    .insert(handle, original);
                handle
            }
            Some(_) => {
                error::set(6);
                0
            }
            None => {
                error::set(2);
                0
            }
        }
    })
}

pub unsafe extern "win64" fn create_semaphore_ex_w(
    _security: usize,
    initial: i32,
    maximum: i32,
    name: *const u16,
    _flags: u32,
    _access: u32,
) -> usize {
    if maximum <= 0 || initial < 0 || initial > maximum {
        error::set(87);
        return 0;
    }
    create(
        unsafe { text::string(name) },
        Object::Semaphore {
            count: initial,
            maximum,
        },
    )
}
pub unsafe extern "win64" fn create_semaphore_w(
    security: usize,
    initial: i32,
    maximum: i32,
    name: *const u16,
) -> usize {
    unsafe { create_semaphore_ex_w(security, initial, maximum, name, 0, 0) }
}
pub unsafe extern "win64" fn release_semaphore(
    handle: usize,
    count: i32,
    previous: *mut i32,
) -> i32 {
    let result = with(
        |t| match t.get_mut(&resolve(handle)).map(|n| &mut n.object) {
            Some(Object::Semaphore {
                count: current,
                maximum,
            }) => {
                if count <= 0 || *current > *maximum - count {
                    return Err(ERROR_TOO_MANY_POSTS);
                }
                let old = *current;
                *current += count;
                Ok(old)
            }
            _ => Err(6),
        },
    );
    match result {
        Ok(old) => {
            if !previous.is_null() {
                unsafe { *previous = old };
            }
            changed();
            1
        }
        Err(code) => {
            error::set(code);
            0
        }
    }
}
fn create_mutex(owned: bool, name: Option<String>) -> usize {
    let owner = owned.then(|| std::thread::current().id());
    create(
        name,
        Object::Mutex {
            owner,
            depth: owned as u32,
            abandoned: false,
        },
    )
}
pub unsafe extern "win64" fn create_mutex_w(
    _security: usize,
    owned: i32,
    name: *const u16,
) -> usize {
    create_mutex(owned != 0, unsafe { text::string(name) })
}
pub unsafe extern "win64" fn create_mutex_a(
    _security: usize,
    owned: i32,
    name: *const std::ffi::c_char,
) -> usize {
    let name = (!name.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
    });
    create_mutex(owned != 0, name)
}
pub extern "win64" fn release_mutex(handle: usize) -> i32 {
    let me = std::thread::current().id();
    let result = with(
        |t| match t.get_mut(&resolve(handle)).map(|n| &mut n.object) {
            Some(Object::Mutex { owner, depth, .. }) if *owner == Some(me) => {
                *depth -= 1;
                if *depth == 0 {
                    *owner = None;
                }
                Ok(())
            }
            Some(Object::Mutex { .. }) => Err(ERROR_NOT_OWNER),
            _ => Err(6),
        },
    );
    match result {
        Ok(()) => {
            changed();
            1
        }
        Err(code) => {
            error::set(code);
            0
        }
    }
}
/// CreateEventExW: CREATE_EVENT_MANUAL_RESET (1), CREATE_EVENT_INITIAL_SET (2).
pub unsafe extern "win64" fn create_event_ex_w(
    security: usize,
    name: *const u16,
    flags: u32,
    _access: u32,
) -> usize {
    let _ = security;
    events::create(flags & 1 != 0, flags & 2 != 0, unsafe { text::string(name) })
}

/// Is `handle` signalled? With `take`, acquire it (auto-reset events reset,
/// semaphores decrement, mutexes become owned). None: not a waitable handle.
fn check(handle: usize, take: bool) -> Option<bool> {
    if let Some(signalled) = events::check(handle, take) {
        return Some(signalled);
    }
    if let Some(exited) = threads::exited(handle) {
        return Some(exited);
    }
    let me = std::thread::current().id();
    with(|t| {
        Some(match &mut t.get_mut(&resolve(handle))?.object {
            Object::Semaphore { count, .. } => {
                let free = *count > 0;
                if free && take {
                    *count -= 1;
                }
                free
            }
            Object::Mutex { owner, depth, abandoned } => {
                let free = owner.is_none_or(|o| o == me);
                if free && take {
                    *owner = Some(me);
                    *depth += 1;
                    if std::mem::take(abandoned) {
                        TOOK_ABANDONED.with(|t| t.set(true));
                    }
                }
                free
            }
        })
    })
}
thread_local! {
    /// Set when this thread's last acquisition took an abandoned mutex.
    static TOOK_ABANDONED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
/// WAIT_OBJECT_0 + i becomes WAIT_ABANDONED_0 + i when the acquisition took an
/// abandoned mutex.
fn abandoned_result(result: u32) -> u32 {
    if result < 64 && TOOK_ABANDONED.with(|t| t.replace(false)) {
        WAIT_ABANDONED_0 + result
    } else {
        result
    }
}
/// A Windows thread ended: every mutex it still owns is released as abandoned.
pub fn thread_exited(thread: ThreadId) {
    let released = with(|table| {
        let mut any = false;
        for named in table.values_mut() {
            if let Object::Mutex { owner, depth, abandoned } = &mut named.object {
                if *owner == Some(thread) {
                    *owner = None;
                    *depth = 0;
                    *abandoned = true;
                    any = true;
                }
            }
        }
        any
    });
    if released {
        changed();
    }
}
/// Waits until `ready` (re-evaluated after every change) or the timeout.
fn wait_until(ms: u32, mut ready: impl FnMut() -> Option<u32>) -> u32 {
    super::iocp::release_worker();
    let deadline = (ms != u32::MAX).then(|| Instant::now() + Duration::from_millis(ms as u64));
    let mut generation = GENERATION.lock().unwrap();
    loop {
        if let Some(result) = ready() {
            return result;
        }
        match deadline {
            None => generation = CHANGED.wait(generation).unwrap(),
            Some(at) => {
                let left = at.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return WAIT_TIMEOUT;
                }
                // Events and threads signal through their own condition variables
                // too; a bounded wait keeps a missed wake-up from hanging.
                generation = CHANGED
                    .wait_timeout(generation, left.min(Duration::from_millis(10)))
                    .unwrap()
                    .0;
            }
        }
    }
}
/// An alertable wait: queued APCs run first, or as soon as one is queued, and the
/// wait returns WAIT_IO_COMPLETION.
pub fn wait_alertable(ms: u32, mut ready: impl FnMut() -> Option<u32>) -> u32 {
    super::iocp::release_worker();
    if apc::run() {
        return apc::WAIT_IO_COMPLETION;
    }
    let result = wait_until(ms, || {
        if apc::pending() {
            Some(apc::WAIT_IO_COMPLETION)
        } else {
            ready()
        }
    });
    if result == apc::WAIT_IO_COMPLETION {
        apc::run();
    }
    result
}
/// WaitForSingleObjectEx with bAlertable.
pub fn wait_single_alertable(handle: usize, ms: u32) -> u32 {
    if check(handle, false).is_none() {
        error::set(6);
        return WAIT_FAILED;
    }
    TOOK_ABANDONED.with(|t| t.set(false));
    abandoned_result(wait_alertable(ms, || check(handle, true)?.then_some(WAIT_OBJECT_0)))
}
/// WaitForSingleObject for semaphores and mutexes; None if not one.
pub fn wait_single(handle: usize, ms: u32) -> Option<u32> {
    with(|t| t.contains_key(&resolve(handle))).then(|| {
        TOOK_ABANDONED.with(|t| t.set(false));
        abandoned_result(wait_until(ms, || check(handle, true)?.then_some(WAIT_OBJECT_0)))
    })
}
pub unsafe extern "win64" fn wait_multiple(
    count: u32,
    handles: *const usize,
    all: i32,
    ms: u32,
) -> u32 {
    unsafe { wait_multiple_ex(count, handles, all, ms, 0) }
}
pub unsafe extern "win64" fn wait_multiple_ex(
    count: u32,
    handles: *const usize,
    all: i32,
    ms: u32,
    alertable: i32,
) -> u32 {
    if count == 0 || count > 64 || handles.is_null() {
        error::set(87);
        return WAIT_FAILED;
    }
    let handles = unsafe { std::slice::from_raw_parts(handles, count as usize) };
    if handles.iter().any(|&h| check(h, false).is_none()) {
        error::set(6);
        return WAIT_FAILED;
    }
    let ready = || {
        if all != 0 {
            if handles.iter().all(|&h| check(h, false) == Some(true)) {
                handles.iter().for_each(|&h| {
                    check(h, true);
                });
                return Some(WAIT_OBJECT_0);
            }
            None
        } else {
            handles
                .iter()
                .position(|&h| check(h, true) == Some(true))
                .map(|i| WAIT_OBJECT_0 + i as u32)
        }
    };
    TOOK_ABANDONED.with(|t| t.set(false));
    abandoned_result(if alertable != 0 {
        wait_alertable(ms, ready)
    } else {
        wait_until(ms, ready)
    })
}
