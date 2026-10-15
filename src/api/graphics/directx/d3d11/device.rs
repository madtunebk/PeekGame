use super::super::{
    com::{method, native_of, surface_of, wrap, Com, Kind},
    types::*,
};
use std::ffi::c_void;
pub(crate) unsafe extern "win64" fn create_texture(
    object: *mut Com,
    desc: *const c_void,
    data: *const c_void,
    output: *mut *mut Com,
) -> i32 {
    if output.is_null() {
        return E_POINTER;
    }
    unsafe {
        output.write(std::ptr::null_mut());
    }
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(
        *mut c_void,
        *const c_void,
        *const c_void,
        *mut *mut c_void,
    ) -> i32 = unsafe { method(native, 5) };
    let mut texture = std::ptr::null_mut();
    let hr = unsafe { call(native, desc, data, &mut texture) };
    if hr >= 0 {
        unsafe {
            output.write(wrap(texture, Kind::Texture, surface_of(object)));
        }
    }
    hr
}
pub(crate) unsafe extern "win64" fn create_view(
    object: *mut Com,
    resource: *mut Com,
    desc: *const c_void,
    output: *mut *mut Com,
) -> i32 {
    if output.is_null() {
        return E_POINTER;
    }
    unsafe {
        output.write(std::ptr::null_mut());
    }
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *const c_void,
        *mut *mut c_void,
    ) -> i32 = unsafe { method(native, 9) };
    let mut view = std::ptr::null_mut();
    let hr = unsafe { call(native, native_of(resource), desc, &mut view) };
    if hr >= 0 {
        unsafe {
            output.write(wrap(view, Kind::Rtv, surface_of(object)));
        }
    }
    hr
}
