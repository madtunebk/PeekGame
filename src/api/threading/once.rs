//! INIT_ONCE (ported from peek's init_once.inc). The caller owns one pointer-sized
//! field; a side record per field lets competing synchronous callers wait.
use crate::api::system::error;
use std::{
    collections::HashMap,
    sync::{Condvar, Mutex},
    thread::{self, ThreadId},
};
const CHECK_ONLY: u32 = 1;
const ASYNC: u32 = 2;
const INIT_FAILED: u32 = 4;
const ERROR_IO_PENDING: u32 = 997;
#[derive(Clone, Copy, PartialEq)]
enum State {
    Uninitialized,
    Pending,
    AsyncPending,
    Complete,
}
struct Record {
    state: State,
    owner: Option<ThreadId>,
    context: usize,
    is_async: bool,
}
type Records = HashMap<usize, Record>;
static RECORDS: Mutex<Option<Records>> = Mutex::new(None);
static CHANGED: Condvar = Condvar::new();

fn with<T>(f: impl FnOnce(&mut Records) -> T) -> T {
    f(RECORDS.lock().unwrap().get_or_insert_with(HashMap::new))
}

pub unsafe extern "win64" fn initialize(once: *mut usize) {
    if once.is_null() {
        return;
    }
    with(|r| r.remove(&(once as usize)));
    CHANGED.notify_all();
    unsafe { *once = 0 };
}
pub unsafe extern "win64" fn begin(
    once: *mut usize,
    flags: u32,
    pending: *mut i32,
    context: *mut usize,
) -> i32 {
    if once.is_null()
        || pending.is_null()
        || flags & CHECK_ONLY != 0 && flags & ASYNC != 0
        || flags > 3
    {
        error::set(87);
        return 0;
    }
    if !context.is_null() {
        unsafe { *context = 0 };
    }
    let key = once as usize;
    let mut guard = RECORDS.lock().unwrap();
    loop {
        let records = guard.get_or_insert_with(HashMap::new);
        let state = records.get(&key).map_or(State::Uninitialized, |r| r.state);
        match state {
            State::Complete => {
                let record = &records[&key];
                if flags & CHECK_ONLY == 0 && record.is_async != (flags & ASYNC != 0) {
                    error::set(87);
                    return 0;
                }
                unsafe { *pending = 0 };
                if !context.is_null() {
                    unsafe { *context = record.context };
                }
                return 1;
            }
            _ if flags & CHECK_ONLY != 0 => {
                unsafe { *pending = 1 };
                error::set(ERROR_IO_PENDING);
                return 0;
            }
            State::Uninitialized => {
                let is_async = flags & ASYNC != 0;
                let state = if is_async {
                    State::AsyncPending
                } else {
                    State::Pending
                };
                records.insert(
                    key,
                    Record {
                        state,
                        owner: Some(thread::current().id()),
                        context: 0,
                        is_async,
                    },
                );
                unsafe {
                    *once = if is_async { 2 } else { 1 };
                    *pending = 1;
                }
                return 1;
            }
            State::AsyncPending if flags & ASYNC != 0 => {
                unsafe { *pending = 1 };
                return 1;
            }
            State::Pending if flags & ASYNC == 0 => guard = CHANGED.wait(guard).unwrap(),
            _ => {
                error::set(87);
                return 0;
            }
        }
    }
}
pub unsafe extern "win64" fn complete(once: *mut usize, flags: u32, context: usize) -> i32 {
    if once.is_null()
        || ![0, ASYNC, INIT_FAILED].contains(&flags)
        || context & 3 != 0
        || flags == INIT_FAILED && context != 0
    {
        error::set(87);
        return 0;
    }
    let ok = with(|records| {
        let expected = if flags == ASYNC {
            State::AsyncPending
        } else {
            State::Pending
        };
        let Some(record) = records.get_mut(&(once as usize)) else {
            return false;
        };
        if record.state != expected
            || expected == State::Pending && record.owner != Some(thread::current().id())
        {
            return false;
        }
        if flags == INIT_FAILED {
            records.remove(&(once as usize));
            unsafe { *once = 0 };
        } else {
            record.state = State::Complete;
            record.context = context;
            unsafe { *once = context | 3 };
        }
        true
    });
    CHANGED.notify_all();
    if !ok {
        error::set(87);
    }
    ok as i32
}
type Callback = extern "win64" fn(*mut usize, usize, *mut usize) -> i32;
pub unsafe extern "win64" fn execute(
    once: *mut usize,
    callback: Option<Callback>,
    parameter: usize,
    context: *mut usize,
) -> i32 {
    let Some(callback) = callback else {
        error::set(87);
        return 0;
    };
    let (mut pending, mut stored) = (0, 0usize);
    if unsafe { begin(once, 0, &mut pending, &mut stored) } == 0 {
        return 0;
    }
    if pending != 0 {
        if callback(once, parameter, &mut stored) == 0 {
            unsafe { complete(once, INIT_FAILED, 0) };
            return 0;
        }
        if unsafe { complete(once, 0, stored) } == 0 {
            return 0;
        }
    }
    if !context.is_null() {
        unsafe { *context = stored };
    }
    1
}
