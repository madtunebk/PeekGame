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
pub unsafe extern "win64" fn create_a(
    security: *const c_void,
    manual: i32,
    initial: i32,
    name: *const u8,
) -> usize {
    if !security.is_null() || !name.is_null() {
        error::set(50);
        return 0;
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    events().lock().unwrap().insert(
        handle,
        Arc::new(Event {
            manual: manual != 0,
            state: Mutex::new(initial != 0),
            changed: Condvar::new(),
        }),
    );
    handle
}
pub unsafe extern "win64" fn create_w(
    security: *const c_void,
    manual: i32,
    initial: i32,
    name: *const u16,
) -> usize {
    unsafe { create_a(security, manual, initial, name.cast()) }
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
    if event.manual {
        event.changed.notify_all();
    } else {
        event.changed.notify_one();
    }
    1
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
    if events().lock().unwrap().remove(&handle).is_some() || super::threads::close(handle) {
        1
    } else {
        error::set(6);
        0
    }
}
pub extern "win64" fn wait(handle: usize, milliseconds: u32) -> u32 {
    if let Some(result) = super::threads::wait(handle, milliseconds) {
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
        error::set(50);
        return u32::MAX;
    }
    wait(handle, milliseconds)
}
