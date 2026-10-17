use super::*;
use crate::api::window::drop_data::Owned;
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};
struct Session {
    generation: u64,
    source: u64,
    data: Option<Owned>,
    point: i64,
    keys: u32,
    copy: bool,
    uri: bool,
    entered: bool,
    effect: u32,
    dropped: bool,
    request: Option<(u64, Instant)>,
    failed: bool,
}
thread_local! {static SESSIONS:RefCell<HashMap<usize,Session>>=RefCell::new(HashMap::new());static RUNNING:Cell<bool>=const{Cell::new(false)};}

pub(super) fn forget(hwnd: usize, t: &Target) {
    let session = SESSIONS.with(|all| all.borrow_mut().remove(&hwnd));
    if let Some(mut s) = session {
        end(t, &mut s);
    }
}
fn end(t: &Target, s: &mut Session) {
    if s.entered {
        s.entered = false;
        t.leave();
    }
    if s.dropped {
        t.connection.xdnd_finished(s.source, t.native, false);
    }
}
fn feedback(hwnd: usize, t: &Target, s: &mut Session) {
    s.effect = 0;
    if s.copy && !s.failed {
        if let Some(data) = &s.data {
            let mut effect = 1;
            let hr = if !s.entered {
                s.entered = true;
                t.data_call(3, data.pointer(), s.keys, s.point, &mut effect)
            } else {
                t.over(s.keys, s.point, &mut effect)
            };
            if hr >= 0 && effect & 1 != 0 && live(hwnd, t) {
                s.effect = 1;
            }
        }
    }
    t.connection.xdnd_status(s.source, t.native, s.effect == 1);
}
fn drop_now(hwnd: usize, t: &Target, s: &mut Session) {
    let mut effect = s.effect;
    let ok = if s.entered && effect == 1 && live(hwnd, t) {
        let data = s.data.as_ref().unwrap();
        s.entered = false;
        t.data_call(6, data.pointer(), s.keys, s.point, &mut effect) >= 0 && effect & 1 != 0
    } else {
        if s.entered {
            s.entered = false;
            t.leave();
        }
        false
    };
    t.connection.xdnd_finished(s.source, t.native, ok);
    s.dropped = false;
}
fn handle(hwnd: usize, t: &Target, event: Event) {
    let mut previous = SESSIONS.with(|all| all.borrow_mut().remove(&hwnd));
    if let Event::Client {
        kind: 0,
        source,
        data,
    } = event
    {
        if let Some(s) = &mut previous {
            end(t, s);
        }
        if !live(hwnd, t) {
            return;
        }
        let version = (data[1] as u32 >> 24) as u8;
        let offered = if data[1] & 1 != 0 {
            t.connection.xdnd_types(source)
        } else {
            data[2..5].iter().map(|v| *v as u32 as u64).collect()
        };
        let session = Session {
            generation: t.generation,
            source,
            data: None,
            point: 0,
            keys: 0,
            copy: false,
            uri: version >= 3 && offered.contains(&t.connection.xdnd_uri()),
            entered: false,
            effect: 0,
            dropped: false,
            request: None,
            failed: false,
        };
        SESSIONS.with(|all| all.borrow_mut().insert(hwnd, session));
        return;
    }
    let Some(mut s) = previous else { return };
    if s.generation != t.generation {
        return;
    }
    let mut keep = true;
    match event {
        Event::Client { kind, source, data } if source == s.source => match kind {
            1 => {
                if !s.dropped {
                    let (x, y, keys) = t.connection.xdnd_point(data[2]);
                    s.point = (x as u32 as u64 | ((y as u32 as u64) << 32)) as i64;
                    s.keys = keys;
                    s.copy = t.connection.xdnd_can_copy(s.source, data[4] as u32 as u64);
                    if s.uri && s.copy && s.request.is_none() && !s.failed {
                        let time = data[3] as u32 as u64;
                        s.request = Some((time, Instant::now()));
                        t.connection.xdnd_request(t.native, time);
                    }
                    feedback(hwnd, t, &mut s);
                }
            }
            2 => {
                end(t, &mut s);
                keep = false;
            }
            3 => {
                s.dropped = true;
                if s.data.is_some() || s.failed || s.request.is_none() {
                    drop_now(hwnd, t, &mut s);
                    keep = false;
                }
            }
            _ => {}
        },
        Event::Selection {
            selection,
            target,
            property,
            time,
        } if selection == t.connection.xdnd_selection()
            && target == t.connection.xdnd_uri()
            && s.request.is_some_and(|(sent, _)| sent == time) =>
        {
            if property == t.connection.xdnd_property() {
                s.data = t
                    .connection
                    .xdnd_read(t.native, property, 4 * 1024 * 1024)
                    .filter(|(kind, format, _)| *kind == t.connection.xdnd_uri() && *format == 8)
                    .and_then(|(_, _, bytes)| Owned::from_uri(&bytes));
            }
            s.failed = s.data.is_none();
            feedback(hwnd, t, &mut s);
            if s.dropped {
                drop_now(hwnd, t, &mut s);
                keep = false;
            }
        }
        _ => {}
    }
    if keep && live(hwnd, t) {
        SESSIONS.with(|all| all.borrow_mut().insert(hwnd, s));
    } else if s.entered {
        end(t, &mut s);
    }
}
pub(crate) fn run_pending() -> bool {
    if RUNNING.with(|v| v.replace(true)) {
        return false;
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            RUNNING.with(|v| v.set(false));
        }
    }
    let _reset = Reset;
    let me = state::current_thread();
    let mut ran = false;
    loop {
        let next = {
            let mut pending = PENDING.lock().unwrap();
            pending
                .iter()
                .position(|(owner, _, _)| *owner == me)
                .and_then(|i| pending.remove(i))
        };
        let Some((_, hwnd, event)) = next else { break };
        if let Some(t) = target(hwnd) {
            debug_assert_eq!(t.owner, me);
            handle(hwnd, &t, event);
            ran = true;
        }
    }
    let expired = SESSIONS.with(|all| {
        all.borrow()
            .iter()
            .filter(|(_, s)| {
                s.data.is_none()
                    && s.request
                        .is_some_and(|(_, at)| at.elapsed() > Duration::from_secs(5))
            })
            .map(|(&h, _)| h)
            .collect::<Vec<_>>()
    });
    for hwnd in expired {
        let session = SESSIONS.with(|all| all.borrow_mut().remove(&hwnd));
        if let (Some(t), Some(mut s)) = (target(hwnd), session) {
            end(&t, &mut s);
            t.connection.xdnd_status(s.source, t.native, false);
        }
        ran = true;
    }
    ran
}
