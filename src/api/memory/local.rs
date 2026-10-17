use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{Mutex, OnceLock},
};
unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}
static ALLOCATIONS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
fn allocations() -> &'static Mutex<HashMap<usize, usize>> {
    ALLOCATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub extern "win64" fn allocate(flags: u32, size: usize) -> usize {
    if flags & !0x40 != 0 {
        crate::api::system::error::set(50);
        return 0;
    }
    let p = unsafe {
        if flags & 0x40 != 0 {
            calloc(1, size.max(1))
        } else {
            malloc(size.max(1))
        }
    };
    if p.is_null() {
        crate::api::system::error::set(8);
        return 0;
    }
    allocations().lock().unwrap().insert(p as usize, size);
    p as usize
}
pub extern "win64" fn reallocate(handle: usize, size: usize, flags: u32) -> usize {
    if handle == 0 {
        return allocate(flags & 0x40, size);
    }
    if flags & 0x80 != 0 {
        // LMEM_MODIFY
        return handle;
    }
    let mut map = allocations().lock().unwrap();
    let Some(&old_size) = map.get(&handle) else {
        crate::api::system::error::set(6);
        return 0;
    };
    let p = unsafe { realloc(handle as *mut c_void, size.max(1)) };
    if p.is_null() {
        crate::api::system::error::set(8);
        return 0;
    }
    if flags & 0x40 != 0 && size > old_size {
        unsafe {
            std::ptr::write_bytes((p as *mut u8).add(old_size), 0, size - old_size);
        }
    }
    map.remove(&handle);
    map.insert(p as usize, size);
    p as usize
}
pub extern "win64" fn release(handle: usize) -> usize {
    if handle == 0 {
        return 0;
    }
    if allocations().lock().unwrap().remove(&handle).is_none() {
        crate::api::system::error::set(6);
        return handle;
    }
    unsafe {
        free(handle as *mut c_void);
    }
    0
}
pub extern "win64" fn size(handle: usize) -> usize {
    match allocations().lock().unwrap().get(&handle) {
        Some(size) => *size,
        None => {
            crate::api::system::error::set(6);
            0
        }
    }
}
