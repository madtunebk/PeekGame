pub(crate) mod classes;
pub(crate) mod display;
pub(crate) mod input;
pub(crate) mod misc;
mod pump;
pub(crate) mod queue;
mod state;
mod strings;
pub(crate) mod types;
pub(crate) mod windows;
#[repr(C)]
pub struct NativeWindow {
    pub display: *mut std::ffi::c_void,
    pub window: u64,
}
/// Linux host ABI, for the Vulkan surface bridge. Not a Windows export.
/// # Safety
/// `output` must point to writable NativeWindow storage.
#[no_mangle]
pub unsafe extern "C" fn peekgame_x11_window(hwnd: usize, output: *mut NativeWindow) -> u32 {
    if output.is_null() {
        return 87;
    }
    let native = state::windows()
        .get(&hwnd)
        .map(|w| w.native)
        .filter(|&n| n != 0);
    match (native, state::connection()) {
        (Some(window), Ok(c)) => {
            unsafe {
                output.write(NativeWindow {
                    display: c.display,
                    window,
                })
            };
            0
        }
        _ => types::ERROR_WINDOW,
    }
}
pub(crate) fn surface(
    hwnd: usize,
) -> Option<std::sync::Arc<crate::host::graphics::surface::Surface>> {
    state::windows().get(&hwnd).and_then(|w| w.surface.clone())
}
