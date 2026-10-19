use std::{cell::RefCell, ffi::c_void, sync::{atomic::{AtomicU32, Ordering}, OnceLock}};
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

// The process-wide OLE task allocator shares storage with CoTaskMemAlloc/Free.
// IMalloc is a COM interface, so the game may use it before CoInitializeEx.
#[repr(C)]
struct TaskAllocator {
    vtable: *const usize,
    refs: AtomicU32,
}
unsafe impl Sync for TaskAllocator {}
unsafe impl Send for TaskAllocator {}

unsafe extern "win64" fn malloc_query_interface(
    this: *mut TaskAllocator,
    iid: *const u8,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32; // E_POINTER
    }
    unsafe { *out = std::ptr::null_mut() };
    if iid.is_null() {
        return 0x80004003u32 as i32;
    }
    let requested = unsafe { std::slice::from_raw_parts(iid, 16) };
    const IUNKNOWN: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];
    const IMALLOC: [u8; 16] = [2, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];
    if requested != IUNKNOWN && requested != IMALLOC {
        return 0x80004002u32 as i32; // E_NOINTERFACE
    }
    unsafe { *out = this.cast() };
    unsafe { malloc_add_ref(this) };
    0
}

unsafe extern "win64" fn malloc_add_ref(this: *mut TaskAllocator) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

unsafe extern "win64" fn malloc_release(this: *mut TaskAllocator) -> u32 {
    let refs = unsafe { &(*this).refs };
    let mut current = refs.load(Ordering::Relaxed);
    loop {
        if current <= 1 { return 1; } // process-owned singleton
        match refs.compare_exchange_weak(current, current - 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return current - 1,
            Err(actual) => current = actual,
        }
    }
}

unsafe extern "win64" fn malloc_alloc(_this: *mut TaskAllocator, size: usize) -> *mut c_void {
    allocate(size)
}
unsafe extern "win64" fn malloc_realloc(_this: *mut TaskAllocator, p: *mut c_void, size: usize) -> *mut c_void {
    unsafe { reallocate(p, size) }
}
unsafe extern "win64" fn malloc_free(_this: *mut TaskAllocator, p: *mut c_void) {
    unsafe { release(p) }
}
unsafe extern "C" { fn malloc_usable_size(p: *mut c_void) -> usize; }
unsafe extern "win64" fn malloc_get_size(_this: *mut TaskAllocator, p: *mut c_void) -> usize {
    if p.is_null() { usize::MAX } else { unsafe { malloc_usable_size(p) } }
}
unsafe extern "win64" fn malloc_did_alloc(_this: *mut TaskAllocator, _p: *mut c_void) -> i32 {
    -1 // cannot determine ownership; Windows permits this answer
}
unsafe extern "win64" fn malloc_heap_minimize(_this: *mut TaskAllocator) {}

fn task_allocator() -> &'static TaskAllocator {
    static VTABLE: OnceLock<[usize; 9]> = OnceLock::new();
    static OBJECT: OnceLock<TaskAllocator> = OnceLock::new();
    let vtable = VTABLE.get_or_init(|| [
        malloc_query_interface as *const () as usize,
        malloc_add_ref as *const () as usize,
        malloc_release as *const () as usize,
        malloc_alloc as *const () as usize,
        malloc_realloc as *const () as usize,
        malloc_free as *const () as usize,
        malloc_get_size as *const () as usize,
        malloc_did_alloc as *const () as usize,
        malloc_heap_minimize as *const () as usize,
    ]);
    OBJECT.get_or_init(|| TaskAllocator { vtable: vtable.as_ptr(), refs: AtomicU32::new(1) })
}

pub unsafe extern "win64" fn get_malloc(context: u32, out: *mut *mut c_void) -> i32 {
    if out.is_null() { return 0x80004003u32 as i32; }
    unsafe { *out = std::ptr::null_mut() };
    if context != 1 { return 0x80070057u32 as i32; } // MEMCTX_TASK
    let object = task_allocator() as *const TaskAllocator as *mut TaskAllocator;
    unsafe { malloc_add_ref(object) };
    unsafe { *out = object.cast() };
    0
}

#[cfg(test)]
mod malloc_tests {
    use super::*;

    #[test]
    fn task_allocator_uses_cotask_memory_and_com_identity() {
        unsafe {
            let mut object = std::ptr::null_mut();
            assert_eq!(get_malloc(1, &mut object), 0);
            assert!(!object.is_null());
            let vtable = *(object as *mut *const usize);
            let query: unsafe extern "win64" fn(*mut c_void, *const u8, *mut *mut c_void) -> i32 =
                std::mem::transmute(*vtable);
            let alloc: unsafe extern "win64" fn(*mut c_void, usize) -> *mut c_void =
                std::mem::transmute(*vtable.add(3));
            let free: unsafe extern "win64" fn(*mut c_void, *mut c_void) =
                std::mem::transmute(*vtable.add(5));
            let get_size: unsafe extern "win64" fn(*mut c_void, *mut c_void) -> usize =
                std::mem::transmute(*vtable.add(6));
            let release_ref: unsafe extern "win64" fn(*mut c_void) -> u32 =
                std::mem::transmute(*vtable.add(2));
            let iid = [2, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];
            let mut queried = std::ptr::null_mut();
            assert_eq!(query(object, iid.as_ptr(), &mut queried), 0);
            assert_eq!(queried, object);
            let p = alloc(object, 48);
            assert!(!p.is_null());
            assert!(get_size(object, p) >= 48);
            free(object, p);
            let p = allocate(16);
            assert!(!p.is_null());
            free(object, p);
            release_ref(queried);
            release_ref(object);
            assert_eq!(get_malloc(0, &mut object), 0x80070057u32 as i32);
            assert!(object.is_null());
        }
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

fn parse_guid(s: &str) -> Option<[u8; 16]> {
    let clean = s.trim().trim_start_matches('{').trim_end_matches('}');
    let parts: Vec<&str> = clean.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    if parts[0].len() != 8 || parts[1].len() != 4 || parts[2].len() != 4 || parts[3].len() != 4 || parts[4].len() != 12 {
        return None;
    }
    let d1 = u32::from_str_radix(parts[0], 16).ok()?;
    let d2 = u16::from_str_radix(parts[1], 16).ok()?;
    let d3 = u16::from_str_radix(parts[2], 16).ok()?;
    let d4_0 = u8::from_str_radix(&parts[3][0..2], 16).ok()?;
    let d4_1 = u8::from_str_radix(&parts[3][2..4], 16).ok()?;
    let mut d4_rest = [0u8; 6];
    for i in 0..6 {
        d4_rest[i] = u8::from_str_radix(&parts[4][i*2..i*2+2], 16).ok()?;
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&d1.to_le_bytes());
    out[4..6].copy_from_slice(&d2.to_le_bytes());
    out[6..8].copy_from_slice(&d3.to_le_bytes());
    out[8] = d4_0;
    out[9] = d4_1;
    out[10..16].copy_from_slice(&d4_rest);
    Some(out)
}

pub unsafe extern "win64" fn iid_from_string(lpsz: *const u16, lpiid: *mut u8) -> i32 {
    if lpiid.is_null() {
        return 0x80070057u32 as i32; // E_INVALIDARG
    }
    if lpsz.is_null() {
        unsafe { lpiid.write_bytes(0, 16) };
        return 0; // S_OK
    }
    let s = unsafe { crate::api::text::string(lpsz) }.unwrap_or_default();
    match parse_guid(&s) {
        Some(guid) => {
            unsafe { std::ptr::copy_nonoverlapping(guid.as_ptr(), lpiid, 16) };
            0
        }
        None => 0x800401E0u32 as i32, // CO_E_IIDSTRING
    }
}

pub unsafe extern "win64" fn clsid_from_string(lpsz: *const u16, lpclsid: *mut u8) -> i32 {
    unsafe { iid_from_string(lpsz, lpclsid) }
}

pub unsafe extern "win64" fn string_from_iid(rclsid: *const u8, lplpsz: *mut *mut u16) -> i32 {
    if rclsid.is_null() || lplpsz.is_null() {
        return 0x80070057u32 as i32;
    }
    let d1 = unsafe { (rclsid as *const u32).read_unaligned() };
    let d2 = unsafe { (rclsid.add(4) as *const u16).read_unaligned() };
    let d3 = unsafe { (rclsid.add(6) as *const u16).read_unaligned() };
    let d4 = unsafe { std::slice::from_raw_parts(rclsid.add(8), 8) };
    let s = format!("{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        d1, d2, d3, d4[0], d4[1], d4[2], d4[3], d4[4], d4[5], d4[6], d4[7]);
    let utf16: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    let mem = allocate(utf16.len() * 2) as *mut u16;
    if mem.is_null() {
        return 0x8007000Eu32 as i32;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), mem, utf16.len());
        *lplpsz = mem;
    }
    0
}
