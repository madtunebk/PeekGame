use super::types::*;
use std::ffi::{c_char, c_int, c_long, c_ulong};
#[link(name = "X11")]
unsafe extern "C" {
    pub fn XInitThreads() -> c_int;
    pub fn XOpenDisplay(name: *const c_char) -> *mut Display;
    pub fn XCloseDisplay(display: *mut Display) -> c_int;
    pub fn XDefaultScreen(display: *mut Display) -> c_int;
    pub fn XDisplayWidth(display: *mut Display, screen: c_int) -> c_int;
    pub fn XDisplayHeight(display: *mut Display, screen: c_int) -> c_int;
    pub fn XRootWindow(display: *mut Display, screen: c_int) -> c_ulong;
    pub fn XCreateSimpleWindow(
        display: *mut Display,
        parent: c_ulong,
        x: c_int,
        y: c_int,
        width: u32,
        height: u32,
        border: u32,
        border_pixel: c_ulong,
        background: c_ulong,
    ) -> c_ulong;
    pub fn XSelectInput(display: *mut Display, window: c_ulong, mask: c_long) -> c_int;
    pub fn XStoreName(display: *mut Display, window: c_ulong, name: *const c_char) -> c_int;
    pub fn XInternAtom(
        display: *mut Display,
        name: *const c_char,
        only_if_exists: c_int,
    ) -> c_ulong;
    pub fn XSetWMProtocols(
        display: *mut Display,
        window: c_ulong,
        atoms: *mut c_ulong,
        count: c_int,
    ) -> c_int;
    pub fn XChangeProperty(
        display: *mut Display,
        window: c_ulong,
        property: c_ulong,
        kind: c_ulong,
        format: c_int,
        mode: c_int,
        data: *const u8,
        count: c_int,
    ) -> c_int;
    pub fn XCreateFontCursor(display: *mut Display, shape: u32) -> c_ulong;
    pub fn XDefineCursor(display: *mut Display, window: c_ulong, cursor: c_ulong) -> c_int;
    pub fn XMapWindow(display: *mut Display, window: c_ulong) -> c_int;
    pub fn XUnmapWindow(display: *mut Display, window: c_ulong) -> c_int;
    pub fn XDestroyWindow(display: *mut Display, window: c_ulong) -> c_int;
    pub fn XFlush(display: *mut Display) -> c_int;
    pub fn XPending(display: *mut Display) -> c_int;
    pub fn XNextEvent(display: *mut Display, event: *mut Event) -> c_int;
    pub fn XLookupKeysym(event: *mut Key, index: c_int) -> c_ulong;
    pub fn XLookupString(
        event: *mut Key,
        buffer: *mut c_char,
        length: c_int,
        symbol: *mut c_ulong,
        compose: *mut std::ffi::c_void,
    ) -> c_int;
}
