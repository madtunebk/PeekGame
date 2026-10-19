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
/// CRITICAL_SECTION fields: DebugInfo @0, LockCount @8, RecursionCount @12,
/// OwningThread @16, LockSemaphore @24, SpinCount @32. LockSemaphore holds a pointer
/// to our host mutex, so Enter/Leave need no global lookup; the table owns it (kept
/// alive until DeleteCriticalSection or re-initialization).
const LOCK_COUNT: usize = 8;
const RECURSION: usize = 12;
const OWNER: usize = 16;
const HOST: usize = 24;
fn get(p: *mut c_void) -> Arc<Critical> {
    if p.is_null() {
        stop("null critical section");
    }
    if let Some(value) = sections().lock().unwrap().get(&(p as usize)).cloned() {
        return value;
    }
    if unsafe { initialize_spin(p, 0) } == 0 {
        stop("critical section used before initialization or after deletion");
    }
    sections().lock().unwrap().get(&(p as usize)).cloned().unwrap()
}
/// Ownership fields programs read (OwningThread == GetCurrentThreadId() checks);
/// updated only while holding the lock. LockCount: -1 free, -2 held (Vista encoding).
fn mark(p: *mut c_void, entered: bool) {
    unsafe {
        let base = p.cast::<u8>();
        let depth = base.add(RECURSION).cast::<i32>();
        if entered {
            *depth += 1;
            base.add(OWNER).cast::<usize>().write(crate::api::system::startup::thread_id() as usize);
            base.add(LOCK_COUNT).cast::<i32>().write(-2);
        } else {
            *depth -= 1;
            if *depth == 0 {
                base.add(OWNER).cast::<usize>().write(0);
                base.add(LOCK_COUNT).cast::<i32>().write(-1);
            }
        }
    }
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
    // Memory holding a critical section may be freed without DeleteCriticalSection
    // and reused for a new one; Windows simply re-initializes, and so do we (the
    // old pthread mutex goes when its last user drops it).
    let mut map = sections().lock().unwrap();
    unsafe {
        std::ptr::write_bytes(p, 0, 40);
        p.cast::<u8>().add(LOCK_COUNT).cast::<i32>().write(-1);
        p.cast::<u8>().add(HOST).cast::<*const Critical>().write(Arc::as_ptr(&value));
        p.cast::<u8>().add(32).cast::<u64>().write(spin as u64);
    }
    map.insert(p as usize, value);
    1
}
pub extern "win64" fn enter(p: *mut c_void) {
    let value = get(p);
    let code = unsafe { pthread_mutex_lock(value.0.get().cast()) };
    if code != 0 {
        stop(&format!("EnterCriticalSection host lock failed: pthread error {code}, section {p:p}"));
    }
    mark(p, true);
}
pub extern "win64" fn leave(p: *mut c_void) {
    let value = get(p);
    mark(p, false);
    if unsafe { pthread_mutex_unlock(value.0.get().cast()) } != 0 {
        stop("LeaveCriticalSection without ownership");
    }
}
pub extern "win64" fn try_enter(p: *mut c_void) -> i32 {
    let value = get(p);
    match unsafe { pthread_mutex_trylock(value.0.get().cast()) } {
        0 => {
            mark(p, true);
            1
        }
        16 => 0,
        _ => stop("TryEnterCriticalSection host error"),
    }
}
pub extern "win64" fn delete(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    // A caller can delete during shutdown after its storage has already been
    // reclaimed. There is no host mutex left to destroy in that case.
    if sections().lock().unwrap().remove(&(p as usize)).is_some() {
        unsafe { p.cast::<u8>().add(HOST).cast::<usize>().write(0) };
    }
}
pub unsafe extern "win64" fn set_spin_count(p: *mut c_void, spin: u32) -> u32 {
    if p.is_null() {
        return 0;
    }
    let old = unsafe { p.cast::<u8>().add(32).cast::<u64>().read() } as u32;
    unsafe {
        p.cast::<u8>().add(32).cast::<u64>().write(spin as u64);
    }
    old
}
