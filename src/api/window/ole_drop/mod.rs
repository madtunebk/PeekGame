//! Xdnd local-file transfers delivered on the registered window's OLE thread.
use super::{queue, state};
use crate::host::x11::{xdnd::Event, Connection};
use std::{
    collections::{HashMap, VecDeque},
    ffi::c_void,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, LazyLock, Mutex,
    },
};
const INVALID: i32 = 0x80040102u32 as i32;
const NOT_REGISTERED: i32 = 0x80040100u32 as i32;
struct Target {
    pointer: usize,
    native: u64,
    owner: u32,
    generation: u64,
    connection: Arc<Connection>,
}
impl Target {
    unsafe fn entry(&self, index: usize) -> usize {
        unsafe {
            let vt = *(self.pointer as *const *const usize);
            *vt.add(index)
        }
    }
    fn ref_call(&self, index: usize) {
        unsafe {
            let f: extern "win64" fn(usize) -> u32 = std::mem::transmute(self.entry(index));
            f(self.pointer);
        }
    }
    fn leave(&self) {
        unsafe {
            let f: extern "win64" fn(usize) -> i32 = std::mem::transmute(self.entry(5));
            f(self.pointer);
        }
    }
    fn data_call(&self, index: usize, data: usize, keys: u32, point: i64, effect: &mut u32) -> i32 {
        unsafe {
            let f: extern "win64" fn(usize, usize, u32, i64, *mut u32) -> i32 =
                std::mem::transmute(self.entry(index));
            f(self.pointer, data, keys, point, effect)
        }
    }
    fn over(&self, keys: u32, point: i64, effect: &mut u32) -> i32 {
        unsafe {
            let f: extern "win64" fn(usize, u32, i64, *mut u32) -> i32 =
                std::mem::transmute(self.entry(4));
            f(self.pointer, keys, point, effect)
        }
    }
}
impl Drop for Target {
    fn drop(&mut self) {
        self.ref_call(2);
    }
}
static TARGETS: LazyLock<Mutex<HashMap<usize, Arc<Target>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT: AtomicU64 = AtomicU64::new(1);
static PENDING: Mutex<VecDeque<(u32, usize, Event)>> = Mutex::new(VecDeque::new());
fn target(hwnd: usize) -> Option<Arc<Target>> {
    TARGETS.lock().unwrap().get(&hwnd).cloned()
}
fn live(hwnd: usize, t: &Target) -> bool {
    TARGETS
        .lock()
        .unwrap()
        .get(&hwnd)
        .is_some_and(|v| v.generation == t.generation)
}
pub extern "win64" fn register(hwnd: usize, pointer: *mut c_void) -> i32 {
    let Ok((native, owner, destroying)) =
        state::with_window(hwnd, |w| (w.native, w.owner, w.destroying))
    else {
        return INVALID;
    };
    if native == 0 || destroying || owner != state::current_thread() {
        return INVALID;
    }
    if pointer.is_null() {
        return 0x80070057u32 as i32;
    }
    if !crate::api::com::apartment::ole_initialized() {
        return 0x8007000eu32 as i32;
    }
    if target(hwnd).is_some() {
        return 0x80040101u32 as i32;
    }
    let Ok(connection) = state::connection() else {
        return 0x80004005u32 as i32;
    };
    let t = Arc::new(Target {
        pointer: pointer as usize,
        native,
        owner,
        generation: NEXT.fetch_add(1, Ordering::Relaxed),
        connection,
    });
    t.ref_call(1);
    // AddRef can reenter and destroy or register the window.
    if !state::exists(hwnd) {
        return INVALID;
    }
    let inserted = {
        let mut all = TARGETS.lock().unwrap();
        if let std::collections::hash_map::Entry::Vacant(e) = all.entry(hwnd) {
            e.insert(t.clone());
            true
        } else {
            false
        }
    };
    if !inserted {
        return 0x80040101u32 as i32;
    }
    t.connection.xdnd_aware(native, true);
    0
}
pub extern "win64" fn revoke(hwnd: usize) -> i32 {
    let Ok(owner) = state::with_window(hwnd, |w| w.owner) else {
        return INVALID;
    };
    if owner != state::current_thread() {
        return INVALID;
    }
    if target(hwnd).is_none() {
        return NOT_REGISTERED;
    }
    forget(hwnd);
    0
}
pub(super) fn forget(hwnd: usize) {
    let old = TARGETS.lock().unwrap().remove(&hwnd);
    PENDING.lock().unwrap().retain(|(_, h, _)| *h != hwnd);
    if let Some(t) = old {
        t.connection.xdnd_aware(t.native, false);
        session::forget(hwnd, &t);
    }
}
pub(super) fn enqueue(hwnd: usize, event: Event) {
    if let Some(owner) = queue::owner(hwnd) {
        PENDING.lock().unwrap().push_back((owner, hwnd, event));
        queue::notify();
    }
}

mod session;
pub(super) use session::run_pending;
