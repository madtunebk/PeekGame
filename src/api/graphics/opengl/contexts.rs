use super::{fail, pixel};
use crate::host::graphics::glx;
use std::{
    cell::Cell,
    collections::HashMap,
    ffi::c_void,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
    thread::ThreadId,
};
#[derive(Clone)]
pub(super) struct Context {
    pub native: usize,
    pub display: usize,
    pub config: usize,
    pub owner: Option<ThreadId>,
    pub interval: i32,
}
pub(super) static CONTEXTS: LazyLock<Mutex<HashMap<usize, Context>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT: AtomicUsize = AtomicUsize::new(0x71000000);
thread_local! {static CURRENT:Cell<(usize,usize)>=const{Cell::new((0,0))};}
pub(super) fn current() -> (usize, usize) {
    CURRENT.with(Cell::get)
}
pub(crate) extern "win64" fn get_current_context() -> usize {
    current().0
}
pub(crate) extern "win64" fn get_current_dc() -> usize {
    current().1
}
pub(crate) unsafe extern "win64" fn create_context(dc: usize) -> usize {
    unsafe { create_context_attribs(dc, 0, std::ptr::null()) }
}
pub(crate) unsafe extern "win64" fn create_context_attribs(
    dc: usize,
    share: usize,
    attrs: *const i32,
) -> usize {
    let Some(f) = pixel::format(dc) else {
        return fail(2000, 0);
    };
    let mut parsed = Vec::new();
    if !attrs.is_null() {
        let mut ended = false;
        for i in 0..128 {
            let key = unsafe { *attrs.add(i * 2) };
            if key == 0 {
                ended = true;
                break;
            }
            let val = unsafe { *attrs.add(i * 2 + 1) };
            match key {
                0x2091 if val >= 1 => {}
                0x2092 if val >= 0 => {}
                0x2094 if val & !3 == 0 => {}
                0x9126 if val == 1 || val == 2 => {}
                0x2093 if val == 0 => continue,
                _ => return fail(87, 0),
            }
            parsed.extend([key, val]);
        }
        if !ended {
            return fail(87, 0);
        }
        parsed.push(0);
    }
    // Serialize native creation against deletion of the shared context.
    let mut all = CONTEXTS.lock().unwrap();
    let shared = if share == 0 {
        0
    } else {
        let Some(c) = all
            .get(&share)
            .filter(|c| c.display == f.display && c.config == f.config)
        else {
            return fail(6, 0);
        };
        c.native
    };
    let native = unsafe {
        glx::peekgame_glx_create(
            f.display as *mut c_void,
            f.config as *mut c_void,
            shared as *mut c_void,
            if attrs.is_null() {
                std::ptr::null()
            } else {
                parsed.as_ptr()
            },
        )
    };
    if native.is_null() {
        return fail(2000, 0);
    }
    let handle = NEXT.fetch_add(8, Ordering::Relaxed);
    all.insert(
        handle,
        Context {
            native: native as usize,
            display: f.display,
            config: f.config,
            owner: None,
            interval: 0,
        },
    );
    handle
}
pub(crate) unsafe extern "win64" fn make_current(dc: usize, handle: usize) -> i32 {
    let tid = std::thread::current().id();
    // Keep ownership changes serialized with creation/deletion. GLX make-current
    // does not invoke application GL debug callbacks.
    let mut all = CONTEXTS.lock().unwrap();
    let old = current().0;
    if handle == 0 {
        if let Some(c) = all.get_mut(&old) {
            if unsafe { glx::glXMakeCurrent(c.display as *mut c_void, 0, std::ptr::null_mut()) }
                == 0
            {
                return fail(6, 0);
            }
            c.owner = None;
        }
        CURRENT.with(|c| c.set((0, 0)));
        return 1;
    }
    let Some(f) = pixel::format(dc) else {
        return fail(2000, 0);
    };
    let Some(c) = all.get(&handle) else {
        return fail(6, 0);
    };
    if c.owner.is_some_and(|t| t != tid) {
        return fail(170, 0);
    }
    if c.display != f.display || c.config != f.config {
        return fail(2000, 0);
    }
    if unsafe { glx::glXMakeCurrent(f.display as *mut c_void, f.window, c.native as *mut c_void) }
        == 0
    {
        return fail(2000, 0);
    }
    if let Some(c) = all.get_mut(&old) {
        c.owner = None
    }
    all.get_mut(&handle).unwrap().owner = Some(tid);
    CURRENT.with(|c| c.set((handle, dc)));
    1
}
pub(crate) unsafe extern "win64" fn delete_context(handle: usize) -> i32 {
    let tid = std::thread::current().id();
    let mut all = CONTEXTS.lock().unwrap();
    let Some(c) = all.get(&handle) else {
        return fail(6, 0);
    };
    if c.owner.is_some_and(|t| t != tid) {
        return fail(170, 0);
    }
    if current().0 == handle {
        if unsafe { glx::glXMakeCurrent(c.display as *mut c_void, 0, std::ptr::null_mut()) } == 0 {
            return fail(6, 0);
        }
        CURRENT.with(|c| c.set((0, 0)));
    }
    let c = all.remove(&handle).unwrap();
    drop(all);
    unsafe { glx::glXDestroyContext(c.display as *mut c_void, c.native as *mut c_void) };
    1
}
pub(crate) unsafe extern "win64" fn swap_buffers(dc: usize) -> i32 {
    if current().1 != dc || current().0 == 0 {
        return fail(6, 0);
    }
    let Some(f) = pixel::format(dc) else {
        return fail(2000, 0);
    };
    unsafe { glx::glXSwapBuffers(f.display as *mut c_void, f.window) };
    1
}
pub(crate) extern "win64" fn share_lists(_a: usize, _b: usize) -> i32 {
    fail(50, 0)
}

/// Release native context ownership before a Windows worker thread finishes.
pub(crate) fn release_thread() {
    unsafe {
        make_current(0, 0);
    }
}
