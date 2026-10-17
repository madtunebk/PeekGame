use std::{cell::RefCell, ffi::c_void};
thread_local! {static OLE:std::cell::Cell<u32>=const{std::cell::Cell::new(0)};}
pub(crate) fn ole_initialized() -> bool {
    OLE.with(|n| n.get() != 0)
}
thread_local! {static APARTMENT:RefCell<Option<(u32,u32)>>=const{RefCell::new(None)};}
pub(super) fn initialized() -> bool {
    APARTMENT.with(|state| state.borrow().is_some())
}

pub extern "win64" fn initialize(reserved: *mut c_void, flags: u32) -> i32 {
    if !reserved.is_null() || flags & !14 != 0 {
        return 0x80070057u32 as i32;
    }
    let model = flags & 2;
    APARTMENT.with(|state| {
        let mut state = state.borrow_mut();
        match state.as_mut() {
            None => {
                *state = Some((model, 1));
                0
            }
            Some((current, count)) if *current == model => {
                *count += 1;
                1
            }
            _ => 0x80010106u32 as i32,
        }
    })
}
pub extern "win64" fn uninitialize() {
    APARTMENT.with(|state| {
        let mut state = state.borrow_mut();
        if let Some((_, count)) = state.as_mut() {
            *count -= 1;
            if *count == 0 {
                *state = None;
            }
        }
    });
}
pub extern "win64" fn ole_initialize(reserved: *mut c_void) -> i32 {
    let hr = initialize(reserved, 2);
    if hr >= 0 {
        OLE.with(|n| n.set(n.get() + 1));
    }
    hr
}
pub unsafe extern "win64" fn create_instance(
    class: *const u8,
    outer: *mut c_void,
    _context: u32,
    iid: *const u8,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe {
        *out = std::ptr::null_mut();
    }
    if !initialized() {
        return 0x800401f0u32 as i32;
    }
    if !class.is_null()
        && unsafe { std::slice::from_raw_parts(class, 16) } == crate::api::audio::CLASS
    {
        if !outer.is_null() {
            return 0x80040110u32 as i32;
        }
        return unsafe { crate::api::audio::create(iid, out) };
    }
    0x80040154u32 as i32
}
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}
pub extern "win64" fn allocate(size: usize) -> *mut c_void {
    unsafe { malloc(size.max(1)) }
}
pub unsafe extern "win64" fn reallocate(p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { realloc(p, size) }
}
pub unsafe extern "win64" fn release(p: *mut c_void) {
    unsafe {
        free(p);
    }
}

pub extern "win64" fn ole_uninitialize() {
    OLE.with(|n| {
        if n.get() != 0 {
            n.set(n.get() - 1);
            uninitialize();
        }
    });
}

pub extern "win64" fn initialize_security(
    _sec_desc: usize,
    _auth_svc: i32,
    _as_auth_svc: usize,
    _reserved1: usize,
    _authn_level: u32,
    _imp_level: u32,
    _auth_info: usize,
    _capabilities: u32,
    _reserved3: usize,
) -> i32 {
    0
}

pub extern "win64" fn set_proxy_blanket(
    proxy: usize,
    _authn_svc: u32,
    _authz_svc: u32,
    _server_princ_name: usize,
    _authn_level: u32,
    _imp_level: u32,
    _auth_info: usize,
    _capabilities: u32,
) -> i32 {
    if proxy == 0 {
        return 0x80070057u32 as i32;
    }
    0
}

