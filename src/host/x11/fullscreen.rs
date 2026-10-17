//! EWMH fullscreen state and real Xinerama monitor selection, before/after map.
use super::{
    ffi,
    monitors::{self, Monitor},
    types, Connection,
};
#[repr(C)]
struct Screen {
    number: i32,
    x: i16,
    y: i16,
    width: i16,
    height: i16,
}
#[link(name = "Xinerama")]
unsafe extern "C" {
    fn XineramaQueryScreens(display: *mut types::Display, count: *mut i32) -> *mut Screen;
}
impl Connection {
    pub(crate) fn fullscreen(&self, window: u64, monitor: Option<&Monitor>) {
        unsafe {
            let mut attributes = types::Attributes::default();
            if ffi::XGetWindowAttributes(self.display, window, &mut attributes) == 0 {
                return;
            }
            let state = self.atom(c"_NET_WM_STATE");
            let full = self.atom(c"_NET_WM_STATE_FULLSCREEN");
            if let Some(m) = monitor {
                let (ox, oy) = monitors::origin(self);
                let mut count = 0;
                let list = XineramaQueryScreens(self.display, &mut count);
                let screen = if !list.is_null() {
                    let found = std::slice::from_raw_parts(list, count.max(0) as usize)
                        .iter()
                        .find(|s| {
                            s.x as i32 == m.x + ox
                                && s.y as i32 == m.y + oy
                                && s.width as i32 == m.width
                                && s.height as i32 == m.height
                        })
                        .map(|s| s.number);
                    ffi::XFree(list.cast());
                    found
                } else {
                    None
                };
                if let Some(screen) = screen {
                    let selection = self.atom(c"_NET_WM_FULLSCREEN_MONITORS");
                    if attributes.map_state == 0 {
                        let values = [screen as u64; 4];
                        ffi::XChangeProperty(
                            self.display,
                            window,
                            selection,
                            6,
                            32,
                            0,
                            values.as_ptr().cast(),
                            4,
                        );
                    } else {
                        let mut event = types::Event {
                            client: types::Client {
                                base: types::Any {
                                    kind: 33,
                                    serial: 0,
                                    send_event: 1,
                                    display: self.display,
                                    window,
                                },
                                message_type: selection,
                                format: 32,
                                data: [
                                    screen as i64,
                                    screen as i64,
                                    screen as i64,
                                    screen as i64,
                                    1,
                                ],
                            },
                        };
                        ffi::XSendEvent(
                            self.display,
                            self.root,
                            0,
                            (1 << 20) | (1 << 19),
                            &mut event,
                        );
                    }
                }
            }
            if attributes.map_state == 0 {
                let mut values = self
                    .xdnd_read(window, state, 4096)
                    .filter(|(kind, format, _)| *kind == 4 && *format == 32)
                    .map(|(_, _, bytes)| {
                        bytes
                            .chunks_exact(8)
                            .map(|b| u64::from_ne_bytes(b.try_into().unwrap()))
                            .filter(|&atom| atom != full)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if monitor.is_some() {
                    values.push(full);
                }
                ffi::XChangeProperty(
                    self.display,
                    window,
                    state,
                    4,
                    32,
                    0,
                    values.as_ptr().cast(),
                    values.len() as i32,
                );
            } else {
                self.set_state(window, monitor.is_some(), c"_NET_WM_STATE_FULLSCREEN", None);
            }
            ffi::XFlush(self.display);
        }
    }
}
