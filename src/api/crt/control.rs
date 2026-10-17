//! CRT startup initializers, recursive numbered locks and error reporting.
use std::{
    sync::{
        atomic::{AtomicI32, Ordering},
        Condvar, Mutex,
    },
    thread::ThreadId,
};
static LOCKS: Mutex<[(Option<ThreadId>, u32); 64]> = Mutex::new([(None, 0); 64]);
static CHANGED: Condvar = Condvar::new();
static ERROR_MODE: AtomicI32 = AtomicI32::new(0);
pub extern "win64" fn lock(number: i32) {
    if !(0..64).contains(&number) {
        eprintln!("STOP: CRT _lock invalid number {number}");
        std::process::exit(78);
    }
    let me = std::thread::current().id();
    let mut locks = LOCKS.lock().unwrap();
    while locks[number as usize].0.is_some_and(|owner| owner != me) {
        locks = CHANGED.wait(locks).unwrap();
    }
    locks[number as usize].0 = Some(me);
    locks[number as usize].1 += 1;
}
pub extern "win64" fn unlock(number: i32) {
    let me = std::thread::current().id();
    let mut locks = LOCKS.lock().unwrap();
    if !(0..64).contains(&number) || locks[number as usize].0 != Some(me) {
        eprintln!("STOP: CRT _unlock without ownership");
        std::process::exit(78);
    }
    let slot = &mut locks[number as usize];
    slot.1 -= 1;
    if slot.1 == 0 {
        slot.0 = None;
        CHANGED.notify_all();
    }
}
pub extern "win64" fn set_error_mode(mode: i32) -> i32 {
    if mode == 3 {
        return ERROR_MODE.load(Ordering::Relaxed);
    }
    if !(0..=2).contains(&mode) {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return -1;
    }
    ERROR_MODE.swap(mode, Ordering::Relaxed)
}
pub type Init = unsafe extern "win64" fn() -> i32;
pub unsafe extern "win64" fn init_term_e(
    mut begin: *const Option<Init>,
    end: *const Option<Init>,
) -> i32 {
    if begin == end {
        return 0;
    }
    if begin.is_null() || end.is_null() || (begin as usize) > (end as usize) {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return 22;
    }
    while begin < end {
        if let Some(f) = unsafe { *begin } {
            let result = unsafe { f() };
            if result != 0 {
                return result;
            }
        }
        begin = unsafe { begin.add(1) };
    }
    0
}
pub extern "win64" fn getpid() -> i32 {
    crate::api::system::startup::process_id() as i32
}
pub extern "win64" fn fpreset() {
    let mxcsr = 0x1f80u32;
    unsafe {
        std::arch::asm!("fninit","ldmxcsr [{value}]",value=in(reg)&mxcsr,options(nostack));
    }
}
pub unsafe extern "win64" fn assertion(
    message: *const std::ffi::c_char,
    file: *const std::ffi::c_char,
    line: u32,
) -> ! {
    fn text(p: *const std::ffi::c_char) -> String {
        if p.is_null() {
            "?".into()
        } else {
            unsafe { std::ffi::CStr::from_ptr(p) }
                .to_string_lossy()
                .into_owned()
        }
    }
    eprintln!(
        "Assertion failed: {}, file {}, line {line}",
        text(message),
        text(file)
    );
    std::process::abort()
}
