use super::{state::STATE, types::*};
use crate::host::{time, x11::NativeEvent};
use std::sync::atomic::Ordering;
pub(super) fn pump() {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let events = state
            .connection
            .as_ref()
            .map(|c| c.events())
            .unwrap_or_default();
        for event in events {
            let native = match event {
                NativeEvent::Resize { window, .. }
                | NativeEvent::Close { window }
                | NativeEvent::Key { window, .. } => window,
            };
            let Some((&hwnd, _)) = state.windows.iter().find(|(_, w)| w.native == native) else {
                continue;
            };
            let mut message = Message {
                window: hwnd,
                time: time::milliseconds(),
                ..Message::default()
            };
            match event {
                NativeEvent::Resize { width, height, .. } => {
                    let window = state.windows.get_mut(&hwnd).unwrap();
                    if width == window.width && height == window.height {
                        continue;
                    }
                    window.width = width;
                    window.height = height;
                    window
                        .surface
                        .raw
                        .width
                        .store(width as u32, Ordering::Release);
                    window
                        .surface
                        .raw
                        .height
                        .store(height as u32, Ordering::Release);
                    message.message = WM_SIZE;
                    message.lparam =
                        ((height as u32 & 0xffff) << 16 | (width as u32 & 0xffff)) as isize;
                }
                NativeEvent::Close { .. } => message.message = WM_CLOSE,
                NativeEvent::Key {
                    down,
                    key,
                    scan,
                    character,
                    time,
                    x,
                    y,
                    ..
                } => {
                    message.message = if down { WM_KEYDOWN } else { WM_KEYUP };
                    message.wparam = key;
                    message.lparam = (1u32 | (scan as u32) << 16 | if down { 0 } else { 3 << 30 })
                        as i32 as isize;
                    message.private = character;
                    message.point = Point { x, y };
                    if time != 0 {
                        message.time = time;
                    }
                }
            }
            state.queue.push_back(message);
        }
    });
}
