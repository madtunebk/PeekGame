mod events;
mod fullscreen;
pub(crate) mod modes;
pub(crate) mod xdnd;
mod ffi;
pub(crate) mod identity;
pub(crate) mod keyboard;
pub(crate) mod monitors;
mod types;
pub(crate) use events::NativeEvent;
use std::{
    ffi::{c_int, c_void, CString},
    sync::{Arc, OnceLock},
};
/// Size of the default X screen. None without a display.
pub(crate) fn monitors() -> Vec<monitors::Monitor> {
    Connection::shared()
        .map(|c| monitors::list(&c))
        .unwrap_or_default()
}
pub(crate) fn primary_size() -> Option<(i32, i32)> {
    monitors().first().map(|m| (m.width, m.height))
}
/// X errors (a window gone meanwhile, a focus request the server refuses) are the
/// normal result of racing the window manager: they are logged, never fatal.
unsafe extern "C" fn on_error(display: *mut types::Display, event: *mut c_void) -> c_int {
    let e = unsafe { &*(event as *const types::ErrorEvent) };
    let mut text = [0i8; 128];
    unsafe { ffi::XGetErrorText(display, e.error_code as c_int, text.as_mut_ptr(), 128) };
    let text = unsafe { std::ffi::CStr::from_ptr(text.as_ptr()) }.to_string_lossy();
    if std::env::var_os("PEEKGAME_X11_ERRORS").is_some() {
        eprintln!(
            "X11: {text} (request {}, resource {:#x})",
            e.request_code, e.resource
        );
    }
    0
}
pub(crate) struct Connection {
    pub display: *mut types::Display,
    protocols: u64,
    delete: u64,
    root: u64,
    screen: c_int,
    /// The XInput2 major opcode when the extension is present and raw motion is
    /// selected on the root; GenericEvents carry it in their `extension` field.
    /// None falls the raw-input path back to core-pointer deltas.
    xi_opcode: Option<c_int>,
}
const EVENT_MASK: i64 = 1 /* KeyPress */
    | 2 /* KeyRelease */
    | 4 /* ButtonPress */
    | 8 /* ButtonRelease */
    | 0x10 /* EnterWindow */
    | 0x20 /* LeaveWindow */
    | 0x40 /* PointerMotion */
    | 0x8000 /* Exposure */
    | 0x20000 /* StructureNotify */
    | 0x200000 /* FocusChange */;
impl Connection {
    /// The process-wide connection every window uses (Xlib serializes calls after
    /// XInitThreads). Fails with ERROR_NOT_SUPPORTED without a display.
    pub(crate) fn shared() -> Result<Arc<Connection>, u32> {
        static SHARED: OnceLock<Option<Arc<Connection>>> = OnceLock::new();
        SHARED
            .get_or_init(|| Connection::open().ok().map(Arc::new))
            .clone()
            .ok_or(50)
    }
    pub(crate) fn open() -> Result<Self, u32> {
        static INITIALIZED: OnceLock<bool> = OnceLock::new();
        if !*INITIALIZED.get_or_init(|| unsafe {
            let ok = ffi::XInitThreads() != 0;
            ffi::XSetErrorHandler(Some(on_error));
            ok
        }) {
            return Err(50);
        }
        let display = unsafe { ffi::XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            return Err(50);
        }
        unsafe {
            let screen = ffi::XDefaultScreen(display);
            let root = ffi::XRootWindow(display, screen);
            Ok(Self {
                display,
                protocols: ffi::XInternAtom(display, c"WM_PROTOCOLS".as_ptr(), 0),
                delete: ffi::XInternAtom(display, c"WM_DELETE_WINDOW".as_ptr(), 0),
                root,
                screen,
                xi_opcode: Self::init_raw_motion(display, root),
            })
        }
    }
    /// Opts the connection into XInput2 raw relative-motion events on the root,
    /// returning the extension's opcode. These report device motion before the X
    /// server's pointer acceleration and never stop at a screen edge, which is what
    /// a game reads through WM_INPUT for camera look. None when XInput2 2.1
    /// is unavailable, leaving the raw path on core-pointer deltas.
    fn init_raw_motion(display: *mut types::Display, root: u64) -> Option<c_int> {
        const XI_RAW_MOTION: usize = 17;
        const XI_ALL_MASTER_DEVICES: c_int = 1;
        // PEEKGAME_NO_XI2 forces the raw-input path back onto core-pointer deltas,
        // to compare mouse feel or to work around a server that mishandles XInput2.
        if std::env::var_os("PEEKGAME_NO_XI2").is_some() {
            return None;
        }
        unsafe {
            let (mut opcode, mut event, mut error) = (0, 0, 0);
            if ffi::XQueryExtension(
                display,
                c"XInputExtension".as_ptr(),
                &mut opcode,
                &mut event,
                &mut error,
            ) == 0
            {
                return None;
            }
            // XI 2.0 can stop delivering raw motion during a core pointer grab,
            // including the implicit grab while a mouse button is held. Camera
            // look must keep working while aiming/firing: XI 2.1 delivers raw
            // events regardless of grabs. Older servers use core deltas instead.
            let (mut major, mut minor) = (2, 1);
            if ffi::XIQueryVersion(display, &mut major, &mut minor) != 0
                || (major, minor) < (2, 1)
            {
                return None;
            }
            // A mask covering XI_RawMotion (bit 17 -> 3 bytes); request it from all
            // master devices so every physical mouse feeds the one raw stream.
            let mut mask = [0u8; 4];
            mask[XI_RAW_MOTION >> 3] |= 1 << (XI_RAW_MOTION & 7);
            let mut em = types::IEventMask {
                deviceid: XI_ALL_MASTER_DEVICES,
                mask_len: mask.len() as c_int,
                mask: mask.as_mut_ptr(),
            };
            ffi::XISelectEvents(display, root, &mut em, 1);
            ffi::XFlush(display);
            Some(opcode)
        }
    }
    fn atom(&self, name: &std::ffi::CStr) -> u64 {
        unsafe { ffi::XInternAtom(self.display, name.as_ptr(), 0) }
    }
    /// Whether unaccelerated XInput2 raw relative motion is available on this
    /// connection (the raw-input camera-look path uses it in place of core deltas).
    pub(crate) fn raw_motion_enabled(&self) -> bool {
        self.xi_opcode.is_some()
    }
    /// An X font cursor (cursorfont.h shape).
    pub(crate) fn font_cursor(&self, shape: u32) -> Result<u64, u32> {
        let cursor = unsafe { ffi::XCreateFontCursor(self.display, shape) };
        if cursor == 0 {
            Err(8)
        } else {
            Ok(cursor)
        }
    }
    /// A fully transparent cursor (ShowCursor(FALSE), SetCursor(NULL)).
    pub(crate) fn blank_cursor(&self) -> u64 {
        static BLANK: OnceLock<u64> = OnceLock::new();
        *BLANK.get_or_init(|| unsafe {
            let data = [0i8; 8];
            let pixmap = ffi::XCreateBitmapFromData(self.display, self.root, data.as_ptr(), 8, 8);
            let mut color = types::Color {
                pixel: 0,
                red: 0,
                green: 0,
                blue: 0,
                flags: 0,
                pad: 0,
            };
            let mut other = types::Color { ..color };
            let cursor = ffi::XCreatePixmapCursor(
                self.display,
                pixmap,
                pixmap,
                &mut color,
                &mut other,
                0,
                0,
            );
            ffi::XFreePixmap(self.display, pixmap);
            cursor
        })
    }
    /// A two-color cursor from X bitmaps (LSB-first bits, rows padded to bytes):
    /// `mask` bits are visible, `source` bits white and the rest black.
    pub(crate) fn bitmap_cursor(
        &self,
        width: u32,
        height: u32,
        source: &[u8],
        mask: &[u8],
        hot_x: u32,
        hot_y: u32,
    ) -> Result<u64, u32> {
        unsafe {
            let source = ffi::XCreateBitmapFromData(self.display, self.root, source.as_ptr().cast(), width, height);
            let mask = ffi::XCreateBitmapFromData(self.display, self.root, mask.as_ptr().cast(), width, height);
            let mut white = types::Color { pixel: 0, red: 0xffff, green: 0xffff, blue: 0xffff, flags: 7, pad: 0 };
            let mut black = types::Color { red: 0, green: 0, blue: 0, ..white };
            let cursor = if source != 0 && mask != 0 {
                ffi::XCreatePixmapCursor(self.display, source, mask, &mut white, &mut black, hot_x, hot_y)
            } else {
                0
            };
            for pixmap in [source, mask] {
                if pixmap != 0 {
                    ffi::XFreePixmap(self.display, pixmap);
                }
            }
            if cursor == 0 {
                Err(8)
            } else {
                Ok(cursor)
            }
        }
    }
    /// A window: top-level under the root, or a child of `parent`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        &self,
        parent: u64,
        title: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        cursor: u64,
    ) -> Result<u64, u32> {
        let (x, y) = if parent == 0 {
            let (ox, oy) = monitors::origin(self);
            (x + ox, y + oy)
        } else {
            (x, y)
        };
        unsafe {
            let window = ffi::XCreateSimpleWindow(
                self.display,
                if parent == 0 { self.root } else { parent },
                x,
                y,
                width.max(1),
                height.max(1),
                0,
                0,
                0,
            );
            if window == 0 {
                return Err(8);
            }
            ffi::XSelectInput(self.display, window, EVENT_MASK);
            if parent==0{let mut hints=types::SizeHints{flags:1<<2,x,y,..Default::default()};ffi::XSetWMNormalHints(self.display,window,&mut hints);}

            if parent == 0 {
                if let Some(class) = identity::property() {
                    ffi::XChangeProperty(
                        self.display,
                        window,
                        self.atom(c"WM_CLASS"),
                        31,
                        8,
                        0,
                        class.as_ptr(),
                        class.len() as i32,
                    );
                }
            }

            if parent == 0 {
                let mut delete = self.delete;
                ffi::XSetWMProtocols(self.display, window, &mut delete, 1);
            }
            self.set_title(window, title);
            if cursor != 0 {
                ffi::XDefineCursor(self.display, window, cursor);
            }
            ffi::XFlush(self.display);
            Ok(window)
        }
    }
    pub(crate) fn set_title(&self, window: u64, title: &str) {
        let title = CString::new(title.replace('\0', "")).unwrap_or_default();
        unsafe {
            ffi::XStoreName(self.display, window, title.as_ptr());
            ffi::XChangeProperty(
                self.display,
                window,
                self.atom(c"_NET_WM_NAME"),
                self.atom(c"UTF8_STRING"),
                8,
                0,
                title.as_bytes().as_ptr(),
                title.as_bytes().len() as i32,
            );
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn show(&self, window: u64, visible: bool) {
        if std::env::var_os("PEEKGAME_TRACE_WINDOW").is_some() {
            eprintln!("[window] {} {window:#x}", if visible { "XMapWindow" } else { "XUnmapWindow" });
        }
        unsafe {
            if visible {
                ffi::XMapWindow(self.display, window);
            } else {
                ffi::XUnmapWindow(self.display, window);
            }
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn set_icon(&self, window: u64, width: u32, height: u32, pixels: &[u32]) {
        let values: Vec<u64> = if pixels.is_empty() {
            Vec::new()
        } else {
            std::iter::once(width as u64)
                .chain(std::iter::once(height as u64))
                .chain(pixels.iter().map(|&p| p as u64))
                .collect()
        };
        unsafe {
            ffi::XChangeProperty(
                self.display,
                window,
                self.atom(c"_NET_WM_ICON"),
                6,
                32,
                0,
                values.as_ptr().cast(),
                values.len() as i32,
            );
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn destroy(&self, window: u64) {
        unsafe {
            ffi::XDestroyWindow(self.display, window);
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn move_resize(
        &self,
        window: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        top_level: bool,
    ) {
        let (x, y) = if top_level {
            let (ox, oy) = monitors::origin(self);
            (x + ox, y + oy)
        } else {
            (x, y)
        };
        if std::env::var_os("PEEKGAME_TRACE_WINDOW").is_some() {
            eprintln!("[window] XMoveResizeWindow {window:#x} ({x},{y}) {width}x{height}");
        }
        unsafe {
            ffi::XMoveResizeWindow(self.display, window, x, y, width.max(1), height.max(1));
            ffi::XFlush(self.display);
        }
    }
    /// Fixed-size windows (no WS_THICKFRAME) get equal minimum and maximum sizes.
    pub(crate) fn set_resizable(&self, window: u64, resizable: bool, width: i32, height: i32) {
        let mut hints = types::SizeHints::default();
        let mut supplied=0;unsafe{ffi::XGetWMNormalHints(self.display,window,&mut hints,&mut supplied);}
        hints.flags &= !((1<<4)|(1<<5));
        if !resizable {
            hints.flags |= (1 << 4) | (1 << 5); // PMinSize | PMaxSize
            hints.min_width = width;
            hints.max_width = width;
            hints.min_height = height;
            hints.max_height = height;
        }
        if std::env::var_os("PEEKGAME_TRACE_WINDOW").is_some() {
            if resizable {
                eprintln!("[window] WM_NORMAL_HINTS {window:#x} resizable (flags={:#x})", hints.flags);
            } else {
                eprintln!("[window] WM_NORMAL_HINTS {window:#x} fixed {width}x{height} (flags={:#x})", hints.flags);
            }
        }
        unsafe {
            ffi::XSetWMNormalHints(self.display, window, &mut hints);
            ffi::XFlush(self.display);
        }
    }
    /// Window-manager decorations on or off (_MOTIF_WM_HINTS).
    pub(crate) fn set_decorated(&self, window: u64, decorated: bool) {
        let hints: [u64; 5] = [
            2, /* MWM_HINTS_DECORATIONS */
            0,
            decorated as u64,
            0,
            0,
        ];
        let atom = self.atom(c"_MOTIF_WM_HINTS");
        unsafe {
            ffi::XChangeProperty(
                self.display,
                window,
                atom,
                atom,
                32,
                0,
                hints.as_ptr().cast(),
                5,
            );
            ffi::XFlush(self.display);
        }
    }
    /// Adds or removes _NET_WM_STATE entries (fullscreen, maximized) through the
    /// window manager, as EWMH asks for mapped windows.
    pub(crate) fn set_state(
        &self,
        window: u64,
        add: bool,
        first: &std::ffi::CStr,
        second: Option<&std::ffi::CStr>,
    ) {
        let mut event = types::Event { pad: [0; 24] };
        unsafe {
            event.client = types::Client {
                base: types::Any {
                    kind: 33,
                    serial: 0,
                    send_event: 1,
                    display: self.display,
                    window,
                },
                message_type: self.atom(c"_NET_WM_STATE"),
                format: 32,
                data: [
                    add as i64,
                    self.atom(first) as i64,
                    second.map_or(0, |s| self.atom(s)) as i64,
                    1,
                    0,
                ],
            };
            ffi::XSendEvent(
                self.display,
                self.root,
                0,
                (1 << 20) | (1 << 19),
                &mut event,
            );
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn iconify(&self, window: u64) {
        unsafe {
            ffi::XIconifyWindow(self.display, window, self.screen);
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn raise_and_focus(&self, window: u64) {
        unsafe {
            ffi::XRaiseWindow(self.display, window);
            ffi::XSetInputFocus(self.display, window, 2 /* RevertToParent */, 0);
            ffi::XFlush(self.display);
        }
    }
    pub(crate) fn focus(&self, window: u64) {
        unsafe {
            ffi::XSetInputFocus(self.display, window, 2, 0);
            ffi::XFlush(self.display);
        }
    }
    /// Screen position of a window's origin.
    pub(crate) fn origin(&self, window: u64) -> (i32, i32) {
        let (mut x, mut y, mut child) = (0, 0, 0);
        unsafe {
            ffi::XTranslateCoordinates(
                self.display,
                window,
                self.root,
                0,
                0,
                &mut x,
                &mut y,
                &mut child,
            )
        };
        let (ox, oy) = monitors::origin(self);
        (x - ox, y - oy)
    }
    /// Pointer position on the screen and the button/modifier mask.
    pub(crate) fn pointer(&self) -> (i32, i32, u32) {
        let (mut root, mut child, mut x, mut y, mut wx, mut wy, mut mask) = (0, 0, 0, 0, 0, 0, 0);
        unsafe {
            ffi::XQueryPointer(
                self.display,
                self.root,
                &mut root,
                &mut child,
                &mut x,
                &mut y,
                &mut wx,
                &mut wy,
                &mut mask,
            )
        };
        let (ox, oy) = monitors::origin(self);
        (x - ox, y - oy, mask)
    }
    pub(crate) fn warp(&self, x: i32, y: i32) {
        let (ox, oy) = monitors::origin(self);
        let (x, y) = (x + ox, y + oy);
        unsafe {
            ffi::XWarpPointer(self.display, 0, self.root, 0, 0, 0, 0, x, y);
            ffi::XFlush(self.display);
        }
    }
    /// Confines the pointer to `window` (ClipCursor) or releases it.
    pub(crate) fn confine(&self, window: Option<u64>) -> bool {
        unsafe {
            let ok = match window {
                Some(w) => {
                    let mask = 4 | 8 | 0x40; // button press/release, motion
                    // owner_events=False prevents XI 2.1 raw motion arriving
                    // twice, through both the grab and the root subscription.
                    ffi::XGrabPointer(self.display, w, 0, mask, 1, 1, w, 0, 0) == 0
                }
                None => {
                    ffi::XUngrabPointer(self.display, 0);
                    true
                }
            };
            ffi::XFlush(self.display);
            ok
        }
    }
    pub(crate) fn define_cursor(&self, window: u64, cursor: u64) {
        unsafe {
            ffi::XDefineCursor(self.display, window, cursor);
            ffi::XFlush(self.display);
        }
    }
    /// The keysym of an X keycode at a shift level (0 plain, 1 shift, 2 AltGr).
    pub(crate) fn keysym(&self, keycode: u32, level: i32) -> u64 {
        unsafe { ffi::XkbKeycodeToKeysym(self.display, keycode as u8, 0, level) }
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
