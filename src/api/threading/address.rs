//! WaitOnAddress / WakeByAddressSingle / WakeByAddressAll with Windows' 1/2/4/8-byte
//! comparisons (ported from peek's wait_address.inc). Waiters are matched by exact
//! address, so a collision never wakes an unrelated waiter.
use crate::api::system::error;
use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
const ERROR_TIMEOUT: u32 = 1460;
struct Waiter {
    address: usize,
    signaled: Mutex<bool>,
    changed: Condvar,
}
static WAITERS: Mutex<Vec<Arc<Waiter>>> = Mutex::new(Vec::new());

unsafe fn value(address: usize, size: usize) -> u64 {
    use std::sync::atomic::*;
    unsafe {
        match size {
            1 => AtomicU8::from_ptr(address as *mut u8).load(Ordering::Acquire) as u64,
            2 => AtomicU16::from_ptr(address as *mut u16).load(Ordering::Acquire) as u64,
            4 => AtomicU32::from_ptr(address as *mut u32).load(Ordering::Acquire) as u64,
            _ => AtomicU64::from_ptr(address as *mut u64).load(Ordering::Acquire),
        }
    }
}
pub unsafe extern "win64" fn wait(
    address: usize,
    compare: *const u8,
    size: usize,
    timeout: u32,
) -> i32 {
    if address == 0
        || compare.is_null()
        || ![1, 2, 4, 8].contains(&size)
        || !address.is_multiple_of(size)
    {
        error::set(87);
        return 0;
    }
    let mut expected = [0u8; 8];
    unsafe { std::ptr::copy_nonoverlapping(compare, expected.as_mut_ptr(), size) };
    let expected = u64::from_le_bytes(expected);
    let waiter = {
        let mut waiters = WAITERS.lock().unwrap();
        if unsafe { value(address, size) } != expected {
            return 1;
        }
        if timeout == 0 {
            error::set(ERROR_TIMEOUT);
            return 0;
        }
        let waiter = Arc::new(Waiter {
            address,
            signaled: Mutex::new(false),
            changed: Condvar::new(),
        });
        waiters.push(waiter.clone());
        waiter
    };
    let deadline =
        (timeout != u32::MAX).then(|| Instant::now() + Duration::from_millis(timeout as u64));
    let mut signaled = waiter.signaled.lock().unwrap();
    while !*signaled {
        match deadline {
            None => signaled = waiter.changed.wait(signaled).unwrap(),
            Some(at) => {
                let left = at.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                signaled = waiter.changed.wait_timeout(signaled, left).unwrap().0;
            }
        }
    }
    let woken = *signaled;
    drop(signaled);
    WAITERS.lock().unwrap().retain(|w| !Arc::ptr_eq(w, &waiter));
    if !woken {
        error::set(ERROR_TIMEOUT);
    }
    woken as i32
}
fn wake(address: usize, all: bool) {
    let waiters = WAITERS.lock().unwrap();
    for waiter in waiters.iter().filter(|w| w.address == address) {
        let mut signaled = waiter.signaled.lock().unwrap();
        if *signaled {
            continue;
        }
        *signaled = true;
        waiter.changed.notify_one();
        if !all {
            return;
        }
    }
}
pub extern "win64" fn wake_single(address: usize) {
    wake(address, false);
}
pub extern "win64" fn wake_all(address: usize) {
    wake(address, true);
}
