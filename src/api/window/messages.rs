use super::{pump, state::STATE, types::*};
use crate::{api::system::error, host::time};
pub(crate) extern "win64" fn post_message_w(
    hwnd: usize,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> i32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if hwnd != 0 && !state.windows.contains_key(&hwnd) {
            error::set(ERROR_WINDOW);
            return 0;
        }
        state.queue.push_back(Message {
            window: hwnd,
            message,
            wparam,
            lparam,
            time: time::milliseconds(),
            ..Message::default()
        });
        1
    })
}
pub(crate) extern "win64" fn post_quit_message(code: i32) {
    post_message_w(0, WM_QUIT, code as u32 as usize, 0);
}
pub(crate) unsafe extern "win64" fn peek_message_w(
    output: *mut Message,
    hwnd: usize,
    min: u32,
    max: u32,
    flags: u32,
) -> i32 {
    if output.is_null() || flags > 1 || (max != 0 && min > max) {
        error::set(87);
        return 0;
    }
    pump::pump();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if hwnd != 0 && hwnd != usize::MAX && !state.windows.contains_key(&hwnd) {
            error::set(ERROR_WINDOW);
            return 0;
        }
        let index = state.queue.iter().position(|m| {
            m.message == WM_QUIT
                || ((hwnd == 0 || (hwnd == usize::MAX && m.window == 0) || m.window == hwnd)
                    && ((min == 0 && max == 0) || (m.message >= min && m.message <= max)))
        });
        let Some(index) = index else {
            return 0;
        };
        let message = if flags == 1 {
            state.queue.remove(index).unwrap()
        } else {
            state.queue[index]
        };
        unsafe {
            output.write(message);
        }
        1
    })
}
pub(crate) unsafe extern "win64" fn dispatch_message_w(message: *const Message) -> isize {
    if message.is_null() {
        error::set(87);
        return 0;
    }
    let message = unsafe { *message };
    if message.window == 0 {
        return 0;
    }
    let proc = STATE.with(|s| s.borrow().windows.get(&message.window).map(|w| w.procedure));
    match proc {
        Some(proc) => unsafe {
            proc(
                message.window,
                message.message,
                message.wparam,
                message.lparam,
            )
        },
        None => {
            error::set(ERROR_WINDOW);
            0
        }
    }
}
pub(crate) unsafe extern "win64" fn translate_message(message: *const Message) -> i32 {
    if message.is_null() {
        error::set(87);
        return 0;
    }
    let message = unsafe { *message };
    if message.message != WM_KEYDOWN && message.message != WM_KEYUP {
        return 0;
    }
    if message.message == WM_KEYDOWN && message.private != 0 {
        post_message_w(
            message.window,
            WM_CHAR,
            message.private as usize,
            message.lparam,
        );
    }
    1
}
