//! X events to Windows messages, routed to the thread that owns each window.
use super::{
    input, queue,
    state::{self, connection, windows},
    types::*,
};
use crate::host::{
    time,
    x11::{keyboard, NativeEvent},
};
use std::sync::{atomic::Ordering, Mutex};
static PUMP: Mutex<Pump> = Mutex::new(Pump {
    last_click: (0, 0, 0, 0, 0),
});
struct Pump {
    /// Last button press: window, button, time, x, y (for double clicks).
    last_click: (usize, u32, u32, i32, i32),
}
/// MK_* flags from an X state mask.
fn mouse_keys(state: u32) -> usize {
    let mut k = 0;
    if state & 0x100 != 0 {
        k |= 1; // MK_LBUTTON
    }
    if state & 0x400 != 0 {
        k |= 2; // MK_RBUTTON
    }
    if state & 1 != 0 {
        k |= 4; // MK_SHIFT
    }
    if state & 4 != 0 {
        k |= 8; // MK_CONTROL
    }
    if state & 0x200 != 0 {
        k |= 0x10; // MK_MBUTTON
    }
    if input::key_down(5) {
        k |= 0x20;
    }
    if input::key_down(6) {
        k |= 0x40;
    }
    k
}
fn post(hwnd: usize, mut m: Message) {
    if let Some(thread) = queue::owner(hwnd) {
        m.window = hwnd;
        queue::post_to(thread, m);
    }
}
/// Reads pending X events (one thread at a time) and turns them into messages.
pub(super) fn pump() {
    let Ok(mut pump) = PUMP.try_lock() else {
        return;
    };
    let Ok(c) = connection() else {
        return;
    };
    if windows().is_empty() {
        return;
    }
    for event in c.events() {
        let Some(hwnd) = state::by_native(event.window()) else {
            continue;
        };
        let now = time::milliseconds();
        match event {
            NativeEvent::Xdnd {event,..} => super::ole_drop::enqueue(hwnd,event),
            NativeEvent::Configure {width,height,x,y,..}=>{
                let (x,y)=if state::with_window(hwnd,|w|w.parent==0).unwrap_or(false){state::client_origin(hwnd).unwrap_or((x,y))}else{(x,y)};
                if std::env::var_os("PEEKGAME_TRACE_WINDOW").is_some() {
                    let expected=state::with_window(hwnd,|w|w.expected).ok().flatten();
                    eprintln!("[window] X Configure {hwnd:#x} ({x},{y}) {width}x{height} expected={expected:?}");
                }
                let changed=state::with_window(hwnd,|w|{
                    let moved=(w.x,w.y)!=(x,y);w.x=x;w.y=y;
                    let resized=(w.width,w.height)!=(width,height);w.expected=None;
                    w.width=width;w.height=height;
                    if resized{w.paint=true;if let Some(s)=&w.surface{s.raw.width.store(width.max(1) as u32,Ordering::Release);s.raw.height.store(height.max(1) as u32,Ordering::Release);}}
                    (moved,resized,if w.maximized{2}else{0})
                });
                if let Ok((moved,resized,_kind))=changed{
                    if moved || resized {
                        queue::notify_position(hwnd, WindowPos {
                            window: hwnd, after: 0, x, y, width, height,
                            // SWP_NOZORDER | SWP_NOACTIVATE, plus unchanged fields.
                            flags: 4 | 0x10 | if moved { 0 } else { 2 } | if resized { 0 } else { 1 },
                        });
                    }
                    if moved{super::windows::update_fullscreen(hwnd);}
                }
            }
            NativeEvent::Close { .. } => post(hwnd, queue::message(hwnd, WM_CLOSE, 0, 0)),
            NativeEvent::Key {
                down,
                repeat,
                scan,
                base,
                character,
                state,
                time,
                x,
                y,
                ..
            } => {
                input::sync_toggles(state);
                let num_lock = state & 0x10 != 0;
                let vk = keyboard::virtual_key(scan, base, num_lock, state & 1 != 0);
                if vk == 0 {
                    continue;
                }
                let specific = keyboard::positional_vk(scan, false);
                let was_down = input::key_down(vk);
                input::set_key(vk, down);
                if specific != vk && specific != 0 {
                    input::set_key(specific, down);
                }
                let alt = input::key_down(0x12);
                let ctrl = input::key_down(0x11);
                // Alt (without Ctrl) and F10 make the system key messages.
                let system = (alt && !ctrl) || vk == 0x79 || vk == 0x12;
                let message = match (down, system) {
                    (true, false) => WM_KEYDOWN,
                    (false, false) => WM_KEYUP,
                    (true, true) => WM_SYSKEYDOWN,
                    (false, true) => WM_SYSKEYUP,
                };
                let lparam = 1u32
                    | ((scan & 0xff) as u32) << 16
                    | if scan & 0xe000 != 0 { 1 << 24 } else { 0 }
                    | if alt { 1 << 29 } else { 0 }
                    | if !down || repeat || was_down {
                        1 << 30
                    } else {
                        0
                    }
                    | if down { 0 } else { 1 << 31 };
                input::raw_keyboard(hwnd, scan, specific.max(vk), down, message);
                post(
                    hwnd,
                    Message {
                        window: hwnd,
                        message,
                        wparam: vk as usize,
                        lparam: lparam as i32 as isize,
                        time: if time != 0 { time } else { now },
                        point: Point { x, y },
                        // The character for TranslateMessage.
                        private: if down { character.unwrap_or(0) } else { 0 },
                    },
                );
            }
            NativeEvent::Button {
                down,
                button,
                state,
                time,
                x,
                y,
                x_root,
                y_root,
                ..
            } => {
                let target = match input::capture() {
                    0 => hwnd,
                    c => c,
                };
                let (x, y) = if target != hwnd {
                    let o = state::client_origin(target).unwrap_or((0, 0));
                    (x_root - o.0, y_root - o.1)
                } else {
                    (x, y)
                };
                let time = if time != 0 { time } else { now };
                // X state is from before the event: apply this button.
                let bit = match button {
                    1 => 0x100,
                    2 => 0x200,
                    3 => 0x400,
                    _ => 0,
                };
                let state = if down { state | bit } else { state & !bit };
                match button {
                    1..=3 | 8 | 9 => {
                        let (vk, raw_down, raw_up): (u8, u16, u16) = match button {
                            1 => (1, 1, 2),
                            3 => (2, 4, 8),
                            2 => (4, 0x10, 0x20),
                            8 => (5, 0x40, 0x80),
                            _ => (6, 0x100, 0x200),
                        };
                        input::set_key(vk, down);
                        input::raw_mouse(hwnd, if down { raw_down } else { raw_up }, 0, None);
                        let keys = mouse_keys(state);
                        let (message, wparam) = match button {
                            1 => (0x201, keys),
                            3 => (0x204, keys),
                            2 => (0x207, keys),
                            b => (WM_XBUTTONDOWN, keys | (if b == 8 { 1 } else { 2 }) << 16),
                        };
                        let mut message = if down { message } else { message + 1 };
                        if down {
                            // CS_DBLCLKS windows get double clicks.
                            let (lw, lb, lt, lx, ly) = pump.last_click;
                            let dbl = lw == target
                                && lb == button
                                && time.wrapping_sub(lt) <= 500
                                && (lx - x).abs() <= 4
                                && (ly - y).abs() <= 4;
                            let class_dbl = windows()
                                .get(&target)
                                .and_then(|w| super::classes::by_atom(w.class))
                                .is_some_and(|c| c.style & 8 != 0);
                            if dbl && class_dbl {
                                message += 2;
                                pump.last_click = (0, 0, 0, 0, 0);
                            } else {
                                pump.last_click = (target, button, time, x, y);
                            }
                        }
                        post(
                            target,
                            Message {
                                window: target,
                                message,
                                wparam,
                                lparam: pack(x, y),
                                time,
                                point: Point {
                                    x: x_root,
                                    y: y_root,
                                },
                                private: 0,
                            },
                        );
                    }
                    4..=7 if down => {
                        let delta: i16 = if button == 4 || button == 7 {
                            120
                        } else {
                            -120
                        };
                        let horizontal = button >= 6;
                        input::raw_mouse(hwnd, if horizontal { 0x800 } else { 0x400 }, delta, None);
                        post(
                            target,
                            Message {
                                window: target,
                                message: if horizontal {
                                    WM_MOUSEHWHEEL
                                } else {
                                    WM_MOUSEWHEEL
                                },
                                wparam: mouse_keys(state) | (delta as u16 as usize) << 16,
                                lparam: pack(x_root, y_root),
                                time,
                                point: Point {
                                    x: x_root,
                                    y: y_root,
                                },
                                private: 0,
                            },
                        );
                    }
                    _ => {}
                }
            }
            NativeEvent::Motion {
                state,
                time,
                x,
                y,
                x_root,
                y_root,
                ..
            } => {
                input::raw_mouse(hwnd, 0, 0, Some((x_root, y_root)));
                let target = match input::capture() {
                    0 => hwnd,
                    c => c,
                };
                let (x, y) = if target != hwnd {
                    let o = state::client_origin(target).unwrap_or((0, 0));
                    (x_root - o.0, y_root - o.1)
                } else {
                    (x, y)
                };
                post(
                    target,
                    Message {
                        window: target,
                        message: WM_MOUSEMOVE,
                        wparam: mouse_keys(state),
                        lparam: pack(x, y),
                        time: if time != 0 { time } else { now },
                        point: Point {
                            x: x_root,
                            y: y_root,
                        },
                        private: 0,
                    },
                );
            }
            NativeEvent::Crossing { entered, .. } => {
                if !entered
                    && state::with_window(hwnd, |w| std::mem::replace(&mut w.track_leave, false))
                        == Ok(true)
                {
                    post(hwnd, queue::message(hwnd, WM_MOUSELEAVE, 0, 0));
                }
            }
            NativeEvent::Focus { gained, .. } => input::focus_changed(hwnd, gained),
            NativeEvent::Expose { .. } => {
                let _ = state::with_window(hwnd, |w| w.paint = true);
            }
            NativeEvent::Map { mapped, .. } => {
                super::input::trace_map(hwnd, mapped);
                if mapped {
                    let restored = state::with_window(hwnd, |w| {
                        let was = w.minimized;
                        w.minimized = false;
                        w.visible = true;
                        (was, w.width, w.height)
                    });
                    if let Ok((true, width, height)) = restored {
                        post(hwnd, queue::message(hwnd, WM_SIZE, 0, pack(width, height)));
                    }
                    input::mapped(hwnd);
                } else {
                    // Unmapped while Windows still shows it: the window manager minimized it.
                    let minimized = state::with_window(hwnd, |w| {
                        if w.own_unmaps > 0 {
                            w.own_unmaps -= 1;
                            return false;
                        }
                        if w.visible && !w.minimized && w.parent == 0 {
                            w.minimized = true;
                            true
                        } else {
                            false
                        }
                    });
                    if minimized == Ok(true) {
                        post(hwnd, queue::message(hwnd, WM_SIZE, 1, 0));
                    }
                }
            }
        }
    }
}
