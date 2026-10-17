//! Power-request object lifetime. Host suspend/display inhibition is not implemented.
use super::error;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};
static NEXT: AtomicUsize = AtomicUsize::new(0x6e00_0000);
static REQUESTS: OnceLock<Mutex<HashMap<usize, String>>> = OnceLock::new();
fn requests() -> &'static Mutex<HashMap<usize, String>> {
    REQUESTS.get_or_init(|| Mutex::new(HashMap::new()))
}
#[repr(C)]
pub struct ReasonContext {
    version: u32,
    flags: u32,
    reason: *const u16,
}
pub unsafe extern "win64" fn create(context: *const ReasonContext) -> usize {
    if context.is_null() {
        error::set(87);
        return usize::MAX;
    }
    let context = unsafe { &*context };
    if context.version != 0 || !matches!(context.flags, 1 | 2) {
        error::set(87);
        return usize::MAX;
    }
    // Detailed contexts reference Windows string-table resources, not plain UTF-16.
    if context.flags == 2 {
        error::set(50);
        return usize::MAX;
    }
    let reason = match unsafe { crate::host::paths::wide(context.reason) } {
        Ok(reason) => reason,
        Err(code) => {
            error::set(code);
            return usize::MAX;
        }
    };
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    requests().lock().unwrap().insert(handle, reason);
    handle
}
fn request(handle: usize, kind: u32) -> i32 {
    if !requests().lock().unwrap().contains_key(&handle) {
        error::set(6);
        return 0;
    }
    // DisplayRequired=0, SystemRequired=1, AwayModeRequired=2, ExecutionRequired=3.
    error::set(if kind > 3 { 87 } else { 50 });
    0
}
pub extern "win64" fn set(handle: usize, kind: u32) -> i32 {
    request(handle, kind)
}
pub extern "win64" fn clear(handle: usize, kind: u32) -> i32 {
    request(handle, kind)
}
pub fn close(handle: usize) -> bool {
    requests().lock().unwrap().remove(&handle).is_some()
}

#[repr(C)]
pub struct SystemPowerStatus {
    pub ac_line_status: u8,
    pub battery_flag: u8,
    pub battery_life_percent: u8,
    pub system_status_flag: u8,
    pub battery_life_time: u32,
    pub battery_full_life_time: u32,
}

pub unsafe extern "win64" fn get_system_power_status(status: *mut SystemPowerStatus) -> i32 {
    if status.is_null() {
        error::set(87);
        return 0;
    }
    unsafe {
        (*status).ac_line_status = 1;
        (*status).battery_flag = 128;
        (*status).battery_life_percent = 255;
        (*status).system_status_flag = 0;
        (*status).battery_life_time = !0;
        (*status).battery_full_life_time = !0;
    }
    1
}
