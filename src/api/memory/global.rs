//! GlobalAlloc/GlobalLock/GlobalUnlock/GlobalFree/GlobalSize (ported from peek's
//! dragdrop.inc). GMEM_FIXED handles are the data pointer; GMEM_MOVEABLE handles are
//! a separate token whose data GlobalLock returns.
use crate::api::system::error;
use std::sync::Mutex;
const GMEM_MOVEABLE: u32 = 0x2;
const GMEM_ZEROINIT: u32 = 0x40;
const ERROR_NOT_LOCKED: u32 = 158;
struct Block {
    handle: usize,
    data: Box<[u8]>,
    locks: u32,
}
static BLOCKS: Mutex<Vec<Block>> = Mutex::new(Vec::new());
static NEXT_HANDLE: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0x4710_0000);

pub extern "win64" fn alloc(flags: u32, bytes: usize) -> usize {
    if flags & !(GMEM_MOVEABLE | GMEM_ZEROINIT) != 0 {
        error::set(87);
        return 0;
    }
    // Always zeroed: GMEM_ZEROINIT is then free, and unzeroed memory is never relied on.
    let data = vec![0u8; bytes.max(1)].into_boxed_slice();
    let handle = if flags & GMEM_MOVEABLE != 0 {
        NEXT_HANDLE.fetch_add(16, std::sync::atomic::Ordering::Relaxed)
    } else {
        data.as_ptr() as usize
    };
    BLOCKS.lock().unwrap().push(Block {
        handle,
        data,
        locks: 0,
    });
    handle
}
pub extern "win64" fn lock(handle: usize) -> usize {
    let mut blocks = BLOCKS.lock().unwrap();
    match blocks.iter_mut().find(|b| b.handle == handle) {
        Some(b) => {
            b.locks += 1;
            b.data.as_ptr() as usize
        }
        None => {
            error::set(6);
            0
        }
    }
}
pub extern "win64" fn unlock(handle: usize) -> i32 {
    let mut blocks = BLOCKS.lock().unwrap();
    match blocks.iter_mut().find(|b| b.handle == handle) {
        Some(b) if b.locks > 0 => {
            b.locks -= 1;
            if b.locks == 0 {
                error::set(0);
            }
            (b.locks != 0) as i32
        }
        Some(_) => {
            error::set(ERROR_NOT_LOCKED);
            0
        }
        None => {
            error::set(6);
            0
        }
    }
}
pub extern "win64" fn free(handle: usize) -> usize {
    if handle == 0 {
        return 0;
    }
    let mut blocks = BLOCKS.lock().unwrap();
    match blocks.iter().position(|b| b.handle == handle) {
        Some(i) if blocks[i].locks == 0 => {
            blocks.swap_remove(i);
            0
        }
        Some(_) => {
            error::set(ERROR_NOT_LOCKED);
            handle
        }
        None => {
            error::set(6);
            handle
        }
    }
}
pub extern "win64" fn size(handle: usize) -> usize {
    match BLOCKS.lock().unwrap().iter().find(|b| b.handle == handle) {
        Some(b) => b.data.len(),
        None => {
            error::set(6);
            0
        }
    }
}
