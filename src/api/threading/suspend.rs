//! SuspendThread, ResumeThread, GetThreadContext and SetThreadContext for running
//! threads (after peek's suspend.inc). A suspended thread is stopped in a signal
//! handler: it saves its registers as a Windows CONTEXT in its slot, says so, and
//! waits on a futex until its suspend count drops to zero, then applies a context
//! SetThreadContext may have changed. Garbage collectors (Mono's) stop threads
//! this way to scan their stacks and registers. Slots use atomics only, so the
//! handler never takes a lock the stopped code might hold.
use super::threads;
use crate::{api::system::error, seh::Context};
use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
};
const SIGNAL: i32 = 36; // SIGRTMIN + 2
const RUNNING: u32 = 0;
const REQUESTED: u32 = 1;
const STOPPED: u32 = 2;
struct Slot {
    tid: AtomicU32,
    count: AtomicU32,
    state: AtomicU32,
    changed: AtomicBool,
    context: UnsafeCell<Context>,
}
unsafe impl Sync for Slot {}
const SLOTS: usize = 512;
static TABLE: std::sync::OnceLock<Box<[Slot]>> = std::sync::OnceLock::new();
fn table() -> &'static [Slot] {
    TABLE.get_or_init(|| {
        (0..SLOTS)
            .map(|_| Slot {
                tid: AtomicU32::new(0),
                count: AtomicU32::new(0),
                state: AtomicU32::new(RUNNING),
                changed: AtomicBool::new(false),
                context: UnsafeCell::new(unsafe { std::mem::zeroed() }),
            })
            .collect()
    })
}
fn slot(tid: u32, create: bool) -> Option<&'static Slot> {
    let t = table();
    if let Some(s) = t.iter().find(|s| s.tid.load(Ordering::Acquire) == tid) {
        return Some(s);
    }
    if !create {
        return None;
    }
    t.iter().find(|s| {
        s.tid
            .compare_exchange(0, tid, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    })
}
unsafe extern "C" {
    fn syscall(number: i64, ...) -> i64;
    fn getpid() -> i32;
    fn sigaction(signal: i32, action: *const SigAction, old: *mut SigAction) -> i32;
}
#[repr(C)]
struct SigAction {
    handler: usize,
    mask: [u64; 16],
    flags: i32,
    restorer: usize,
}
fn futex_wait(word: &AtomicU32, value: u32) {
    unsafe {
        syscall(
            202,
            word as *const AtomicU32,
            0, /* FUTEX_WAIT */
            value,
            std::ptr::null::<u8>(),
        )
    };
}
fn futex_wake(word: &AtomicU32) {
    unsafe {
        syscall(
            202,
            word as *const AtomicU32,
            1, /* FUTEX_WAKE */
            i32::MAX,
        )
    };
}
unsafe extern "C" fn on_suspend(_signal: i32, _info: *mut u8, u: *mut u8) {
    let tid = unsafe { syscall(186) } as u32;
    let Some(s) = slot(tid, false) else {
        return;
    };
    if s.state.load(Ordering::Acquire) != REQUESTED {
        return;
    }
    unsafe { crate::seh::context_from_signal(u, &mut *s.context.get()) };
    s.changed.store(false, Ordering::Release);
    s.state.store(STOPPED, Ordering::Release);
    futex_wake(&s.state);
    while s.state.load(Ordering::Acquire) == STOPPED {
        futex_wait(&s.state, STOPPED);
    }
    if s.changed.swap(false, Ordering::AcqRel) {
        unsafe { crate::seh::context_to_signal(&mut *s.context.get(), u) };
    }
}
fn install() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let action = SigAction {
            handler: on_suspend as *const () as usize,
            mask: [0; 16],
            flags: 4 | 0x1000_0000, // SA_SIGINFO | SA_RESTART
            restorer: 0,
        };
        unsafe { sigaction(SIGNAL, &action, std::ptr::null_mut()) };
    });
}
fn target(handle: usize) -> Option<u32> {
    let tid = threads::id_of(handle);
    if tid.is_none() {
        error::set(6);
    }
    tid
}
/// SuspendThread: the previous suspend count. A thread created suspended counts 1.
pub extern "win64" fn suspend(handle: usize) -> u32 {
    let Some(tid) = target(handle) else {
        return u32::MAX;
    };
    if tid == crate::api::system::startup::thread_id() {
        // Self-suspension would never return here; Windows allows it, programs
        // rarely use it.
        error::set(50);
        return u32::MAX;
    }
    if threads::created_suspended(handle) {
        return 1;
    }
    install();
    let Some(s) = slot(tid, true) else {
        error::set(8);
        return u32::MAX;
    };
    let previous = s.count.fetch_add(1, Ordering::AcqRel);
    if previous == 0 {
        s.state.store(REQUESTED, Ordering::Release);
        if unsafe { syscall(234, getpid(), tid as i32, SIGNAL) } != 0 {
            // The thread is gone.
            s.count.store(0, Ordering::Release);
            s.state.store(RUNNING, Ordering::Release);
            error::set(5);
            return u32::MAX;
        }
        while s.state.load(Ordering::Acquire) == REQUESTED {
            futex_wait(&s.state, REQUESTED);
        }
    }
    previous
}
/// ResumeThread: the previous suspend count; the thread runs again at zero.
pub extern "win64" fn resume(handle: usize) -> u32 {
    if threads::created_suspended(handle) {
        return threads::resume(handle);
    }
    let Some(tid) = target(handle) else {
        return u32::MAX;
    };
    let Some(s) = slot(tid, false) else {
        return 0;
    };
    let previous = s.count.load(Ordering::Acquire);
    if previous == 0 {
        return 0;
    }
    if s.count.fetch_sub(1, Ordering::AcqRel) == 1 {
        s.state.store(RUNNING, Ordering::Release);
        futex_wake(&s.state);
    }
    previous
}
/// GetThreadContext of a suspended thread (the current thread's own context is
/// RtlCaptureContext's job).
pub unsafe extern "win64" fn get_context(handle: usize, out: *mut Context) -> i32 {
    let Some(tid) = target(handle) else {
        return 0;
    };
    match slot(tid, false).filter(|s| s.state.load(Ordering::Acquire) == STOPPED) {
        Some(s) if !out.is_null() => {
            let flags = unsafe { (*out).flags };
            unsafe {
                out.write(*s.context.get());
                (*out).flags = flags;
            }
            1
        }
        _ => {
            error::set(if tid == crate::api::system::startup::thread_id() {
                50
            } else {
                5
            });
            0
        }
    }
}
pub unsafe extern "win64" fn set_context(handle: usize, context: *const Context) -> i32 {
    let Some(tid) = target(handle) else {
        return 0;
    };
    match slot(tid, false).filter(|s| s.state.load(Ordering::Acquire) == STOPPED) {
        Some(s) if !context.is_null() => {
            unsafe { *s.context.get() = *context };
            s.changed.store(true, Ordering::Release);
            1
        }
        _ => {
            error::set(5);
            0
        }
    }
}
