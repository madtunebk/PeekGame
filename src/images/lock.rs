//! A recursive mutex: a DllMain may call LoadLibrary on the thread that holds it.
use std::{
    sync::{Condvar, Mutex},
    thread::{self, ThreadId},
};
pub struct Recursive {
    state: Mutex<(Option<ThreadId>, usize)>,
    free: Condvar,
}
pub struct Guard<'a>(&'a Recursive);
impl Recursive {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new((None, 0)),
            free: Condvar::new(),
        }
    }
    pub fn lock(&self) -> Guard<'_> {
        let me = thread::current().id();
        let mut state = self.state.lock().unwrap();
        while state.0.is_some_and(|owner| owner != me) {
            state = self.free.wait(state).unwrap();
        }
        state.0 = Some(me);
        state.1 += 1;
        Guard(self)
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.1 -= 1;
        if state.1 == 0 {
            state.0 = None;
            self.0.free.notify_one();
        }
    }
}
