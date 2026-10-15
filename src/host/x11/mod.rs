mod events;
mod ffi;
mod keyboard;
mod types;
pub(crate) use events::NativeEvent;
use std::{ffi::CString, sync::OnceLock};
/// Size of the default X screen, from a process-wide connection kept open for
/// display queries (monitors, metrics). None without a display.
pub(crate) fn screen_size() -> Option<(i32, i32)> {
    struct Shared(*mut types::Display);
    unsafe impl Send for Shared {}
    unsafe impl Sync for Shared {}
    static SHARED: OnceLock<Option<Shared>> = OnceLock::new();
    let shared = SHARED.get_or_init(|| {
        Connection::open().ok().map(|c| {
            let display = c.display;
            std::mem::forget(c);
            Shared(display)
        })
    });
    let display = shared.as_ref()?.0;
    unsafe {
        let screen = ffi::XDefaultScreen(display);
        Some((
            ffi::XDisplayWidth(display, screen),
            ffi::XDisplayHeight(display, screen),
        ))
    }
}
pub(crate) struct Connection {
    pub display: *mut types::Display,
    protocols: u64,
    delete: u64,
}
impl Connection {
    pub(crate) fn open() -> Result<Self, u32> {
        static INITIALIZED: OnceLock<bool> = OnceLock::new();
        if !*INITIALIZED.get_or_init(|| unsafe { ffi::XInitThreads() != 0 }) {
            return Err(50);
        }
        let display = unsafe { ffi::XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            return Err(50);
        }
        let protocols = unsafe { ffi::XInternAtom(display, c"WM_PROTOCOLS".as_ptr(), 0) };
        let delete = unsafe { ffi::XInternAtom(display, c"WM_DELETE_WINDOW".as_ptr(), 0) };
        Ok(Self {
            display,
            protocols,
            delete,
        })
    }
    pub(crate) fn arrow(&self) -> Result<u64, u32> {
        let cursor = unsafe {
            ffi::XCreateFontCursor(self.display, 68 /* XC_left_ptr */)
        };
        if cursor == 0 {
            Err(8)
        } else {
            Ok(cursor)
        }
    }
    pub(crate) fn create(
        &self,
        title: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        cursor: u64,
    ) -> Result<u64, u32> {
        let title = CString::new(title).map_err(|_| 87u32)?;
        unsafe {
            let screen = ffi::XDefaultScreen(self.display);
            let root = ffi::XRootWindow(self.display, screen);
            let window = ffi::XCreateSimpleWindow(self.display, root, x, y, width, height, 0, 0, 0);
            if window == 0 {
                return Err(8);
            }
            ffi::XSelectInput(self.display, window, (1 << 0) | (1 << 1) | (1 << 17));
            ffi::XStoreName(self.display, window, title.as_ptr());
            let name = ffi::XInternAtom(self.display, c"_NET_WM_NAME".as_ptr(), 0);
            let utf8 = ffi::XInternAtom(self.display, c"UTF8_STRING".as_ptr(), 0);
            ffi::XChangeProperty(
                self.display,
                window,
                name,
                utf8,
                8,
                0,
                title.as_bytes().as_ptr(),
                title.as_bytes().len() as i32,
            );
            let mut delete = self.delete;
            ffi::XSetWMProtocols(self.display, window, &mut delete, 1);
            if cursor != 0 {
                ffi::XDefineCursor(self.display, window, cursor);
            }
            ffi::XFlush(self.display);
            Ok(window)
        }
    }
    pub(crate) fn show(&self, window: u64, visible: bool) {
        unsafe {
            if visible {
                ffi::XMapWindow(self.display, window);
            } else {
                ffi::XUnmapWindow(self.display, window);
            }
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn destroy(&self, window: u64) {
        unsafe {
            ffi::XDestroyWindow(self.display, window);
            ffi::XFlush(self.display);
        }
    }
}
impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            ffi::XCloseDisplay(self.display);
        }
    }
}

// XInitThreads is completed before a Connection is constructed; Xlib serializes calls.
unsafe impl Send for Connection {}
unsafe impl Sync for Connection {}
