//! Host GLX: native handles are never exposed as Windows HGLRCs.
use std::ffi::{c_char, c_void};
#[link(name = "GL")]
unsafe extern "C" {
    pub fn glXMakeCurrent(display: *mut c_void, drawable: u64, context: *mut c_void) -> i32;
    pub fn glXDestroyContext(display: *mut c_void, context: *mut c_void);
    pub fn glXSwapBuffers(display: *mut c_void, drawable: u64);
    pub fn peekgame_glx_format(
        display: *mut c_void,
        window: u64,
        depth: i32,
        stencil: i32,
        alpha: i32,
        double: i32,
        actual: *mut i32,
    ) -> *mut c_void;
    pub fn peekgame_glx_create(
        display: *mut c_void,
        config: *mut c_void,
        share: *mut c_void,
        attributes: *const i32,
    ) -> *mut c_void;
    pub fn peekgame_glx_extensions(display: *mut c_void) -> *const c_char;
    pub fn peekgame_glx_interval(display: *mut c_void, window: u64, interval: i32) -> i32;
    pub fn peekgame_gl_lookup(name: *const c_char) -> *const c_void;
}
