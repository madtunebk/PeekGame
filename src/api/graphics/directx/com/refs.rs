use super::{guids::*, method, native_of, objects::cache, wrap, Com, Kind};
use crate::api::graphics::directx::types::*;
use std::{ffi::c_void, sync::atomic::Ordering};
pub(super) unsafe extern "win64" fn query(
    object: *mut Com,
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
    let native = unsafe { native_of(object) };
    let current = unsafe { (*object).kind };
    let iid = unsafe { *iid };
    let kind = match iid {
        ID3D11DEVICE => Kind::Device,
        ID3D11DEVICECONTEXT => Kind::Context,
        IDXGISWAPCHAIN => Kind::SwapChain,
        ID3D11TEXTURE2D => Kind::Texture,
        ID3D11RENDERTARGETVIEW => Kind::Rtv,
        IUNKNOWN => current,
        ID3D11DEVICECHILD if matches!(current, Kind::Context | Kind::Texture | Kind::Rtv) => {
            current
        }
        ID3D11RESOURCE if current == Kind::Texture => current,
        ID3D11VIEW if current == Kind::Rtv => current,
        _ => return E_NOINTERFACE,
    };
    let query: unsafe extern "C" fn(*mut c_void, *const Guid, *mut *mut c_void) -> i32 =
        unsafe { method(native, 0) };
    let mut result = std::ptr::null_mut();
    let hr = unsafe { query(native, &iid, &mut result) };
    if hr < 0 {
        return hr;
    }
    if kind == current {
        // Query may return a native base-interface pointer. Transfer its reference
        // to our canonical Windows wrapper instead of exposing that host pointer.
        let release: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(result, 2) };
        unsafe {
            add_ref(object);
            release(result);
            output.write(object);
        }
    } else {
        unsafe {
            output.write(wrap(result, kind, (*object).surface.clone()));
        }
    }
    hr
}
pub(super) unsafe extern "win64" fn add_ref(object: *mut Com) -> u32 {
    let native = unsafe { native_of(object) };
    unsafe {
        (*object).refs.fetch_add(1, Ordering::Relaxed);
    }
    let call: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(native, 1) };
    unsafe { call(native) }
}
pub(super) unsafe extern "win64" fn release(object: *mut Com) -> u32 {
    let native = unsafe { native_of(object) };
    let call: unsafe extern "C" fn(*mut c_void) -> u32 = unsafe { method(native, 2) };
    let count = unsafe { call(native) };
    let mut cache = cache().lock().unwrap();
    if unsafe { (*object).refs.fetch_sub(1, Ordering::AcqRel) } == 1 {
        cache.remove(&(native as usize, unsafe { (*object).kind }));
        drop(cache);
        unsafe {
            drop(Box::from_raw(object));
        }
    }
    count
}
