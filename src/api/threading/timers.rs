//! Waitable timers: an event signalled when the due time comes (and again every
//! period). A timer thread sleeps until then; SetWaitableTimer and
//! CancelWaitableTimer replace the schedule.
use super::events;
use crate::api::system::error;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
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
fn create(manual: bool) -> usize {
    let handle = unsafe { events::create_w(std::ptr::null(), manual as i32, 0, std::ptr::null()) };
    if handle != 0 {
        timers().lock().unwrap().insert(
            handle,
            Timer {
                manual,
                generation: 0,
            },
        );
    }
    handle
}
/// CREATE_WAITABLE_TIMER_MANUAL_RESET (1); HIGH_RESOLUTION (2) changes nothing here.
pub extern "win64" fn create_ex_w(
    _security: usize,
    _name: *const u16,
    flags: u32,
    _access: u32,
) -> usize {
    create(flags & 1 != 0)
}
pub extern "win64" fn create_w(_security: usize, manual: i32, _name: *const u16) -> usize {
    create(manual != 0)
}
/// SetWaitableTimer: due time in 100 ns units (negative: relative; positive: an
/// absolute FILETIME), period in milliseconds. Completion routines are not run.
pub unsafe extern "win64" fn set(
    handle: usize,
    due: *const i64,
    period: i32,
    routine: usize,
    _argument: usize,
    _resume: i32,
) -> i32 {
    if due.is_null() {
        error::set(87);
        return 0;
    }
    if routine != 0 {
        static WARNED: std::sync::Once = std::sync::Once::new();
        WARNED.call_once(|| eprintln!("SetWaitableTimer: completion routines are not run"));
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
    let due = unsafe { *due };
    let delay = if due < 0 {
        Duration::from_nanos((-due) as u64 * 100)
    } else {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let at = Duration::from_nanos((due as u64).saturating_sub(116_444_736_000_000_000) * 100);
        at.saturating_sub(now)
    };
    let period = Duration::from_millis(period.max(0) as u64);
    let spawned = std::thread::Builder::new()
        .name("waitable-timer".into())
        .spawn(move || {
            let mut at = Instant::now() + delay;
            loop {
                std::thread::sleep(at.saturating_duration_since(Instant::now()));
                let current = timers().lock().unwrap().get(&handle).map(|t| t.generation);
                if current != Some(generation) {
                    return; // replaced, cancelled or closed
                }
                events::set(handle);
                if period.is_zero() {
                    return;
                }
                at += period;
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
    routine: usize,
    argument: usize,
    _reason: usize,
    _delay: u32,
) -> i32 {
    unsafe { set(handle, due, period, routine, argument, 0) }
}
pub extern "win64" fn cancel(handle: usize) -> i32 {
    match timers().lock().unwrap().get_mut(&handle) {
        Some(t) => {
            t.generation += 1;
            1
        }
        None => {
            error::set(6);
            0
        }
    }
}
