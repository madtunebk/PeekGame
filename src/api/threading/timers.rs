//! Waitable timers: an event signalled when the due time comes (and again every
//! period). One thread per armed timer sleeps until then, and leaves as soon as the
//! timer is set again, cancelled or closed. A completion routine is queued as an APC
//! to the thread that set the timer and runs in its next alertable wait.
use super::events;
use crate::api::system::{error, startup};
use std::{
    collections::HashMap,
    sync::{Condvar, Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Timer {
    manual: bool,
    generation: u64,
}
fn timers() -> &'static Mutex<HashMap<usize, Timer>> {
    static T: OnceLock<Mutex<HashMap<usize, Timer>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}
/// Wakes timer threads when a schedule changes.
static CHANGED: Condvar = Condvar::new();
fn create(manual: bool) -> usize {
    let handle = unsafe { events::create_w(std::ptr::null(), manual as i32, 0, std::ptr::null()) };
    if handle != 0 {
        timers().lock().unwrap().insert(handle, Timer { manual, generation: 0 });
    }
    handle
}
/// CREATE_WAITABLE_TIMER_MANUAL_RESET (1); HIGH_RESOLUTION (2) changes nothing here.
pub extern "win64" fn create_ex_w(_security: usize, _name: *const u16, flags: u32, _access: u32) -> usize {
    create(flags & 1 != 0)
}
pub extern "win64" fn create_w(_security: usize, manual: i32, _name: *const u16) -> usize {
    create(manual != 0)
}
/// CloseHandle of a timer: its schedule ends with it.
pub(crate) fn forget(handle: usize) {
    if timers().lock().unwrap().remove(&handle).is_some() {
        CHANGED.notify_all();
    }
}
/// SetWaitableTimer: due time in 100 ns units (negative: relative; positive: an
/// absolute FILETIME), period in milliseconds, optional completion routine (APC).
pub unsafe extern "win64" fn set(
    handle: usize,
    due: *const i64,
    period: i32,
    routine: Option<super::apc::TimerRoutine>,
    argument: usize,
    _resume: i32,
) -> i32 {
    if due.is_null() || period < 0 {
        error::set(87);
        return 0;
    }
    let generation = {
        let mut list = timers().lock().unwrap();
        let Some(timer) = list.get_mut(&handle) else {
            error::set(6);
            return 0;
        };
        timer.generation += 1;
        if timer.manual {
            events::reset(handle);
        }
        timer.generation
    };
    CHANGED.notify_all();
    let due = unsafe { *due };
    let now_filetime = || {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        116_444_736_000_000_000 + now.as_nanos() as u64 / 100
    };
    let delay = if due < 0 {
        Duration::from_nanos((-due) as u64 * 100)
    } else {
        Duration::from_nanos((due as u64).saturating_sub(now_filetime()) * 100)
    };
    let period = Duration::from_millis(period as u64);
    let owner = startup::thread_id();
    let spawned = std::thread::Builder::new().name("waitable-timer".into()).spawn(move || {
        let mut at = Instant::now() + delay;
        let mut list = timers().lock().unwrap();
        loop {
            let left = at.saturating_duration_since(Instant::now());
            if list.get(&handle).map(|t| t.generation) != Some(generation) {
                return; // set again, cancelled or closed
            }
            if !left.is_zero() {
                list = CHANGED.wait_timeout(list, left).unwrap().0;
                continue;
            }
            drop(list);
            events::set(handle);
            if let Some(routine) = routine {
                super::apc::queue_timer(owner, routine, argument, now_filetime());
            }
            if period.is_zero() {
                return;
            }
            at += period;
            list = timers().lock().unwrap();
        }
    });
    if spawned.is_err() {
        error::set(8);
        return 0;
    }
    1
}
pub unsafe extern "win64" fn set_ex(
    handle: usize,
    due: *const i64,
    period: i32,
    routine: Option<super::apc::TimerRoutine>,
    argument: usize,
    _reason: usize,
    _delay: u32,
) -> i32 {
    unsafe { set(handle, due, period, routine, argument, 0) }
}
pub extern "win64" fn cancel(handle: usize) -> i32 {
    let found = match timers().lock().unwrap().get_mut(&handle) {
        Some(t) => {
            t.generation += 1;
            true
        }
        None => false,
    };
    if found {
        CHANGED.notify_all();
        1
    } else {
        error::set(6);
        0
    }
}
