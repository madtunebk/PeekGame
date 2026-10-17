//! Process and private growable heaps backed by libc, with per-heap ownership.
use crate::api::system::error;
use std::{collections::HashMap, ffi::c_void, sync::{Mutex, LazyLock, atomic::{AtomicUsize, Ordering}}};
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
struct Heap { blocks: HashMap<usize, usize>, initial: usize }
static NEXT: AtomicUsize = AtomicUsize::new(0x50480000);
static HEAPS: LazyLock<Mutex<HashMap<usize, Heap>>> = LazyLock::new(|| {
    Mutex::new(HashMap::from([(process_heap(), Heap { blocks: HashMap::new(), initial: 0 })]))
});
fn process_heap() -> usize { &PROCESS_HEAP as *const u8 as usize }
pub extern "win64" fn create(options: u32, initial: usize, maximum: usize) -> usize {
    // Fixed-size arenas, executable allocations and allocation exceptions need
    // different backends. Do not silently substitute a growable ordinary heap.
    if options & !NO_SERIALIZE != 0 || maximum != 0 { error::set(50); return 0; }
    let Some(bytes) = initial.max(1).checked_add(4095).map(|n| n & !4095) else {
        error::set(8); return 0;
    };
    let reserve = unsafe { calloc(1, bytes) };
    if reserve.is_null() { error::set(8); return 0; }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    HEAPS.lock().unwrap().insert(handle, Heap { blocks: HashMap::new(), initial: reserve as usize });
    handle
}
pub extern "win64" fn destroy(heap: usize) -> i32 {
    if heap == process_heap() { error::set(87); return 0; }
    let Some(owned) = HEAPS.lock().unwrap().remove(&heap) else { error::set(6); return 0; };
    for p in owned.blocks.keys() { unsafe { free(*p as *mut c_void) }; }
    unsafe { free(owned.initial as *mut c_void) };
    1
}
/// No Windows LFH or termination policy is claimed for the libc backend.
pub extern "win64" fn set_information(heap: usize, _class: u32, _info: *const c_void, _length: usize) -> i32 {
    if heap != 0 && !HEAPS.lock().unwrap().contains_key(&heap) { error::set(6); }
    else { error::set(50); }
    0
}
fn stop_flags(api: &str, flags: u32) -> ! {
    eprintln!("STOP: {api}: unsupported flags {flags:#x}");
    std::process::exit(78)
}

pub extern "win64" fn get_process_heap() -> usize {
    process_heap()
}
pub extern "win64" fn alloc(heap: usize, flags: u32, n: usize) -> *mut c_void {
    let mut heaps = HEAPS.lock().unwrap();
    let Some(owned) = heaps.get_mut(&heap) else { error::set(6); return std::ptr::null_mut(); };
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
    owned.blocks.insert(p as usize, n);
    p
}
pub extern "win64" fn release(heap: usize, flags: u32, p: *mut c_void) -> i32 {
    let mut heaps = HEAPS.lock().unwrap();
    let Some(owned) = heaps.get_mut(&heap) else { error::set(6); return 0; };
    if flags & !NO_SERIALIZE != 0 {
        stop_flags("HeapFree", flags);
    }
    if p.is_null() {
        return 1;
    }
    let known = owned.blocks.remove(&(p as usize)).is_some();
    if !known {
        error::set(87);
        return 0;
    }
    unsafe { free(p) };
    1
}
pub extern "win64" fn size(heap: usize, flags: u32, p: *const c_void) -> usize {
    let heaps = HEAPS.lock().unwrap();
    let Some(owned) = heaps.get(&heap) else { error::set(6); return usize::MAX; };
    if flags & !NO_SERIALIZE != 0 {
        stop_flags("HeapSize", flags);
    }
    match owned.blocks.get(&(p as usize)).copied()
    {
        Some(n) => n,
        None => {
            error::set(87);
            usize::MAX
        }
    }
}
pub extern "win64" fn reallocate(heap: usize, flags: u32, p: *mut c_void, n: usize) -> *mut c_void {

    if flags & !(NO_SERIALIZE | GENERATE_EXCEPTIONS | ZERO_MEMORY | REALLOC_IN_PLACE_ONLY) != 0 {
        stop_flags("HeapReAlloc", flags);
    }
    let mut heaps = HEAPS.lock().unwrap();
    let Some(owned) = heaps.get_mut(&heap) else { error::set(6); return std::ptr::null_mut(); };
    let table = &mut owned.blocks;
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
/// reports standard compatibility (0); this backend does not implement LFH.
pub unsafe extern "win64" fn query_information(
    heap: usize,
    class: u32,
    out: *mut c_void,
    length: usize,
    returned: *mut usize,
) -> i32 {
    let heaps = HEAPS.lock().unwrap();
    if !heaps.contains_key(&heap) { error::set(6); return 0; }
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
    unsafe { out.cast::<u32>().write_unaligned(0) };
    1
}
