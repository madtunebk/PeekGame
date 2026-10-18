//! MSVCRT clock/time and the nine-int Windows struct tm.
use crate::api::system::time::Tm;
use std::{
    cell::UnsafeCell,
    sync::OnceLock,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
unsafe extern "C" {
    fn gmtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    fn timegm(tm: *mut Tm) -> i64;
    fn ctime(time: *const i64) -> *mut std::ffi::c_char;
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
pub unsafe extern "win64" fn localtime64_s(out: *mut i32, time: *const i64) -> i32 {
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
    if unsafe { localtime_r(time, &mut host) }.is_null() {
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
pub unsafe extern "win64" fn localtime64(time: *const i64) -> *mut i32 {
    TM.with(|tm| {
        let out = tm.get().cast::<i32>();
        if unsafe { localtime64_s(out, time) } == 0 {
            out
        } else {
            std::ptr::null_mut()
        }
    })
}
pub unsafe extern "win64" fn mktime64(tm: *mut i32) -> i64 {
    if tm.is_null() {
        return -1;
    }
    let mut host = Tm::default();
    unsafe {
        std::ptr::copy_nonoverlapping(tm, (&mut host as *mut Tm).cast::<i32>(), 9);
        timegm(&mut host)
    }
}
pub unsafe extern "win64" fn ctime64(time: *const i64) -> *mut std::ffi::c_char {
    if time.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { ctime(time) }
}
pub unsafe extern "win64" fn ftime64(out: *mut u8) -> i32 {
    if out.is_null() {
        return 22;
    }
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    unsafe {
        out.write_bytes(0, 16);
        (out as *mut i64).write_unaligned(d.as_secs() as i64);
        (out.add(8) as *mut u16).write_unaligned(d.subsec_millis() as u16);
    }
    0
}
pub unsafe extern "win64" fn timespec64_get(ts: *mut i64, base: i32) -> i32 {
    if ts.is_null() || base != 1 {
        return 0;
    }
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    unsafe {
        *ts = d.as_secs() as i64;
        *ts.add(1) = d.subsec_nanos() as i64;
    }
    base
}
pub unsafe extern "win64" fn timespec32_get(ts: *mut i32, base: i32) -> i32 {
    if ts.is_null() || base != 1 {
        return 0;
    }
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    unsafe {
        *ts = d.as_secs() as i32;
        *ts.add(1) = d.subsec_nanos() as i32;
    }
    base
}

