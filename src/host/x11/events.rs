use super::{ffi, keyboard, types, Connection};
/// What the X server reported, in the terms USER32 needs.
#[derive(Clone, Copy)]
pub(crate) enum NativeEvent {
    Xdnd {
        window: u64,
        event: super::xdnd::Event,
    },
    Configure {
        window: u64,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Close {
        window: u64,
    },
    Key {
        window: u64,
        down: bool,
        /// Autorepeat: a press while the key is already down.
        repeat: bool,
        /// Set-1 scan code (0xE0xx extended).
        scan: u16,
        /// Keysym without modifiers (the layout's base symbol).
        base: u64,
        /// The character typed (modifiers applied), if any.
        character: Option<u32>,
        state: u32,
        time: u32,
        x: i32,
        y: i32,
    },
    Button {
        window: u64,
        down: bool,
        button: u32,
        state: u32,
        time: u32,
        x: i32,
        y: i32,
        x_root: i32,
        y_root: i32,
    },
    Motion {
        window: u64,
        state: u32,
        time: u32,
        x: i32,
        y: i32,
        x_root: i32,
        y_root: i32,
    },
    Crossing {
        window: u64,
        entered: bool,
    },
    Focus {
        window: u64,
        gained: bool,
    },
    Expose {
        window: u64,
    },
    Map {
        window: u64,
        mapped: bool,
    },
}
impl NativeEvent {
    pub(crate) fn window(&self) -> u64 {
        match *self {
            NativeEvent::Xdnd { window, .. }
            | NativeEvent::Configure { window, .. }
            | NativeEvent::Close { window }
            | NativeEvent::Key { window, .. }
            | NativeEvent::Button { window, .. }
            | NativeEvent::Motion { window, .. }
            | NativeEvent::Crossing { window, .. }
            | NativeEvent::Focus { window, .. }
            | NativeEvent::Expose { window }
            | NativeEvent::Map { window, .. } => window,
        }
    }
}
const KEY_PRESS: i32 = 2;
const KEY_RELEASE: i32 = 3;
impl Connection {
    /// Drains the events the server has sent.
    pub(crate) fn events(&self) -> Vec<NativeEvent> {
        let mut result = Vec::new();
        let (ox, oy) = super::monitors::origin(self);
        unsafe {
            while ffi::XPending(self.display) > 0 {
                let mut event = types::Event { pad: [0; 24] };
                ffi::XNextEvent(self.display, &mut event);
                match event.kind {
                    22 => {
                        let e = event.configure;
                        result.push(NativeEvent::Configure {
                            window: e.window,
                            x: e.x,
                            y: e.y,
                            width: e.width,
                            height: e.height,
                        });
                    }
                    33 => {
                        let e = event.client;
                        if let Some(event) = self.xdnd_client(e) {
                            result.push(NativeEvent::Xdnd {
                                window: e.base.window,
                                event,
                            });
                            continue;
                        }
                        if e.message_type == self.protocols
                            && e.format == 32
                            && e.data[0] as u64 == self.delete
                        {
                            result.push(NativeEvent::Close {
                                window: e.base.window,
                            });
                        }
                    }
                    KEY_PRESS | KEY_RELEASE => {
                        let mut e = event.key;
                        let down = e.base.kind == KEY_PRESS;
                        // X reports autorepeat as release + press at the same time.
                        if !down && ffi::XPending(self.display) > 0 {
                            let mut next = types::Event { pad: [0; 24] };
                            ffi::XPeekEvent(self.display, &mut next);
                            if next.kind == KEY_PRESS
                                && next.key.keycode == e.keycode
                                && next.key.time == e.time
                            {
                                ffi::XNextEvent(self.display, &mut next);
                                let mut n = next.key;
                                result.push(self.key(&mut n, true, true));
                                continue;
                            }
                        }
                        result.push(self.key(&mut e, down, false));
                    }
                    4 | 5 => {
                        let e = event.key;
                        result.push(NativeEvent::Button {
                            window: e.base.window,
                            down: e.base.kind == 4,
                            button: e.keycode,
                            state: e.state,
                            time: crate::host::time::x11_timestamp(e.time as u32),
                            x: e.x,
                            y: e.y,
                            x_root: e.x_root - ox,
                            y_root: e.y_root - oy,
                        });
                    }
                    6 => {
                        let e = event.key;
                        result.push(NativeEvent::Motion {
                            window: e.base.window,
                            state: e.state,
                            time: crate::host::time::x11_timestamp(e.time as u32),
                            x: e.x,
                            y: e.y,
                            x_root: e.x_root - ox,
                            y_root: e.y_root - oy,
                        });
                    }
                    7 | 8 => {
                        let e = event.crossing;
                        // Grab/ungrab crossings are not the pointer leaving.
                        if e.mode == 0 {
                            result.push(NativeEvent::Crossing {
                                window: e.base.window,
                                entered: e.base.kind == 7,
                            });
                        }
                    }
                    9 | 10 => {
                        let e = event.focus;
                        if std::env::var_os("PEEKGAME_TRACE_INPUT").is_some() {
                            eprintln!(
                                "INPUT: X {} window={:#x} mode={} detail={}",
                                if e.base.kind == 9 { "FocusIn" } else { "FocusOut" },
                                e.base.window,
                                e.mode,
                                e.detail
                            );
                        }
                        // NotifyPointer details are not focus changes of the window.
                        if e.detail != 5 && e.mode != 1 && e.mode != 2 {
                            result.push(NativeEvent::Focus {
                                window: e.base.window,
                                gained: e.base.kind == 9,
                            });
                        }
                    }
                    31 => {
                        let e = event.selection;
                        result.push(NativeEvent::Xdnd {
                            window: e.requestor,
                            event: super::xdnd::Event::Selection {
                                selection: e.selection,
                                target: e.target,
                                property: e.property,
                                time: e.time,
                            },
                        });
                    }
                    12 => result.push(NativeEvent::Expose {
                        window: event.any.window,
                    }),
                    18 | 19 => {
                        // UnmapNotify/MapNotify: `event` member at the window position.
                        let window = event.configure.window;
                        result.push(NativeEvent::Map {
                            window,
                            mapped: event.kind == 19,
                        });
                    }
                    _ => {}
                }
            }
        }
        result
    }
    unsafe fn key(&self, e: &mut types::Key, down: bool, repeat: bool) -> NativeEvent {
        let base = unsafe { ffi::XLookupKeysym(e, 0) };
        let mut text = [0i8; 16];
        let mut symbol = 0;
        let n = unsafe {
            ffi::XLookupString(e, text.as_mut_ptr(), 16, &mut symbol, std::ptr::null_mut())
        };
        // Control characters (Ctrl+letter) come from the string, others from the keysym.
        let character = if n == 1 && (text[0] as u8) < 0x20 {
            Some(text[0] as u8 as u32)
        } else {
            keyboard::character(symbol)
        };
        NativeEvent::Key {
            window: e.base.window,
            down,
            repeat,
            scan: keyboard::scan(e.keycode),
            base,
            character,
            state: e.state,
            time: crate::host::time::x11_timestamp(e.time as u32),
            x: e.x,
            y: e.y,
        }
    }
}
