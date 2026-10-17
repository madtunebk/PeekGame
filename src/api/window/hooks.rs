//! Calling-thread WH_MOUSE hooks. Callbacks run outside all runtime locks.
use super::{
    state,
    types::{Message, Point},
    windows,
};
use crate::api::system::error;
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};
pub type HookProc = unsafe extern "win64" fn(i32, usize, isize) -> isize;
#[derive(Clone, Copy)]
struct Hook {
    handle: usize,
    thread: u32,
    proc: HookProc,
}
static HOOKS: Mutex<Vec<Hook>> = Mutex::new(Vec::new());
static NEXT: AtomicUsize = AtomicUsize::new(0x6c00_0000);
struct Chain {
    hooks: Vec<Hook>,
    next: usize,
}
thread_local! {static CHAINS:RefCell<Vec<Chain>>=const {RefCell::new(Vec::new())};}
pub extern "win64" fn install(
    kind: i32,
    proc: Option<HookProc>,
    _module: usize,
    thread: u32,
) -> usize {
    let Some(proc) = proc else {
        error::set(1426);
        return 0;
    };
    if kind != 7 || thread == 0 || thread != state::current_thread() {
        error::set(50);
        return 0;
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    HOOKS.lock().unwrap().push(Hook {
        handle,
        thread,
        proc,
    });
    handle
}
pub extern "win64" fn uninstall(handle: usize) -> i32 {
    let mut hooks = HOOKS.lock().unwrap();
    let Some(index) = hooks.iter().position(|h| h.handle == handle) else {
        error::set(1404);
        return 0;
    };
    hooks.remove(index);
    1
}
pub extern "win64" fn next(_handle: usize, code: i32, wp: usize, lp: isize) -> isize {
    loop {
        let hook = CHAINS.with(|chains| {
            let mut chains = chains.borrow_mut();
            let chain = chains.last_mut()?;
            let hook = chain.hooks.get(chain.next).copied()?;
            chain.next += 1;
            Some(hook)
        });
        let Some(hook) = hook else {
            return 0;
        };
        if !HOOKS
            .lock()
            .unwrap()
            .iter()
            .any(|h| h.handle == hook.handle)
        {
            continue;
        }
        return unsafe { (hook.proc)(code, wp, lp) };
    }
}
#[repr(C)]
struct MouseHook {
    point: Point,
    window: usize,
    hit: u32,
    extra: usize,
}
pub(super) fn mouse(message: &Message, remove: bool) -> bool {
    if !(0x200..=0x20e).contains(&message.message) {
        return false;
    }
    let thread = state::current_thread();
    let hooks: Vec<_> = HOOKS
        .lock()
        .unwrap()
        .iter()
        .rev()
        .filter(|h| h.thread == thread)
        .copied()
        .collect();
    if hooks.is_empty() {
        return false;
    }
    let mut point = Point {
        x: message.lparam as i16 as i32,
        y: (message.lparam >> 16) as i16 as i32,
    };
    if message.window != 0 && !matches!(message.message, 0x20a | 0x20e) {
        unsafe {
            windows::client_to_screen(message.window, &mut point);
        }
    }
    let info = MouseHook {
        point,
        window: message.window,
        hit: 1,
        extra: 0,
    };
    CHAINS.with(|chains| chains.borrow_mut().push(Chain { hooks, next: 0 }));
    let result = next(
        0,
        if remove { 0 } else { 3 },
        message.message as usize,
        &info as *const _ as isize,
    );
    CHAINS.with(|chains| {
        chains.borrow_mut().pop();
    });
    remove && result != 0
}

/// SetWinEventHook: installs an event hook function for a range of events.
pub extern "win64" fn set_win_event_hook(
    _event_min: u32,
    _event_max: u32,
    _hmod: usize,
    _proc: usize,
    _id_process: u32,
    _id_thread: u32,
    _dw_flags: u32,
) -> usize {
    NEXT.fetch_add(16, Ordering::Relaxed)
}

/// UnhookWinEvent: removes an event hook function.
pub extern "win64" fn unhook_win_event(_hook: usize) -> i32 {
    1
}

