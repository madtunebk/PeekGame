use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{Arc, Mutex, OnceLock},
};
unsafe extern "C" {
    fn pthread_mutexattr_init(a: *mut u64) -> i32;
    fn pthread_mutexattr_settype(a: *mut u64, kind: i32) -> i32;
    fn pthread_mutexattr_destroy(a: *mut u64) -> i32;
    fn pthread_mutex_init(m: *mut u64, a: *const u64) -> i32;
    fn pthread_mutex_destroy(m: *mut u64) -> i32;
    fn pthread_mutex_lock(m: *mut u64) -> i32;
    fn pthread_mutex_unlock(m: *mut u64) -> i32;
    fn pthread_mutex_trylock(m: *mut u64) -> i32;
}
struct Critical(std::cell::UnsafeCell<[u64; 8]>);
// pthread mutex protects its own storage; the global map protects lifetime.
unsafe impl Send for Critical {}
unsafe impl Sync for Critical {}
impl Drop for Critical {
    fn drop(&mut self) {
        unsafe {
            pthread_mutex_destroy(self.0.get().cast());
        }
    }
}
static SECTIONS: OnceLock<Mutex<HashMap<usize, Arc<Critical>>>> = OnceLock::new();
fn sections() -> &'static Mutex<HashMap<usize, Arc<Critical>>> {
    SECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn get(p: *mut c_void) -> Arc<Critical> {
    sections()
        .lock()
        .unwrap()
        .get(&(p as usize))
        .cloned()
        .unwrap_or_else(|| stop("critical section used before initialization or after deletion"))
}
fn stop(message: &str) -> ! {
    eprintln!("STOP: {message}");
    std::process::exit(78)
}
pub unsafe extern "win64" fn initialize(p: *mut c_void) {
    if unsafe { initialize_spin(p, 0) } == 0 {
        stop("InitializeCriticalSection allocation failed");
    }
}
pub unsafe extern "win64" fn initialize_spin(p: *mut c_void, spin: u32) -> i32 {
    if p.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    let value = Arc::new(Critical(std::cell::UnsafeCell::new([0; 8])));
    let mut attr = 0u64;
    unsafe {
        if pthread_mutexattr_init(&mut attr) != 0 {
            crate::api::system::error::set(8);
            return 0;
        }
        let code = pthread_mutexattr_settype(&mut attr, 1);
        let initialized = if code == 0 {
            pthread_mutex_init(value.0.get().cast(), &attr)
        } else {
            code
        };
        pthread_mutexattr_destroy(&mut attr);
        if initialized != 0 {
            crate::api::system::error::set(8);
            return 0;
        }
    }
    let mut map = sections().lock().unwrap();
    if map.contains_key(&(p as usize)) {
        stop("critical section initialized twice");
    }
    unsafe {
        std::ptr::write_bytes(p, 0, 40);
        p.cast::<u8>().add(8).cast::<i32>().write(-1);
        p.cast::<u8>().add(32).cast::<u64>().write(spin as u64);
    }
    map.insert(p as usize, value);
    1
}
pub extern "win64" fn enter(p: *mut c_void) {
    let value = get(p);
    if unsafe { pthread_mutex_lock(value.0.get().cast()) } != 0 {
        stop("EnterCriticalSection host lock failed");
    }
}
pub extern "win64" fn leave(p: *mut c_void) {
    let value = get(p);
    if unsafe { pthread_mutex_unlock(value.0.get().cast()) } != 0 {
        stop("LeaveCriticalSection without ownership");
    }
}
pub extern "win64" fn try_enter(p: *mut c_void) -> i32 {
    let value = get(p);
    match unsafe { pthread_mutex_trylock(value.0.get().cast()) } {
        0 => 1,
        16 => 0,
        _ => stop("TryEnterCriticalSection host error"),
    }
}
pub extern "win64" fn delete(p: *mut c_void) {
    if sections().lock().unwrap().remove(&(p as usize)).is_none() {
        stop("DeleteCriticalSection on uninitialized/deleted object");
    }
}
