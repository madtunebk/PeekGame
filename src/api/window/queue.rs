//! Per-thread message queues (ported from peek's win32_window.inc model).
//! - Posted messages wait in the owner thread's queue; any thread may post.
//! - Sent messages run the window procedure on the owner thread: directly when the
//!   sender is that thread, otherwise when the owner next retrieves messages (the
//!   sender waits for the result, handling messages sent to itself meanwhile).
//! - WM_PAINT and WM_TIMER are generated when nothing posted matches.
//! - X events are read by whichever thread retrieves messages and routed to the
//!   owner of the window they concern.
use super::{pump, state, types::*};
use crate::{api::system::error, host::time};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
pub(super) struct Reply {
    value: Mutex<Option<isize>>,
    done: Condvar,
}
struct Sent {
    message: Message,
    reply: Option<Arc<Reply>>,
    position: Option<WindowPos>,
}
struct Timer {
    window: usize,
    id: usize,
    interval: u32,
    due: Instant,
    procedure: usize,
}
#[derive(Default)]
struct Queue {
    posted: VecDeque<Message>,
    sent: VecDeque<Sent>,
    timers: Vec<Timer>,
    last_time: u32,
    last_point: Point,
    /// QS_* bits of messages added since the last GetQueueStatus/GetMessage/PeekMessage.
    added: u32,
}
static QUEUES: Mutex<Option<HashMap<u32, Queue>>> = Mutex::new(None);
static SIGNAL: Condvar = Condvar::new();
fn with<T>(thread: u32, f: impl FnOnce(&mut Queue) -> T) -> T {
    let mut queues = QUEUES.lock().unwrap();
    f(queues
        .get_or_insert_with(HashMap::new)
        .entry(thread)
        .or_default())
}
pub(super) fn notify() {
    SIGNAL.notify_all();
}
pub(super) fn owner(hwnd: usize) -> Option<u32> {
    state::windows().get(&hwnd).map(|w| w.owner)
}
/// Merge motion only within the current run of mouse messages. A button, wheel,
/// keyboard or other window's event is an ordering boundary.
pub(super) fn merge_pending_raw_mouse(
    thread: u32,
    window: usize,
    dx: i32,
    dy: i32,
    point: Point,
) -> bool {
    with(thread, |q| {
        for pending in q.posted.iter_mut().rev() {
            if pending.window != window {
                return false;
            }
            match pending.message {
                WM_MOUSEMOVE => continue,
                WM_INPUT => {
                    if super::input::merge_raw_mouse(thread, pending.lparam as usize, dx, dy) {
                        pending.time = time::milliseconds();
                        pending.point = point;
                        return true;
                    }
                    return false;
                }
                _ => return false,
            }
        }
        false
    })
}
/// Posts to `thread`'s queue. Coalesce mouse motion across unread raw-motion
/// messages, without moving it across button, wheel or keyboard input.
pub(super) fn post_to(thread: u32, message: Message) {
    if matches!(message.message, 0xff | 0x200..=0x209) {
        super::input::trace_input(format_args!(
            "post thread={thread} hwnd={:#x} msg={:#x} wp={:#x} lp={:#x} time={} now={}",
            message.window,
            message.message,
            message.wparam,
            message.lparam,
            message.time,
            time::milliseconds()
        ));
    }
    with(thread, |q| {
        if message.message == WM_MOUSEMOVE {
            let mut stale = None;
            for (index, pending) in q.posted.iter().enumerate().rev() {
                if pending.window != message.window {
                    break;
                }
                match pending.message {
                    WM_MOUSEMOVE => {
                        stale = Some(index);
                        break;
                    }
                    WM_INPUT if super::input::is_pending_mouse_motion(thread, pending.lparam as usize) => {}
                    _ => break,
                }
            }
            if let Some(index) = stale {
                q.posted.remove(index);
            }
        }
        q.added |= category(message.message) | QS_ALLPOSTMESSAGE;
        q.posted.push_back(message);
    });
    notify();
}
/// Buffered raw reads consume only their WM_INPUT messages on the calling thread.
pub(super) fn consume_raw(handles: &[usize]) {
    with(state::current_thread(), |q| {
        q.posted
            .retain(|m| m.message != WM_INPUT || !handles.contains(&(m.lparam as usize)))
    });
}
pub(super) fn message(hwnd: usize, message: u32, wparam: usize, lparam: isize) -> Message {
    Message {
        window: hwnd,
        message,
        wparam,
        lparam,
        time: time::milliseconds(),
        ..Message::default()
    }
}
/// A notification for the owner thread's window procedure, run when it next
/// retrieves messages (or now, on the owner thread).
pub(super) fn notify_window(hwnd: usize, msg: u32, wparam: usize, lparam: isize) {
    let Some(thread) = owner(hwnd) else {
        return;
    };
    with(thread, |q| {
        q.added |= QS_SENDMESSAGE;
        q.sent.push_back(Sent {
            message: message(hwnd, msg, wparam, lparam),
            reply: None,
            position: None,
        })
    });
    notify();
}
/// Own WINDOWPOS until the owner thread invokes its procedure. Never queue a
/// pointer into the event pump's stack or invoke game code under its mutex.
pub(super) fn notify_position(hwnd: usize, position: WindowPos) {
    let Some(thread) = owner(hwnd) else { return; };
    with(thread, |q| {
        q.added |= QS_SENDMESSAGE;
        q.sent.push_back(Sent {
            message: message(hwnd, WM_WINDOWPOSCHANGED, 0, 0),
            reply: None,
            position: Some(position),
        })
    });
    notify();
}
pub(super) fn procedure(hwnd: usize) -> Option<WndProc> {
    state::windows().get(&hwnd).map(|w| w.procedure)
}
/// Calls a window procedure (no lock held).
pub(super) fn call(hwnd: usize, msg: u32, wparam: usize, lparam: isize) -> isize {
    match procedure(hwnd) {
        Some(p) => unsafe { p(hwnd, msg, wparam, lparam) },
        None => 0,
    }
}
/// Runs the messages other threads sent to this one.
pub(super) fn run_sent() -> bool {
    let me = state::current_thread();
    let mut ran = super::ole_drop::run_pending();
    loop {
        let Some(mut sent) = with(me, |q| q.sent.pop_front()) else {
            return ran;
        };
        ran = true;
        let mut m = sent.message;
        if let Some(position) = sent.position.as_mut() {
            m.lparam = position as *mut WindowPos as isize;
        }
        let result = call(m.window, m.message, m.wparam, m.lparam);
        if let Some(reply) = sent.reply {
            *reply.value.lock().unwrap() = Some(result);
            reply.done.notify_all();
        }
    }
}
/// SendMessage: on the owner thread directly, else through its queue. None: the
/// timeout passed first.
pub(super) fn send(
    hwnd: usize,
    msg: u32,
    wparam: usize,
    lparam: isize,
    timeout: Option<Duration>,
) -> Option<isize> {
    let thread = owner(hwnd)?;
    if thread == state::current_thread() {
        return Some(call(hwnd, msg, wparam, lparam));
    }
    let reply = Arc::new(Reply {
        value: Mutex::new(None),
        done: Condvar::new(),
    });
    with(thread, |q| {
        q.added |= QS_SENDMESSAGE;
        q.sent.push_back(Sent {
            message: message(hwnd, msg, wparam, lparam),
            reply: Some(reply.clone()),
            position: None,
        })
    });
    notify();
    let deadline = timeout.map(|t| Instant::now() + t);
    loop {
        run_sent();
        let value = reply.value.lock().unwrap();
        if let Some(v) = *value {
            return Some(v);
        }
        if deadline.is_some_and(|d| Instant::now() >= d) || !state::exists(hwnd) {
            return None;
        }
        let _ = reply
            .done
            .wait_timeout(value, Duration::from_millis(2))
            .unwrap();
    }
}

/// Does `m` pass a PeekMessage/GetMessage filter? WM_QUIT passes any.
fn matches(m: &Message, hwnd: usize, min: u32, max: u32) -> bool {
    if m.message == WM_QUIT {
        return true;
    }
    let window = match hwnd {
        0 => true,
        usize::MAX => m.window == 0,
        h => m.window == h || is_child_of(m.window, h),
    };
    window && in_range(m.message, min, max)
}
fn in_range(message: u32, min: u32, max: u32) -> bool {
    (min == 0 && max == 0) || (message >= min && message <= max)
}
fn is_child_of(child: usize, parent: usize) -> bool {
    let windows = state::windows();
    let mut current = child;
    while let Some(w) = windows.get(&current) {
        if w.parent == 0 {
            return false;
        }
        if w.parent == parent {
            return true;
        }
        current = w.parent;
    }
    false
}
/// One retrieval: sent messages run, then a posted message, a WM_PAINT or a due
/// WM_TIMER that passes the filter. PM_REMOVE (1) takes it out of the queue.
pub(super) fn retrieve(hwnd: usize, min: u32, max: u32, remove: bool) -> Option<Message> {
    pump::pump();
    run_sent();
    let me = state::current_thread();
    with(me, |q| q.added = 0);
    let found = loop {
        let candidate = with(me, |q| {
            let index = q.posted.iter().position(|m| matches(m, hwnd, min, max))?;
            let m = if remove {
                q.posted.remove(index)?
            } else {
                q.posted[index]
            };
            q.last_time = m.time;
            q.last_point = m.point;
            Some(m)
        });
        if let Some(m) = candidate {
            if super::hooks::mouse(&m, remove) {
                continue;
            }
        }
        break candidate;
    };
    if let Some(m) = found {
        if remove && m.message == WM_INPUT {
            super::input::raw_read(m.lparam as usize);
        }
        if remove
            && m.message == WM_MOUSEMOVE
            && state::exists(m.window)
            && super::input::capture() == 0
        {
            // HTCLIENT, the mouse message in the high word.
            call(
                m.window,
                WM_SETCURSOR,
                m.window,
                (WM_MOUSEMOVE as isize) << 16 | 1,
            );
        }
        return Some(m);
    }
    if hwnd != usize::MAX && in_range(WM_PAINT, min, max) {
        let painting = state::windows()
            .iter()
            .find(|(&h, w)| w.owner == me && w.paint && (hwnd == 0 || hwnd == h))
            .map(|(&h, _)| h);
        if let Some(h) = painting {
            return Some(message(h, WM_PAINT, 0, 0));
        }
    }
    if in_range(WM_TIMER, min, max) {
        let now = Instant::now();
        let due = with(me, |q| {
            let t = q.timers.iter_mut().find(|t| {
                t.due <= now
                    && (hwnd == 0 || (hwnd == usize::MAX && t.window == 0) || t.window == hwnd)
            })?;
            if remove {
                t.due = now + Duration::from_millis(t.interval as u64);
            }
            Some(message(t.window, WM_TIMER, t.id, t.procedure as isize))
        });
        if due.is_some() {
            return due;
        }
    }
    None
}
/// Waits up to `limit` for something to retrieve (X events are read every few
/// milliseconds, posts and sends wake at once).
pub(super) fn wait(limit: Duration) {
    let me = state::current_thread();
    let queues = QUEUES.lock().unwrap();
    let busy = queues
        .as_ref()
        .and_then(|q| q.get(&me))
        .is_some_and(|q| !q.posted.is_empty() || !q.sent.is_empty());
    if busy {
        return;
    }
    let _ = SIGNAL
        .wait_timeout(queues, limit.min(Duration::from_millis(4)))
        .unwrap();
}
/// Is there input for this thread (MsgWaitForMultipleObjects)?
pub(super) fn has_input(mask: u32) -> bool {
    pump::pump();
    let me = state::current_thread();
    let posted = with(me, |q| {
        !q.sent.is_empty()
            || q.posted
                .iter()
                .any(|m| mask == 0 || mask & category(m.message) != 0)
            || ((mask == 0 || mask & 0x10 != 0) && q.timers.iter().any(|t| t.due <= Instant::now()))
    });
    posted
        || ((mask == 0 || mask & 0x20 != 0)
            && state::windows()
                .values()
                .any(|w| w.owner == me && w.paint))
}
const QS_SENDMESSAGE: u32 = 0x40;
const QS_ALLPOSTMESSAGE: u32 = 0x100;
/// GetQueueStatus: high word = QS_* kinds of messages in the calling thread's queue,
/// low word = kinds added since the last GetQueueStatus, GetMessage or PeekMessage;
/// both limited to `flags`, and the reported "added" bits are cleared.
pub(crate) extern "win64" fn get_queue_status(flags: u32) -> u32 {
    const QS_ALL: u32 = 0x1dff; // QS_ALLINPUT, QS_ALLPOSTMESSAGE, QS_RAWINPUT, QS_TOUCH, QS_POINTER
    if flags & !QS_ALL != 0 {
        return fail(1004, 0); // ERROR_INVALID_FLAGS
    }
    pump::pump();
    let me = state::current_thread();
    let now = Instant::now();
    let (mut present, added) = with(me, |q| {
        let mut present = 0;
        for m in &q.posted {
            present |= category(m.message) | QS_ALLPOSTMESSAGE;
        }
        if !q.sent.is_empty() {
            present |= QS_SENDMESSAGE;
        }
        if q.timers.iter().any(|t| t.due <= now) {
            present |= 0x10;
            q.added |= 0x10;
        }
        let added = q.added & flags;
        q.added &= !flags;
        (present, added)
    });
    if state::windows().values().any(|w| w.owner == me && w.paint) {
        present |= 0x20;
    }
    ((present & flags) << 16) | added
}
/// QS_* category of a message.
fn category(message: u32) -> u32 {
    match message {
        0x100..=0x109 => 1, // QS_KEY
        WM_MOUSEMOVE => 2,  // QS_MOUSEMOVE
        0x201..=0x20e => 4, // QS_MOUSEBUTTON
        WM_TIMER => 0x10,   // QS_TIMER
        WM_PAINT => 0x20,   // QS_PAINT
        WM_INPUT => 0x400,  // QS_RAWINPUT
        _ => 8,             // QS_POSTMESSAGE
    }
}
pub(super) fn drop_window(hwnd: usize) {
    let mut queues = QUEUES.lock().unwrap();
    for q in queues.get_or_insert_with(HashMap::new).values_mut() {
        q.posted.retain(|m| m.window != hwnd);
        q.timers.retain(|t| t.window != hwnd);
    }
}
pub(super) fn last_message() -> (u32, Point) {
    with(state::current_thread(), |q| (q.last_time, q.last_point))
}

// ---- API ----------------------------------------------------------------------
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
pub(crate) extern "win64" fn post_message_w(
    hwnd: usize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> i32 {
    if hwnd == 0xffff {
        // HWND_BROADCAST: every top-level window.
        let targets: Vec<(usize, u32)> = state::windows()
            .iter()
            .filter(|(_, w)| w.parent == 0)
            .map(|(&h, w)| (h, w.owner))
            .collect();
        for (h, thread) in targets {
            post_to(thread, message(h, msg, wparam, lparam));
        }
        return 1;
    }
    let thread = if hwnd == 0 {
        state::current_thread()
    } else {
        match owner(hwnd) {
            Some(t) => t,
            None => return fail(ERROR_WINDOW, 0),
        }
    };
    post_to(thread, message(hwnd, msg, wparam, lparam));
    1
}
pub(crate) extern "win64" fn post_thread_message_w(
    thread: u32,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> i32 {
    post_to(thread, message(0, msg, wparam, lparam));
    1
}
pub(crate) extern "win64" fn post_quit_message(code: i32) {
    post_message_w(0, WM_QUIT, code as u32 as usize, 0);
}
pub(crate) extern "win64" fn send_message_w(
    hwnd: usize,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if hwnd == 0xffff {
        let targets: Vec<usize> = state::windows()
            .iter()
            .filter(|(_, w)| w.parent == 0)
            .map(|(&h, _)| h)
            .collect();
        for h in targets {
            send(h, msg, wparam, lparam, None);
        }
        return 0;
    }
    match send(hwnd, msg, wparam, lparam, None) {
        Some(v) => v,
        None => fail(ERROR_WINDOW, 0),
    }
}
/// SendMessageTimeout: 0 with ERROR_TIMEOUT (1460) when the owner does not answer.
pub(crate) unsafe extern "win64" fn send_message_timeout_w(
    hwnd: usize,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _flags: u32,
    timeout: u32,
    result: *mut usize,
) -> isize {
    if !state::exists(hwnd) {
        return fail(ERROR_WINDOW, 0);
    }
    match send(
        hwnd,
        msg,
        wparam,
        lparam,
        Some(Duration::from_millis(timeout as u64)),
    ) {
        Some(v) => {
            if !result.is_null() {
                unsafe { *result = v as usize };
            }
            1
        }
        None => fail(1460, 0),
    }
}
pub(crate) unsafe extern "win64" fn peek_message_w(
    output: *mut Message,
    hwnd: usize,
    min: u32,
    max: u32,
    flags: u32,
) -> i32 {
    if output.is_null() || (max != 0 && min > max) {
        return fail(87, 0);
    }
    if hwnd != 0 && hwnd != usize::MAX && !state::exists(hwnd) {
        return fail(ERROR_WINDOW, 0);
    }
    match retrieve(hwnd, min, max, flags & 1 != 0) {
        Some(m) => {
            unsafe { output.write(m) };
            1
        }
        None => 0,
    }
}
/// GetMessage: 0 for WM_QUIT, -1 for an invalid window.
pub(crate) unsafe extern "win64" fn get_message_w(
    output: *mut Message,
    hwnd: usize,
    min: u32,
    max: u32,
) -> i32 {
    if output.is_null() {
        return fail(87, -1);
    }
    loop {
        if hwnd != 0 && hwnd != usize::MAX && !state::exists(hwnd) {
            return fail(ERROR_WINDOW, -1);
        }
        if let Some(m) = retrieve(hwnd, min, max, true) {
            unsafe { output.write(m) };
            return (m.message != WM_QUIT) as i32;
        }
        wait(Duration::from_millis(4));
    }
}
pub(crate) extern "win64" fn wait_message() -> i32 {
    while !has_input(0) {
        wait(Duration::from_millis(4));
    }
    1
}
pub(crate) unsafe extern "win64" fn dispatch_message_w(message: *const Message) -> isize {
    if message.is_null() {
        return fail(87, 0);
    }
    let m = unsafe { *message };
    if m.message == WM_TIMER && m.lparam != 0 {
        // TIMERPROC(hwnd, WM_TIMER, id, time).
        let procedure: unsafe extern "win64" fn(usize, u32, usize, u32) =
            unsafe { std::mem::transmute(m.lparam) };
        unsafe { procedure(m.window, WM_TIMER, m.wparam, time::milliseconds()) };
        return 0;
    }
    if m.window == 0 {
        return 0;
    }
    match procedure(m.window) {
        Some(p) => unsafe { p(m.window, m.message, m.wparam, m.lparam) },
        None => fail(ERROR_WINDOW, 0),
    }
}
pub(crate) extern "win64" fn get_message_time() -> i32 {
    last_message().0 as i32
}
pub(crate) extern "win64" fn get_message_pos() -> u32 {
    let (_, p) = last_message();
    static LAST: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);
    let point = (p.x as u32 as u64 | ((p.y as u32 as u64) << 32)) as i64;
    if LAST.swap(point, std::sync::atomic::Ordering::Relaxed) != point {
        super::input::trace_input(format_args!("GetMessagePos {},{}", p.x, p.y));
    }
    pack(p.x, p.y) as u32
}
pub(crate) extern "win64" fn get_message_extra_info() -> isize {
    0
}
pub(crate) extern "win64" fn in_send_message() -> i32 {
    0
}
/// SetTimer: WM_TIMER for `hwnd` (or a thread timer with a new id when hwnd is 0).
pub(crate) extern "win64" fn set_timer(
    hwnd: usize,
    id: usize,
    interval: u32,
    procedure: usize,
) -> usize {
    if hwnd != 0 && !state::exists(hwnd) {
        return fail(ERROR_WINDOW, 0);
    }
    let interval = interval.clamp(10, 0x7fff_ffff);
    let me = state::current_thread();
    with(me, |q| {
        let id = if hwnd == 0 && id == 0 {
            0x7000 + q.timers.len() + 1
        } else {
            id
        };
        q.timers.retain(|t| !(t.window == hwnd && t.id == id));
        q.timers.push(Timer {
            window: hwnd,
            id,
            interval,
            due: Instant::now() + Duration::from_millis(interval as u64),
            procedure,
        });
        if hwnd == 0 {
            id
        } else {
            id.max(1)
        }
    })
}
pub(crate) extern "win64" fn kill_timer(hwnd: usize, id: usize) -> i32 {
    let me = state::current_thread();
    let removed = with(me, |q| {
        let before = q.timers.len();
        q.timers.retain(|t| !(t.window == hwnd && t.id == id));
        before != q.timers.len()
    });
    if removed {
        1
    } else {
        fail(1402, 0) // ERROR_INVALID_TIMER... as Windows: ERROR_INVALID_PARAMETER-like
    }
}
/// MsgWaitForMultipleObjectsEx: WAIT_OBJECT_0 + n when input in `mask` is queued;
/// MWMO_ALERTABLE (2) also returns for APCs.
pub(crate) unsafe extern "win64" fn msg_wait_ex(
    count: u32,
    handles: *const usize,
    timeout: u32,
    mask: u32,
    flags: u32,
) -> u32 {
    let deadline =
        (timeout != u32::MAX).then(|| Instant::now() + Duration::from_millis(timeout as u64));
    let alertable = flags & 2 != 0;
    let wait_all = flags & 1 != 0;
    loop {
        if count > 0 {
            let r = unsafe {
                crate::api::threading::objects::wait_multiple_ex(
                    count,
                    handles,
                    wait_all as i32,
                    0,
                    alertable as i32,
                )
            };
            if r != 258 {
                return r;
            }
        } else if alertable && crate::api::threading::apc::run() {
            return crate::api::threading::apc::WAIT_IO_COMPLETION;
        }
        if !wait_all && has_input(mask) {
            return count;
        }
        let left = match deadline {
            Some(d) => {
                let now = Instant::now();
                if now >= d {
                    return 258;
                }
                d - now
            }
            None => Duration::from_millis(4),
        };
        wait(left);
    }
}
pub(crate) unsafe extern "win64" fn msg_wait(
    count: u32,
    handles: *const usize,
    wait_all: i32,
    timeout: u32,
    mask: u32,
) -> u32 {
    unsafe { msg_wait_ex(count, handles, timeout, mask, (wait_all != 0) as u32) }
}
/// RegisterWindowMessage: one id (0xC000..0xFFFF) per name, case-insensitively.
pub(crate) fn register_message(name: &str) -> u32 {
    if name.is_empty() || name.encode_utf16().count() > 255 {
        error::set(87);
        return 0;
    }
    static NAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut names = NAMES.lock().unwrap();
    let index = match names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
        Some(i) => i,
        None => {
            if names.len() >= 0x4000 {
                error::set(8);
                return 0;
            }
            names.push(name.to_owned());
            names.len() - 1
        }
    };
    0xc000 + index as u32
}
