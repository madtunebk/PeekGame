//! Fiber-local storage without fibers: one slot table per thread, like TLS, plus
//! each slot's callback, which runs for the slot's non-NULL value when FlsFree
//! releases the slot and when a Windows thread exits (ported from peek's
//! sync_objects.inc). Slots are as many as the program allocates.
use crate::api::system::error;
use std::{cell::RefCell, sync::Mutex};
type Callback = Option<extern "win64" fn(usize)>;
const OUT_OF_INDEXES: u32 = u32::MAX;
/// Per slot: allocated, and its callback.
static SLOTS: Mutex<Vec<Option<Callback>>> = Mutex::new(Vec::new());
thread_local! {static VALUES: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };}

fn callback(slot: u32) -> Option<Callback> {
    SLOTS.lock().unwrap().get(slot as usize).copied().flatten()
}
pub extern "win64" fn alloc(callback: Callback) -> u32 {
    let mut slots = SLOTS.lock().unwrap();
    let index = match slots.iter().position(Option::is_none) {
        Some(free) => free,
        None => {
            slots.push(None);
            slots.len() - 1
        }
    };
    slots[index] = Some(callback);
    VALUES.with(|v| {
        if let Some(value) = v.borrow_mut().get_mut(index) {
            *value = 0;
        }
    });
    u32::try_from(index).unwrap_or_else(|_| {
        error::set(8);
        OUT_OF_INDEXES
    })
}
pub extern "win64" fn set(slot: u32, value: usize) -> i32 {
    if callback(slot).is_none() {
        error::set(87);
        return 0;
    }
    VALUES.with(|v| {
        let mut v = v.borrow_mut();
        if v.len() <= slot as usize {
            v.resize(slot as usize + 1, 0);
        }
        v[slot as usize] = value;
    });
    1
}
pub extern "win64" fn get(slot: u32) -> usize {
    if callback(slot).is_none() {
        error::set(87);
        return 0;
    }
    error::set(0);
    VALUES.with(|v| v.borrow().get(slot as usize).copied().unwrap_or(0))
}
/// FlsGetValue2: as FlsGetValue, without touching the last error.
pub extern "win64" fn get2(slot: u32) -> usize {
    VALUES.with(|v| v.borrow().get(slot as usize).copied().unwrap_or(0))
}
pub extern "win64" fn free(slot: u32) -> i32 {
    let Some(callback) = callback(slot) else {
        error::set(87);
        return 0;
    };
    // Windows runs the callback for every thread's value; values of other threads
    // are not reachable here, so only the caller's is released.
    let value = VALUES.with(|v| v.borrow_mut().get_mut(slot as usize).map(std::mem::take));
    if let (Some(callback), Some(value)) = (callback, value.filter(|v| *v != 0)) {
        callback(value);
    }
    SLOTS.lock().unwrap()[slot as usize] = None;
    1
}
/// Thread exit: each slot's callback for the thread's non-NULL values.
pub fn thread_exit() {
    let values = VALUES.with(|v| std::mem::take(&mut *v.borrow_mut()));
    for (slot, value) in values.into_iter().enumerate() {
        if value == 0 {
            continue;
        }
        if let Some(Some(callback)) = callback(slot as u32) {
            callback(value);
        }
    }
}
