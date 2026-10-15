use super::super::{
    com::{method, native_of, Com},
    types::Mapped,
};
use std::ffi::c_void;
pub(crate) unsafe extern "win64" fn map(
    object: *mut Com,
    resource: *mut Com,
    subresource: u32,
    kind: u32,
    flags: u32,
    output: *mut Mapped,
) -> i32 {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void, u32, u32, u32, *mut Mapped) -> i32 =
        unsafe { method(native, 14) };
    unsafe {
        call(
            native,
            native_of(resource),
            subresource,
            kind,
            flags,
            output,
        )
    }
}
pub(crate) unsafe extern "win64" fn unmap(object: *mut Com, resource: *mut Com, subresource: u32) {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void, u32) = unsafe { method(native, 15) };
    unsafe {
        call(native, native_of(resource), subresource);
    }
}
pub(crate) unsafe extern "win64" fn set_targets(
    object: *mut Com,
    count: u32,
    views: *const *mut Com,
    depth: *mut Com,
) {
    if count > 8 || (count != 0 && views.is_null()) {
        eprintln!("PeekGame: invalid OMSetRenderTargets array");
        std::process::exit(1);
    }
    let mut native_views = Vec::new();
    for i in 0..count {
        native_views.push(unsafe { native_of(*views.add(i as usize)) });
    }
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, u32, *const *mut c_void, *mut c_void) =
        unsafe { method(native, 33) };
    unsafe {
        call(
            native,
            count,
            if count == 0 {
                std::ptr::null()
            } else {
                native_views.as_ptr()
            },
            native_of(depth),
        );
    }
}
pub(crate) unsafe extern "win64" fn copy(
    object: *mut Com,
    destination: *mut Com,
    source: *mut Com,
) {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
        unsafe { method(native, 47) };
    unsafe {
        call(native, native_of(destination), native_of(source));
    }
}
pub(crate) unsafe extern "win64" fn clear_view(
    object: *mut Com,
    view: *mut Com,
    color: *const f32,
) {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, *mut c_void, *const f32) =
        unsafe { method(native, 50) };
    unsafe {
        call(native, native_of(view), color);
    }
}
pub(crate) unsafe extern "win64" fn clear_state(object: *mut Com) {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void) = unsafe { method(native, 110) };
    unsafe {
        call(native);
    }
}
