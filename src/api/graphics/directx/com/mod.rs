pub(super) mod guids;
mod names;
mod objects;
mod refs;
mod tables;
pub use objects::{native_of, surface_of, wrap, Com, Kind};
use std::ffi::c_void;
/// Access a native SysV COM method. The caller supplies the exact native signature.
pub unsafe fn method<T: Copy>(object: *mut c_void, slot: usize) -> T {
    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<usize>());
    let table = unsafe { *object.cast::<*const usize>() };
    let pointer = unsafe { *table.add(slot) };
    unsafe { std::mem::transmute_copy(&pointer) }
}
