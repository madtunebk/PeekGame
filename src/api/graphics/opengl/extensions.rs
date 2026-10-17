use super::{
    contexts::{current, CONTEXTS},
    fail, pixel,
};
use crate::host::graphics::glx;
use std::ffi::{c_char, c_void, CStr};
fn extensions(dc: usize) -> *const c_char {
    let Some(f) = pixel::format(dc) else {
        return std::ptr::null();
    };
    let host = unsafe { glx::peekgame_glx_extensions(f.display as *mut c_void) };
    if host.is_null() {
        return std::ptr::null();
    }
    let host = unsafe { CStr::from_ptr(host) }.to_bytes();
    let has = |name: &[u8]| host.split(|&c| c == b' ').any(|e| e == name);
    // Only advertise context attributes when the GLX provider supports them.
    let create = has(b"GLX_ARB_create_context") && has(b"GLX_ARB_create_context_profile");
    let swap = has(b"GLX_EXT_swap_control") || has(b"GLX_MESA_swap_control");
    match (create,swap){
 (true,true)=>c"WGL_ARB_extensions_string WGL_EXT_extensions_string WGL_ARB_create_context WGL_ARB_create_context_profile WGL_EXT_swap_control".as_ptr(),
 (true,false)=>c"WGL_ARB_extensions_string WGL_EXT_extensions_string WGL_ARB_create_context WGL_ARB_create_context_profile".as_ptr(),
 (false,true)=>c"WGL_ARB_extensions_string WGL_EXT_extensions_string WGL_EXT_swap_control".as_ptr(),
 _=>c"WGL_ARB_extensions_string WGL_EXT_extensions_string".as_ptr()}
}
pub(crate) extern "win64" fn extensions_arb(dc: usize) -> *const c_char {
    extensions(dc)
}
pub(crate) extern "win64" fn extensions_ext() -> *const c_char {
    extensions(current().1)
}
pub(crate) unsafe extern "win64" fn swap_interval(interval: i32) -> i32 {
    let (handle, dc) = current();
    let Some(f) = pixel::format(dc) else {
        return fail(6, 0);
    };
    if interval < 0 {
        return fail(87, 0);
    }
    if unsafe { glx::peekgame_glx_interval(f.display as *mut c_void, f.window, interval) } == 0 {
        return fail(50, 0);
    }
    if let Some(c) = CONTEXTS.lock().unwrap().get_mut(&handle) {
        c.interval = interval;
    }
    1
}
pub(crate) extern "win64" fn get_swap_interval() -> i32 {
    CONTEXTS
        .lock()
        .unwrap()
        .get(&current().0)
        .map_or(0, |c| c.interval)
}
pub(crate) unsafe extern "win64" fn get_proc_address(name: *const c_char) -> *const c_void {
    if name.is_null() {
        return fail(87, std::ptr::null());
    }
    let name_bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
    if current().0 != 0
        && !name_bytes.starts_with(b"wgl")
        && unsafe { glx::peekgame_gl_lookup(name) }.is_null()
    {
        super::trace(format_args!(
            "unavailable procedure {}",
            String::from_utf8_lossy(name_bytes)
        ));
    }
    match name_bytes {
        b"wglGetExtensionsStringARB" => extensions_arb as *const c_void,
        b"wglGetExtensionsStringEXT" => extensions_ext as *const c_void,
        b"wglCreateContextAttribsARB" => super::create_context_attribs as *const c_void,
        b"wglSwapIntervalEXT" => swap_interval as *const c_void,
        b"wglGetSwapIntervalEXT" => get_swap_interval as *const c_void,
        _ if current().0 != 0 => unsafe { glx::peekgame_gl_lookup(name) },
        _ => std::ptr::null(),
    }
}
