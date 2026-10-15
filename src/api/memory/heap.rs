//! The process heap (GetProcessHeap/HeapAlloc/HeapFree/HeapReAlloc/HeapSize/
//! HeapQueryInformation), ported from peek's heap.inc. Blocks come from malloc; a
//! table from block address to requested size answers HeapSize and tells our blocks
//! from foreign pointers (HeapFree of a foreign pointer fails with
//! ERROR_INVALID_PARAMETER). Supported flags: HEAP_NO_SERIALIZE (the heap is always
//! serialized), HEAP_GENERATE_EXCEPTIONS, HEAP_ZERO_MEMORY and
//! HEAP_REALLOC_IN_PLACE_ONLY. Other heaps stop explicitly.
use crate::api::system::error;
use std::{collections::HashMap, ffi::c_void, sync::Mutex};
unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn calloc(count: usize, n: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn malloc_usable_size(p: *mut c_void) -> usize;
}
const NO_SERIALIZE: u32 = 0x1;
const GENERATE_EXCEPTIONS: u32 = 0x4;
const ZERO_MEMORY: u32 = 0x8;
const REALLOC_IN_PLACE_ONLY: u32 = 0x10;

static PROCESS_HEAP: u8 = 0;
static BLOCKS: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);

fn process_heap() -> usize {
    &PROCESS_HEAP as *const u8 as usize
}
fn check(heap: usize, api: &str) {
    if heap != process_heap() {
        eprintln!("STOP: {api} on heap {heap:#x}: only the process heap exists");
        std::process::exit(78);
    }
}
fn stop_flags(api: &str, flags: u32) -> ! {
    eprintln!("STOP: {api}: unsupported flags {flags:#x}");
    std::process::exit(78)
}

pub extern "win64" fn get_process_heap() -> usize {
    process_heap()
}
pub extern "win64" fn alloc(heap: usize, flags: u32, n: usize) -> *mut c_void {
    check(heap, "HeapAlloc");
    if flags & !(NO_SERIALIZE | GENERATE_EXCEPTIONS | ZERO_MEMORY) != 0 {
        stop_flags("HeapAlloc", flags);
    }
    let p = unsafe {
        if flags & ZERO_MEMORY != 0 {
            calloc(1, n.max(1))
        } else {
            malloc(n.max(1))
        }
    };
    if p.is_null() {
        error::set(8);
        return p;
    }
    BLOCKS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(p as usize, n);
    p
}
pub extern "win64" fn release(heap: usize, flags: u32, p: *mut c_void) -> i32 {
    check(heap, "HeapFree");
    if flags & !NO_SERIALIZE != 0 {
        stop_flags("HeapFree", flags);
    }
    if p.is_null() {
        return 1;
    }
    let known = BLOCKS
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|b| b.remove(&(p as usize)))
        .is_some();
    if !known {
        error::set(87);
        return 0;
    }
    unsafe { free(p) };
    1
}
pub extern "win64" fn size(heap: usize, flags: u32, p: *const c_void) -> usize {
    check(heap, "HeapSize");
    if flags & !NO_SERIALIZE != 0 {
        stop_flags("HeapSize", flags);
    }
    match BLOCKS
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|b| b.get(&(p as usize)).copied())
    {
        Some(n) => n,
        None => {
            error::set(87);
            usize::MAX
        }
    }
}
pub extern "win64" fn reallocate(heap: usize, flags: u32, p: *mut c_void, n: usize) -> *mut c_void {
    check(heap, "HeapReAlloc");
    if flags & !(NO_SERIALIZE | GENERATE_EXCEPTIONS | ZERO_MEMORY | REALLOC_IN_PLACE_ONLY) != 0 {
        stop_flags("HeapReAlloc", flags);
    }
    let mut blocks = BLOCKS.lock().unwrap();
    let Some(table) = blocks.as_mut() else {
        error::set(87);
        return std::ptr::null_mut();
    };
    let Some(&old) = table.get(&(p as usize)) else {
        error::set(87);
        return std::ptr::null_mut();
    };
    if flags & REALLOC_IN_PLACE_ONLY != 0 {
        // Keep the address or fail.
        if n > unsafe { malloc_usable_size(p) } {
            error::set(8);
            return std::ptr::null_mut();
        }
        if flags & ZERO_MEMORY != 0 && n > old {
            unsafe { std::ptr::write_bytes(p.cast::<u8>().add(old), 0, n - old) };
        }
        table.insert(p as usize, n);
        return p;
    }
    let q = unsafe { realloc(p, n.max(1)) };
    if q.is_null() {
        error::set(8);
        if flags & GENERATE_EXCEPTIONS != 0 {
            eprintln!("STOP: HeapReAlloc with HEAP_GENERATE_EXCEPTIONS failed (SEH unavailable)");
            std::process::exit(78);
        }
        return q; // the old block stays valid
    }
    if flags & ZERO_MEMORY != 0 && n > old {
        unsafe { std::ptr::write_bytes(q.cast::<u8>().add(old), 0, n - old) };
    }
    table.remove(&(p as usize));
    table.insert(q as usize, n);
    q
}
/// HeapQueryInformation: only HeapCompatibilityInformation (class 0), which
/// reports the low-fragmentation heap (2) as Windows does by default.
pub unsafe extern "win64" fn query_information(
    heap: usize,
    class: u32,
    out: *mut c_void,
    length: usize,
    returned: *mut usize,
) -> i32 {
    check(heap, "HeapQueryInformation");
    if class != 0 {
        error::set(87);
        return 0;
    }
    if !returned.is_null() {
        unsafe { *returned = 4 };
    }
    if out.is_null() || length < 4 {
        error::set(122);
        return 0;
    }
    unsafe { out.cast::<u32>().write_unaligned(2) };
    1
}
