pub(crate) mod classes;
pub(crate) mod create;
pub(crate) mod cursor;
pub(crate) mod lifecycle;
pub(crate) mod messages;
mod pump;
mod state;
mod strings;
mod types;
#[repr(C)]
pub struct NativeWindow {
    pub display: *mut std::ffi::c_void,
    pub window: u64,
}
/// Linux host ABI, for the future Vulkan surface bridge. Not a Windows export.
/// # Safety
/// `output` must point to writable NativeWindow storage. Call on the window owner thread.
#[no_mangle]
pub unsafe extern "C" fn peekgame_x11_window(hwnd: usize, output: *mut NativeWindow) -> u32 {
    if output.is_null() {
        return 87;
    }
    state::STATE.with(|state| {
        let state = state.borrow();
        let Some(window) = state.windows.get(&hwnd) else {
            return types::ERROR_WINDOW;
        };
        unsafe {
            output.write(NativeWindow {
                display: state.connection.as_ref().unwrap().display,
                window: window.native,
            });
        }
        0
    })
}

pub(crate) fn surface(
    hwnd: usize,
) -> Option<std::sync::Arc<crate::host::graphics::surface::Surface>> {
    state::STATE.with(|s| s.borrow().windows.get(&hwnd).map(|w| w.surface.clone()))
}
