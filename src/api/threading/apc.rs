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
type Queues = HashMap<u32, VecDeque<(Routine, usize)>>;
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
        .push_back((routine, data));
    objects::changed();
    1
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
        let Some((routine, data)) = next else {
            return ran;
        };
        routine(data);
        ran = true;
    }
}
