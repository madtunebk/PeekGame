use super::types::*;
use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
pub type ErrorHandler = unsafe extern "C" fn(*mut Display, *mut c_void) -> c_int;
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
        compose: *mut c_void,
    ) -> c_int;
    pub fn XMoveResizeWindow(
        display: *mut Display,
        window: c_ulong,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub fn XRaiseWindow(display: *mut Display, window: c_ulong) -> c_int;
    pub fn XSetInputFocus(
        display: *mut Display,
        window: c_ulong,
        revert: c_int,
        time: c_ulong,
    ) -> c_int;
    pub fn XQueryPointer(
        display: *mut Display,
        window: c_ulong,
        root: *mut c_ulong,
        child: *mut c_ulong,
        root_x: *mut c_int,
        root_y: *mut c_int,
        x: *mut c_int,
        y: *mut c_int,
        mask: *mut c_uint,
    ) -> c_int;
    pub fn XWarpPointer(
        display: *mut Display,
        source: c_ulong,
        destination: c_ulong,
        source_x: c_int,
        source_y: c_int,
        source_width: c_uint,
        source_height: c_uint,
        x: c_int,
        y: c_int,
    ) -> c_int;
    pub fn XGrabPointer(
        display: *mut Display,
        window: c_ulong,
        owner_events: c_int,
        mask: c_uint,
        pointer_mode: c_int,
        keyboard_mode: c_int,
        confine_to: c_ulong,
        cursor: c_ulong,
        time: c_ulong,
    ) -> c_int;
    pub fn XUngrabPointer(display: *mut Display, time: c_ulong) -> c_int;
    pub fn XTranslateCoordinates(
        display: *mut Display,
        source: c_ulong,
        destination: c_ulong,
        x: c_int,
        y: c_int,
        out_x: *mut c_int,
        out_y: *mut c_int,
        child: *mut c_ulong,
    ) -> c_int;
    pub fn XCreateBitmapFromData(
        display: *mut Display,
        drawable: c_ulong,
        data: *const c_char,
        width: c_uint,
        height: c_uint,
    ) -> c_ulong;
    pub fn XCreatePixmapCursor(
        display: *mut Display,
        source: c_ulong,
        mask: c_ulong,
        foreground: *mut Color,
        background: *mut Color,
        x: c_uint,
        y: c_uint,
    ) -> c_ulong;
    pub fn XFreePixmap(display: *mut Display, pixmap: c_ulong) -> c_int;
    pub fn XSetErrorHandler(handler: Option<ErrorHandler>) -> Option<ErrorHandler>;
    pub fn XSendEvent(
        display: *mut Display,
        window: c_ulong,
        propagate: c_int,
        mask: c_long,
        event: *mut Event,
    ) -> c_int;
    pub fn XIconifyWindow(display: *mut Display, window: c_ulong, screen: c_int) -> c_int;
    pub fn XkbKeycodeToKeysym(
        display: *mut Display,
        keycode: u8,
        group: c_int,
        level: c_int,
    ) -> c_ulong;
    pub fn XSetWMNormalHints(display: *mut Display, window: c_ulong, hints: *mut SizeHints);
    pub fn XPeekEvent(display: *mut Display, event: *mut Event) -> c_int;
    pub fn XGetErrorText(
        display: *mut Display,
        code: c_int,
        buffer: *mut c_char,
        length: c_int,
    ) -> c_int;
}

#[link(name = "X11")]
unsafe extern "C" {
    pub fn XGetWindowProperty(
        display: *mut Display,
        window: c_ulong,
        property: c_ulong,
        offset: c_long,
        length: c_long,
        delete: c_int,
        requested: c_ulong,
        actual: *mut c_ulong,
        format: *mut c_int,
        count: *mut c_ulong,
        remaining: *mut c_ulong,
        data: *mut *mut u8,
    ) -> c_int;
    pub fn XDeleteProperty(display: *mut Display, window: c_ulong, property: c_ulong) -> c_int;
    pub fn XConvertSelection(
        display: *mut Display,
        selection: c_ulong,
        target: c_ulong,
        property: c_ulong,
        requestor: c_ulong,
        time: c_ulong,
    ) -> c_int;
    pub fn XFree(data: *mut c_void) -> c_int;
}

#[link(name = "X11")]
unsafe extern "C" {
    pub fn XGetWindowAttributes(
        display: *mut Display,
        window: c_ulong,
        out: *mut Attributes,
    ) -> c_int;
    pub fn XGetWMNormalHints(
        display: *mut Display,
        window: c_ulong,
        out: *mut SizeHints,
        supplied: *mut c_long,
    ) -> c_int;
}
