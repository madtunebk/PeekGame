use crate::api::system::error;
use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};
struct Event {
    manual: bool,
    state: Mutex<bool>,
    changed: Condvar,
}
static EVENTS: OnceLock<Mutex<HashMap<usize, Arc<Event>>>> = OnceLock::new();
static NEXT: AtomicUsize = AtomicUsize::new(0x65000000);
fn events() -> &'static Mutex<HashMap<usize, Arc<Event>>> {
    EVENTS.get_or_init(|| Mutex::new(HashMap::new()))
}
/// Named events share a process-local namespace (as named mutexes and semaphores
/// do); "Local\\" is the session namespace, the same as no prefix here. The table
/// holds weak references: a name lives as long as one of its handles.
static NAMES: OnceLock<Mutex<HashMap<String, std::sync::Weak<Event>>>> = OnceLock::new();
fn names() -> &'static Mutex<HashMap<String, std::sync::Weak<Event>>> {
    NAMES.get_or_init(|| Mutex::new(HashMap::new()))
}
fn normalize(name: String) -> String {
    name.strip_prefix("Local\\").map(str::to_owned).unwrap_or(name)
}
fn insert(event: Arc<Event>) -> usize {
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    events().lock().unwrap().insert(handle, event);
    handle
}
/// CreateEvent: security attributes only concern child-process inheritance and are
/// accepted. An existing name returns a new handle to that event with
/// ERROR_ALREADY_EXISTS (its state and reset mode unchanged); a name used by a mutex
/// or semaphore is ERROR_INVALID_HANDLE.
pub(crate) fn create(manual: bool, initial: bool, name: Option<String>) -> usize {
    let name = name.filter(|n| !n.is_empty()).map(normalize);
    if let Some(name) = &name {
        if super::objects::name_taken(name) {
            error::set(6);
            return 0;
        }
        let mut names = names().lock().unwrap();
        if let Some(existing) = names.get(name).and_then(std::sync::Weak::upgrade) {
            drop(names);
            let handle = insert(existing);
            error::set(183); // ERROR_ALREADY_EXISTS
            return handle;
        }
        let event = Arc::new(Event { manual, state: Mutex::new(initial), changed: Condvar::new() });
        names.insert(name.clone(), Arc::downgrade(&event));
        drop(names);
        error::set(0);
        return insert(event);
    }
    error::set(0);
    insert(Arc::new(Event { manual, state: Mutex::new(initial), changed: Condvar::new() }))
}
pub unsafe extern "win64" fn create_a(
    _security: *const c_void,
    manual: i32,
    initial: i32,
    name: *const u8,
) -> usize {
    let name = (!name.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(name.cast()) }.to_string_lossy().into_owned()
    });
    create(manual != 0, initial != 0, name)
}
pub unsafe extern "win64" fn create_w(
    _security: *const c_void,
    manual: i32,
    initial: i32,
    name: *const u16,
) -> usize {
    create(manual != 0, initial != 0, unsafe { crate::api::text::string(name) })
}
/// OpenEvent: a new handle to an existing named event (ERROR_FILE_NOT_FOUND if none;
/// ERROR_INVALID_HANDLE if the name belongs to a mutex or semaphore).
fn open(name: Option<String>) -> usize {
    let Some(name) = name.map(normalize) else {
        error::set(87);
        return 0;
    };
    if name == "SteamStart_SharedMemLock" {
        let existing = names().lock().unwrap().get(&name).and_then(std::sync::Weak::upgrade);
        if let Some(event) = existing {
            let h = insert(event);
            set(h);
            return h;
        }
        let h = create(true, true, Some("Local\\SteamStart_SharedMemLock".into()));
        if h != 0 {
            set(h);
            return h;
        }
    }
    let existing = names().lock().unwrap().get(&name).and_then(std::sync::Weak::upgrade);
    match existing {
        Some(event) => insert(event),
        None if super::objects::name_taken(&name) => {
            error::set(6);
            0
        }
        None => {
            error::set(2);
            0
        }
    }
}
pub unsafe extern "win64" fn open_w(_access: u32, _inherit: i32, name: *const u16) -> usize {
    open(unsafe { crate::api::text::string(name) })
}
pub unsafe extern "win64" fn open_a(_access: u32, _inherit: i32, name: *const u8) -> usize {
    open((!name.is_null()).then(|| unsafe { std::ffi::CStr::from_ptr(name.cast()) }.to_string_lossy().into_owned()))
}
fn get(handle: usize) -> Option<Arc<Event>> {
    let event = events().lock().unwrap().get(&handle).cloned();
    if event.is_none() {
        error::set(6);
    }
    event
}
pub extern "win64" fn set(handle: usize) -> i32 {
    let Some(event) = get(handle) else {
        return 0;
    };
    *event.state.lock().unwrap() = true;
    super::objects::changed();
    if event.manual {
        event.changed.notify_all();
    } else {
        event.changed.notify_one();
    }
    1
}
/// Is the event signalled? With `take`, an auto-reset event resets. None: not an event.
pub(crate) fn check(handle: usize, take: bool) -> Option<bool> {
    let event = events().lock().unwrap().get(&handle).cloned()?;
    let mut state = event.state.lock().unwrap();
    let signalled = *state;
    if signalled && take && !event.manual {
        *state = false;
    }
    Some(signalled)
}
pub extern "win64" fn reset(handle: usize) -> i32 {
    let Some(event) = get(handle) else {
        return 0;
    };
    *event.state.lock().unwrap() = false;
    1
}
pub fn duplicate(handle: usize) -> Option<usize> {
    let event = events().lock().unwrap().get(&handle).cloned()?;
    let target = NEXT.fetch_add(16, Ordering::Relaxed);
    events().lock().unwrap().insert(target, event);
    Some(target)
}
pub extern "win64" fn close(handle: usize) -> i32 {
    if handle == crate::api::system::compat::STEAM_PROCESS_HANDLE
        || crate::api::system::process::close_child_handle(handle)
        || crate::api::network::afd::close(handle)
        || super::iocp::close(handle)
        || events().lock().unwrap().remove(&handle).inspect(|_| super::timers::forget(handle)).is_some()
        || super::threads::close(handle)
        || crate::api::filesystem::files::close(handle)
        || super::objects::close(handle)
        || crate::api::system::toolhelp::close(handle)
        || crate::api::system::power::close(handle)
        || crate::api::system::security::close(handle)
        || crate::api::memory::mapping::close(handle)
    {
        1
    } else {
        error::set(6);
        0
    }
}
pub extern "win64" fn wait(handle: usize, milliseconds: u32) -> u32 {
    super::iocp::release_worker();
    if crate::api::memory::mapping::is_steam_response_event(handle) {
        set(handle);
        return 0; // WAIT_OBJECT_0
    }
    if let Some(res) = crate::api::system::process::wait_child_process(handle, milliseconds) {
        return res;
    }
    if let Some(result) = super::threads::wait(handle, milliseconds) {
        return result;
    }
    if let Some(result) = super::objects::wait_single(handle, milliseconds) {
        return result;
    }
    let Some(event) = get(handle) else {
        return u32::MAX;
    };
    let mut state = event.state.lock().unwrap();
    let start = Instant::now();
    while !*state {
        if milliseconds == u32::MAX {
            state = event.changed.wait(state).unwrap();
        } else {
            let duration =
                Duration::from_millis(milliseconds as u64).saturating_sub(start.elapsed());
            if duration.is_zero() {
                return 258;
            }
            let (value, _) = event.changed.wait_timeout(state, duration).unwrap();
            state = value;
        }
    }
    if !event.manual {
        *state = false;
    }
    0
}
pub extern "win64" fn wait_ex(handle: usize, milliseconds: u32, alertable: i32) -> u32 {
    if alertable != 0 {
        return super::objects::wait_single_alertable(handle, milliseconds);
    }
    wait(handle, milliseconds)
}
