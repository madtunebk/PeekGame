//! Minimal Microsoft Concurrency Runtime ABI used by the queue base class.
use std::{ffi::c_void, ptr};

#[repr(C)]
struct ThreadQueue {
    lock: i32,
    head: *mut c_void,
    tail: *mut c_void,
    head_pos: usize,
    tail_pos: usize,
}

#[repr(C)]
struct QueueData {
    tail_pos: usize,
    head_pos: usize,
    queues: [ThreadQueue; 8],
}

#[repr(C)]
struct QueueBase {
    vtable: *const usize,
    data: *mut QueueData,
    alloc_count: usize,
    item_size: usize,
}

#[repr(C)]
struct QueueNode {
    next: *mut QueueNode,
    value: [u8; 0],
}

// The base constructor installs this only until the derived concurrent_queue
// constructor replaces it with its item-specific virtual table.
static BASE_VTABLE: [usize; 6] = [0; 6];

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "win64" fn concurrent_queue_base_v4_ctor(
    this: *mut c_void,
    item_size: usize,
) -> *mut c_void {
    if this.is_null() {
        return this;
    }
    let data = unsafe { calloc(1, std::mem::size_of::<QueueData>()) }.cast::<QueueData>();
    if data.is_null() {
        return ptr::null_mut();
    }
    let object = this.cast::<QueueBase>();
    unsafe {
        (*object).vtable = BASE_VTABLE.as_ptr();
        (*object).data = data;
        (*object).alloc_count = match item_size {
            0..=8 => 32,
            9..=16 => 16,
            17..=32 => 8,
            33..=64 => 4,
            65..=128 => 2,
            _ => 1,
        };
        (*object).item_size = item_size;
    }
    this
}

#[unsafe(no_mangle)]
pub unsafe extern "win64" fn concurrent_queue_base_v4_dtor(this: *mut c_void) {
    if this.is_null() {
        return;
    }
    let object = this.cast::<QueueBase>();
    let data = unsafe { (*object).data };
    if !data.is_null() {
        for lane in unsafe { &mut (*data).queues } {
            let mut node = lane.head.cast::<QueueNode>();
            while !node.is_null() {
                let next = unsafe { (*node).next };
                unsafe { free(node.cast()) };
                node = next;
            }
        }
        unsafe { free(data.cast()) };
        unsafe { (*object).data = ptr::null_mut() };
    }
}

/// Append an item to the base queue's first per-thread lane. The game uses
/// this private helper for small work queues; the queue state is allocated by
/// our matching base constructor above.
#[unsafe(no_mangle)]
pub unsafe extern "win64" fn concurrent_queue_base_v4_internal_push(
    this: *mut c_void,
    item: *const c_void,
) {
    if this.is_null() || item.is_null() {
        return;
    }
    let object = this.cast::<QueueBase>();
    let (data, item_size) = unsafe { ((*object).data, (*object).item_size) };
    if data.is_null() || item_size == 0 {
        return;
    }
    let Some(allocation_size) = std::mem::size_of::<QueueNode>().checked_add(item_size) else {
        return;
    };
    let node = unsafe { calloc(1, allocation_size) }.cast::<QueueNode>();
    if node.is_null() {
        return;
    }
    unsafe {
        ptr::copy_nonoverlapping(
            item.cast::<u8>(),
            node.cast::<u8>().add(std::mem::size_of::<QueueNode>()),
            item_size,
        );
    }
    let lane = unsafe { &mut (*data).queues[0] };
    let lock = unsafe { std::sync::atomic::AtomicI32::from_ptr(&mut lane.lock) };
    while lock
        .compare_exchange(0, 1, std::sync::atomic::Ordering::Acquire, std::sync::atomic::Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop();
    }
    unsafe {
        if lane.tail.is_null() {
            lane.head = node.cast();
        } else {
            (*(lane.tail.cast::<QueueNode>())).next = node;
        }
        lane.tail = node.cast();
        lane.tail_pos = lane.tail_pos.wrapping_add(1);
        (*data).tail_pos = (*data).tail_pos.wrapping_add(1);
    }
    lock.store(0, std::sync::atomic::Ordering::Release);
}

/// Remove the oldest item from the first lane. The MSVC `bool` return is one
/// byte; `out` receives the item bytes only when an entry was available.
#[unsafe(no_mangle)]
pub unsafe extern "win64" fn concurrent_queue_base_v4_internal_pop_if_present(
    this: *mut c_void,
    out: *mut c_void,
) -> u8 {
    if this.is_null() || out.is_null() {
        return 0;
    }
    let object = this.cast::<QueueBase>();
    let (data, item_size) = unsafe { ((*object).data, (*object).item_size) };
    if data.is_null() || item_size == 0 {
        return 0;
    }
    let lane = unsafe { &mut (*data).queues[0] };
    let lock = unsafe { std::sync::atomic::AtomicI32::from_ptr(&mut lane.lock) };
    while lock
        .compare_exchange(0, 1, std::sync::atomic::Ordering::Acquire, std::sync::atomic::Ordering::Relaxed)
        .is_err()
    {
        std::hint::spin_loop();
    }
    let node = lane.head.cast::<QueueNode>();
    if node.is_null() {
        lock.store(0, std::sync::atomic::Ordering::Release);
        return 0;
    }
    unsafe {
        lane.head = (*node).next.cast();
        if lane.head.is_null() {
            lane.tail = ptr::null_mut();
        }
        lane.head_pos = lane.head_pos.wrapping_add(1);
        (*data).head_pos = (*data).head_pos.wrapping_add(1);
        ptr::copy_nonoverlapping(
            node.cast::<u8>().add(std::mem::size_of::<QueueNode>()),
            out.cast::<u8>(),
            item_size,
        );
        free(node.cast());
    }
    lock.store(0, std::sync::atomic::Ordering::Release);
    1
}
