use super::super::{
    com::{guids::*, method, native_of, surface_of, wrap, Com, Kind},
    types::*,
};
use std::ffi::c_void;
pub(crate) unsafe extern "win64" fn present(object: *mut Com, interval: u32, flags: u32) -> i32 {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, u32, u32) -> i32 = unsafe { method(native, 8) };
    unsafe { call(native, interval, flags) }
}
pub(crate) unsafe extern "win64" fn get_buffer(
    object: *mut Com,
    index: u32,
    iid: *const Guid,
    output: *mut *mut Com,
) -> i32 {
    if output.is_null() {
        return E_POINTER;
    }
    unsafe {
        output.write(std::ptr::null_mut());
    }
    if iid.is_null() {
        return E_POINTER;
    }
    if ![ID3D11TEXTURE2D, ID3D11RESOURCE, IUNKNOWN].contains(unsafe { &*iid }) {
        return E_NOINTERFACE;
    }
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, u32, *const Guid, *mut *mut c_void) -> i32 =
        unsafe { method(native, 9) };
    let mut texture = std::ptr::null_mut();
    let hr = unsafe { call(native, index, &ID3D11TEXTURE2D, &mut texture) };
    if hr >= 0 {
        unsafe {
            output.write(wrap(texture, Kind::Texture, surface_of(object)));
        }
    }
    hr
}
pub(crate) unsafe extern "win64" fn resize_buffers(
    object: *mut Com,
    count: u32,
    width: u32,
    height: u32,
    format: u32,
    flags: u32,
) -> i32 {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32, u32) -> i32 =
        unsafe { method(native, 13) };
    unsafe { call(native, count, width, height, format, flags) }
}
