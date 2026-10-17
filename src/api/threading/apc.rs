//! User-mode APCs: QueueUserAPC queues a call for a thread, which runs it in its
//! next alertable wait (SleepEx, WaitFor*ObjectEx with bAlertable); the wait then
//! returns WAIT_IO_COMPLETION.
use super::{objects, threads};
use crate::api::system::{error, startup};
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};
pub const WAIT_IO_COMPLETION: u32 = 0xC0;
type Routine = extern "win64" fn(usize);
/// PTIMERAPCROUTINE: context, due time low and high parts.
pub(crate) type TimerRoutine = extern "win64" fn(usize, u32, u32);
enum Apc {
    User(Routine, usize),
    Timer(TimerRoutine, usize, u64),
}
type Queues = HashMap<u32, VecDeque<Apc>>;
static QUEUES: Mutex<Option<Queues>> = Mutex::new(None);
pub unsafe extern "win64" fn queue(routine: Option<Routine>, thread: usize, data: usize) -> u32 {
    let (Some(routine), Some(id)) = (routine, threads::id_of(thread)) else {
        error::set(6);
        return 0;
    };
    QUEUES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .entry(id)
        .or_default()
        .push_back(Apc::User(routine, data));
    objects::changed();
    1
}
/// Queues a waitable timer's completion routine for the thread that set the timer.
pub(crate) fn queue_timer(thread: u32, routine: TimerRoutine, context: usize, due: u64) {
    QUEUES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .entry(thread)
        .or_default()
        .push_back(Apc::Timer(routine, context, due));
    objects::changed();
}
/// Does the calling thread have APCs queued?
pub fn pending() -> bool {
    let id = startup::thread_id();
    QUEUES
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|q| q.get(&id))
        .is_some_and(|q| !q.is_empty())
}
/// Runs the calling thread's queued APCs; true if any ran.
pub fn run() -> bool {
    let id = startup::thread_id();
    let mut ran = false;
    loop {
        let next = QUEUES
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|q| q.get_mut(&id))
            .and_then(|q| q.pop_front());
        match next {
            Some(Apc::User(routine, data)) => routine(data),
            Some(Apc::Timer(routine, context, due)) => routine(context, due as u32, (due >> 32) as u32),
            None => return ran,
        }
        ran = true;
    }
}
