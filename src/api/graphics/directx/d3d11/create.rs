use super::super::{
    com::{wrap, Com, Kind},
    types::*,
};
use crate::{api::window, host::graphics::dxvk};
use std::ffi::c_void;
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe extern "win64" fn create_device_and_swap_chain(
    adapter: *mut c_void,
    driver: u32,
    software: *mut c_void,
    flags: u32,
    levels: *const u32,
    level_count: u32,
    sdk: u32,
    description: *const SwapDesc,
    swap: *mut *mut Com,
    device: *mut *mut Com,
    feature_level: *mut u32,
    context: *mut *mut Com,
) -> i32 {
    for output in [swap, device, context] {
        if !output.is_null() {
            unsafe {
                output.write(std::ptr::null_mut());
            }
        }
    }
    if !adapter.is_null() || !software.is_null() {
        return E_INVALIDARG;
    }
    let mut surface = None;
    let mut native_desc = None;
    if !description.is_null() {
        let mut copy = unsafe { *description };
        if copy.windowed == 0 {
            return DXGI_UNSUPPORTED;
        }
        let Some(window) = window::surface(copy.window) else {
            return E_INVALIDARG;
        };
        copy.window = &window.raw as *const _ as usize;
        native_desc = Some(copy);
        surface = Some(window);
    } else if !swap.is_null() {
        return E_INVALIDARG;
    }
    let create = match dxvk::create() {
        Ok(create) => create,
        Err(hr) => return hr,
    };
    let (mut s, mut d, mut c) = (
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    let hr = unsafe {
        create(
            std::ptr::null_mut(),
            driver,
            std::ptr::null_mut(),
            flags,
            levels,
            level_count,
            sdk,
            native_desc.as_ref().map_or(std::ptr::null(), |d| d),
            if swap.is_null() {
                std::ptr::null_mut()
            } else {
                &mut s
            },
            if device.is_null() {
                std::ptr::null_mut()
            } else {
                &mut d
            },
            feature_level,
            if context.is_null() {
                std::ptr::null_mut()
            } else {
                &mut c
            },
        )
    };
    if hr < 0 {
        return hr;
    }
    unsafe {
        if !swap.is_null() {
            swap.write(wrap(s, Kind::SwapChain, surface.clone()));
        }
        if !device.is_null() {
            device.write(wrap(d, Kind::Device, surface.clone()));
        }
        if !context.is_null() {
            context.write(wrap(c, Kind::Context, surface));
        }
    }
    hr
}
