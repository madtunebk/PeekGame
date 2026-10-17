//! MSVCRT clock/time and the nine-int Windows struct tm.
use crate::api::system::time::Tm;
use std::{
    cell::UnsafeCell,
    sync::OnceLock,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
unsafe extern "C" {
    fn gmtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
}
static START: OnceLock<Instant> = OnceLock::new();
pub extern "win64" fn clock() -> i32 {
    START.get_or_init(Instant::now).elapsed().as_millis() as i32
}
pub unsafe extern "win64" fn time64(out: *mut i64) -> i64 {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(-1, |d| d.as_secs() as i64);
    if !out.is_null() {
        unsafe {
            *out = t;
        }
    }
    t
}
pub unsafe extern "win64" fn gmtime64_s(out: *mut i32, time: *const i64) -> i32 {
    if out.is_null() {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return 22;
    }
    if time.is_null() || !(0..=32535215999).contains(&unsafe { *time }) {
        unsafe {
            for i in 0..9 {
                *out.add(i) = -1;
            }
            *super::stdio::errno() = 22;
        }
        return 22;
    }
    let mut host = Tm::default();
    if unsafe { gmtime_r(time, &mut host) }.is_null() {
        unsafe {
            *super::stdio::errno() = 22;
        }
        return 22;
    }
    unsafe {
        std::ptr::copy_nonoverlapping((&host as *const Tm).cast::<i32>(), out, 9);
    }
    0
}
thread_local! {static TM:UnsafeCell<[i32;9]>=const {UnsafeCell::new([0;9])};}
pub unsafe extern "win64" fn gmtime64(time: *const i64) -> *mut i32 {
    TM.with(|tm| {
        let out = tm.get().cast::<i32>();
        if unsafe { gmtime64_s(out, time) } == 0 {
            out
        } else {
            std::ptr::null_mut()
        }
    })
}
