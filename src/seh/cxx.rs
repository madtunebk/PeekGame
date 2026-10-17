//! VCRUNTIME C++ exception handling support.
use super::{dispatch::DispatcherContext, Context, ExceptionRecord};
use std::cell::Cell;
use std::ffi::c_void;

thread_local! {
    static CURRENT_EXCEPTION: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
    static CURRENT_CONTEXT: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
}

pub extern "win64" fn current_exception() -> *mut *mut c_void {
    CURRENT_EXCEPTION.with(|c| c.as_ptr())
}

pub extern "win64" fn current_exception_context() -> *mut *mut c_void {
    CURRENT_CONTEXT.with(|c| c.as_ptr())
}

pub unsafe extern "win64" fn cxx_throw_exception(object: usize, throw_info: usize) {
    let magic = 0x19930520u64;
    let image_base = crate::images::by_address(throw_info)
        .map(|m| m.base as u64)
        .unwrap_or(0);
    let args = [magic, object as u64, throw_info as u64, image_base];
    unsafe { crate::seh::raise_exception_entry(0xE06D_7363, 1, 4, args.as_ptr()) };
}

pub unsafe extern "win64" fn cxx_frame_handler_3(
    _record: *mut ExceptionRecord,
    _frame: u64,
    _context: *mut Context,
    _d: *mut DispatcherContext,
) -> i32 {
    1 // ExceptionContinueSearch
}
