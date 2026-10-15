use super::{ffi, keyboard, types, Connection};
#[derive(Clone, Copy)]
pub(crate) enum NativeEvent {
    Resize {
        window: u64,
        width: i32,
        height: i32,
    },
    Close {
        window: u64,
    },
    Key {
        window: u64,
        down: bool,
        key: usize,
        scan: u8,
        character: u32,
        time: u32,
        x: i32,
        y: i32,
    },
}
impl Connection {
    pub(crate) fn events(&self) -> Vec<NativeEvent> {
        let mut result = Vec::new();
        unsafe {
            while ffi::XPending(self.display) > 0 {
                let mut event = types::Event { pad: [0; 24] };
                ffi::XNextEvent(self.display, &mut event);
                match event.kind {
                    22 => {
                        let e = event.configure;
                        result.push(NativeEvent::Resize {
                            window: e.window,
                            width: e.width,
                            height: e.height,
                        });
                    }
                    33 => {
                        let e = event.client;
                        if e.message_type == self.protocols
                            && e.format == 32
                            && e.data[0] as u64 == self.delete
                        {
                            result.push(NativeEvent::Close {
                                window: e.base.window,
                            });
                        }
                    }
                    2 | 3 => {
                        let mut e = event.key;
                        let symbol = ffi::XLookupKeysym(&mut e, 0);
                        if let Some((key, scan)) = keyboard::key(symbol) {
                            let mut text = [0i8; 16];
                            let mut symbol = 0;
                            let n = ffi::XLookupString(
                                &mut e,
                                text.as_mut_ptr(),
                                16,
                                &mut symbol,
                                std::ptr::null_mut(),
                            );
                            let character = if n == 1 && (1..128).contains(&(text[0] as u8)) {
                                text[0] as u8 as u32
                            } else {
                                0
                            };
                            result.push(NativeEvent::Key {
                                window: e.base.window,
                                down: e.base.kind == 2,
                                key,
                                scan,
                                character,
                                time: e.time as u32,
                                x: e.x,
                                y: e.y,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        result
    }
}
