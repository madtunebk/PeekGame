//! SRW locks and condition variables (ported from peek's kernel32_ext.inc).
//! The SRWLOCK word itself holds the state (0 = free, as SRWLOCK_INIT), so locks need
//! neither initialization nor destruction: -1 is an exclusive owner, n > 0 is n
//! shared owners. The CONDITION_VARIABLE word is a wake sequence. Blocking uses a
//! private futex on the word.
use crate::api::{system::error, threading::critical};
use std::{
    ffi::c_void,
    sync::atomic::{AtomicI32, Ordering},
};
unsafe extern "C" {
    fn syscall(number: i64, ...) -> i64;
    fn __errno_location() -> *mut i32;
}
const SYS_FUTEX: i64 = 202;
const FUTEX_WAIT_PRIVATE: i32 = 128;
const FUTEX_WAKE_PRIVATE: i32 = 129;
const ETIMEDOUT: i32 = 110;
const ERROR_TIMEOUT: u32 = 1460;
#[repr(C)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}

fn word<'a>(p: *mut c_void) -> &'a AtomicI32 {
    unsafe { AtomicI32::from_ptr(p.cast()) }
}
/// Waits while `*w == seen`; `ms` of u32::MAX waits forever. False on timeout.
fn futex_wait(w: &AtomicI32, seen: i32, ms: u32) -> bool {
    let timeout = Timespec {
        seconds: (ms / 1000) as i64,
        nanoseconds: (ms % 1000) as i64 * 1_000_000,
    };
    let limit = if ms == u32::MAX {
        std::ptr::null()
    } else {
        &timeout as *const Timespec
    };
    let rc = unsafe {
        syscall(
            SYS_FUTEX,
            w.as_ptr(),
            FUTEX_WAIT_PRIVATE,
            seen,
            limit,
            0usize,
            0,
        )
    };
    !(rc == -1 && unsafe { *__errno_location() } == ETIMEDOUT)
}
fn futex_wake(w: &AtomicI32, count: i32) {
    unsafe {
        syscall(
            SYS_FUTEX,
            w.as_ptr(),
            FUTEX_WAKE_PRIVATE,
            count,
            0usize,
            0usize,
            0,
        )
    };
}
fn stop(message: &str) -> ! {
    eprintln!("STOP: {message}");
    std::process::exit(78)
}

pub unsafe extern "win64" fn initialize(lock: *mut c_void) {
    unsafe { lock.cast::<u64>().write(0) };
}
pub extern "win64" fn acquire_exclusive(lock: *mut c_void) {
    let w = word(lock);
    loop {
        match w.compare_exchange(0, -1, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => return,
            Err(seen) => {
                futex_wait(w, seen, u32::MAX);
            }
        }
    }
}
pub extern "win64" fn acquire_shared(lock: *mut c_void) {
    let w = word(lock);
    loop {
        let seen = w.load(Ordering::Relaxed);
        if seen >= 0
            && w.compare_exchange(seen, seen + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        {
            return;
        }
        if seen < 0 {
            futex_wait(w, seen, u32::MAX);
        }
    }
}
pub extern "win64" fn release_exclusive(lock: *mut c_void) {
    let w = word(lock);
    if w.load(Ordering::Relaxed) != -1 {
        stop("ReleaseSRWLockExclusive on a lock not held exclusively");
    }
    w.store(0, Ordering::Release);
    futex_wake(w, i32::MAX);
}
pub extern "win64" fn release_shared(lock: *mut c_void) {
    let w = word(lock);
    let left = w.fetch_sub(1, Ordering::Release) - 1;
    if left < 0 {
        stop("ReleaseSRWLockShared on a lock not held shared");
    }
    if left == 0 {
        futex_wake(w, i32::MAX);
    }
}
pub extern "win64" fn try_acquire_exclusive(lock: *mut c_void) -> u8 {
    word(lock)
        .compare_exchange(0, -1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok() as u8
}
pub extern "win64" fn try_acquire_shared(lock: *mut c_void) -> u8 {
    let w = word(lock);
    let mut seen = w.load(Ordering::Relaxed);
    while (0..i32::MAX).contains(&seen) {
        match w.compare_exchange(seen, seen + 1, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => return 1,
            Err(now) => seen = now,
        }
    }
    0
}

pub unsafe extern "win64" fn initialize_condition(cv: *mut c_void) {
    unsafe { cv.cast::<u64>().write(0) };
}
pub extern "win64" fn wake_condition(cv: *mut c_void) {
    let w = word(cv);
    w.fetch_add(1, Ordering::Release);
    futex_wake(w, 1);
}
pub extern "win64" fn wake_all_condition(cv: *mut c_void) {
    let w = word(cv);
    w.fetch_add(1, Ordering::Release);
    futex_wake(w, i32::MAX);
}
/// Waits until woken or `ms` elapsed (spurious wakeups are allowed by the Windows
/// contract). False on timeout.
fn condition_wait(cv: *mut c_void, ms: u32, unlock: impl FnOnce(), lock: impl FnOnce()) -> bool {
    let w = word(cv);
    let seen = w.load(Ordering::Acquire);
    unlock();
    let woken = ms != 0 && futex_wait(w, seen, ms);
    lock();
    woken
}
pub extern "win64" fn sleep_condition_srw(
    cv: *mut c_void,
    lock: *mut c_void,
    ms: u32,
    flags: u32,
) -> i32 {
    if flags & !1 != 0 {
        error::set(87);
        return 0;
    }
    let woken = if flags & 1 != 0 {
        condition_wait(cv, ms, || release_shared(lock), || acquire_shared(lock))
    } else {
        condition_wait(
            cv,
            ms,
            || release_exclusive(lock),
            || acquire_exclusive(lock),
        )
    };
    if !woken {
        error::set(ERROR_TIMEOUT);
    }
    woken as i32
}
pub unsafe extern "win64" fn sleep_condition_cs(cv: *mut c_void, cs: *mut c_void, ms: u32) -> i32 {
    let woken = condition_wait(cv, ms, || critical::leave(cs), || critical::enter(cs));
    if !woken {
        error::set(ERROR_TIMEOUT);
    }
    woken as i32
}
/// InitializeCriticalSectionEx: flags only tune debug info and spinning.
pub unsafe extern "win64" fn initialize_critical_ex(
    cs: *mut c_void,
    spin: u32,
    _flags: u32,
) -> i32 {
    unsafe { critical::initialize_spin(cs, spin) }
}
