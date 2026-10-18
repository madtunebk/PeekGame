//! IAudioClient state and shared-mode stream lifecycle.
use super::*;
use crate::host::pulse::Playback;
#[derive(Default)]
pub(super) struct State {
    pub stream: Option<Playback>,
    pub format: Option<format::Format>,
    pub pending: Option<Vec<u8>>,
    pub frames: u32,
    pub started: bool,
    pub event_mode: bool,
    pub rate_adjust: bool,
    pub effective_rate: u32,
    pub event: usize,
    pub written: u64,
}
pub(super) unsafe fn state<'a>(this: *mut Object) -> &'a Arc<Mutex<State>> {
    match unsafe { &(*this).kind } {
        Kind::Client(s) | Kind::Render(s) | Kind::Clock(s) | Kind::Adjustment(s) => s,
        _ => unreachable!(),
    }
}
unsafe extern "win64" fn initialize(
    this: *mut Object,
    share: u32,
    flags: u32,
    duration: i64,
    _period: i64,
    wave: *const u8,
    _session: *const u8,
) -> H {
    if std::env::var_os("PEEKGAME_TRACE_AUDIO").is_some() {
        eprintln!("WASAPI Initialize: share={share} flags={flags:#x} duration={duration} period={_period}");
    }
    let mut s = unsafe { state(this) }.lock().unwrap();
    if s.stream.is_some() {
        return 0x88890002u32 as H;
    }
    if share != 0 {
        return 0x8889000eu32 as H;
    }
    if flags & !0x881c0000 != 0 {
        return 0x88890021u32 as H;
    }
    if !(0..=20_000_000).contains(&duration) {
        return ARG;
    }
    let Some(f) = (unsafe { format::parse(wave) }) else {
        return 0x88890008u32 as H;
    };
    if flags & (0x80000000 | 0x100000) == 0 && (f.rate != 48000 || f.channels != 2) {
        return 0x88890008u32 as H;
    }
    let frames = ((duration.max(200_000) as u64 * f.rate as u64).div_ceil(10_000_000) as u32)
        .max(f.rate / 50);
    let stream = match Playback::open(
        f.rate,
        f.channels,
        f.pulse,
        f.align,
        frames,
        flags & 0x100000 != 0,
    ) {
        Ok(p) => p,
        Err(_) => return INVALIDATED,
    };
    eprintln!(
        "WASAPI: {} Hz, {} channels, {}-byte frames, {} frames via PulseAudio/PipeWire ({})",
        f.rate, f.channels, f.align, stream.frames, stream.name
    );
    s.rate_adjust = flags & 0x100000 != 0;
    s.effective_rate = f.rate;
    s.frames = stream.frames;
    s.stream = Some(stream);
    s.format = Some(f);
    s.event_mode = flags & 0x40000 != 0;
    0
}
unsafe extern "win64" fn buffer_size(this: *mut Object, out: *mut u32) -> H {
    if out.is_null() {
        return POINTER;
    }
    let s = unsafe { state(this) }.lock().unwrap();
    if s.stream.is_none() {
        return NOT_INITIALIZED;
    }
    unsafe { *out = s.frames };
    0
}
unsafe extern "win64" fn latency(this: *mut Object, out: *mut i64) -> H {
    if out.is_null() {
        return POINTER;
    }
    let s = unsafe { state(this) }.lock().unwrap();
    if s.format.is_none() {
        return NOT_INITIALIZED;
    }
    unsafe { *out = s.frames as i64 * 10_000_000 / s.effective_rate as i64 };
    0
}
unsafe extern "win64" fn padding(this: *mut Object, out: *mut u32) -> H {
    if out.is_null() {
        return POINTER;
    }
    let s = unsafe { state(this) }.lock().unwrap();
    let Some(p) = &s.stream else {
        return NOT_INITIALIZED;
    };
    match p.padding() {
        Ok(n) => {
            unsafe { *out = n };
            0
        }
        Err(_) => INVALIDATED,
    }
}
unsafe extern "win64" fn supported(
    _: *mut Object,
    share: u32,
    wave: *const u8,
    closest: *mut *mut c_void,
) -> H {
    if !closest.is_null() {
        unsafe { *closest = ptr::null_mut() };
    }
    if share != 0 {
        return 0x8889000eu32 as H;
    }
    let Some(f) = (unsafe { format::parse(wave) }) else {
        return 0x88890008u32 as H;
    };
    if f.rate == 48000 && f.channels == 2 {
        return 0;
    }
    if closest.is_null() {
        return POINTER;
    }
    unsafe { *closest = format::mix() };
    if unsafe { *closest }.is_null() {
        0x8007000eu32 as H
    } else {
        1
    }
}
unsafe extern "win64" fn mix(_: *mut Object, out: *mut *mut c_void) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = format::mix() };
    if unsafe { *out }.is_null() {
        0x8007000eu32 as H
    } else {
        0
    }
}
unsafe extern "win64" fn period(_: *mut Object, normal: *mut i64, minimum: *mut i64) -> H {
    if normal.is_null() && minimum.is_null() {
        return POINTER;
    }
    if !normal.is_null() {
        unsafe { *normal = 100_000 };
    }
    if !minimum.is_null() {
        unsafe { *minimum = 30_000 };
    }
    0
}
unsafe extern "win64" fn start(this: *mut Object) -> H {
    let mut s = unsafe { state(this) }.lock().unwrap();
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    if s.started {
        return 0x88890005u32 as H;
    }
    if s.event_mode && s.event == 0 {
        return 0x88890014u32 as H;
    }
    if stream.cork(false).is_err() {
        return INVALIDATED;
    }
    s.started = true;
    if s.event != 0 {
        crate::api::threading::events::set(s.event);
    }
    0
}
unsafe extern "win64" fn stop(this: *mut Object) -> H {
    let mut s = unsafe { state(this) }.lock().unwrap();
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    if !s.started {
        return 1;
    }
    if stream.cork(true).is_err() {
        return INVALIDATED;
    }
    s.started = false;
    0
}
unsafe extern "win64" fn reset(this: *mut Object) -> H {
    let mut s = unsafe { state(this) }.lock().unwrap();
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    if s.started {
        return 0x88890005u32 as H;
    }
    if s.pending.is_some() {
        return 0x8889000bu32 as H;
    }
    if stream.flush().is_err() {
        return INVALIDATED;
    }
    s.written = 0;
    0
}
unsafe extern "win64" fn event(this: *mut Object, handle: usize) -> H {
    let mut s = unsafe { state(this) }.lock().unwrap();
    let Some(stream) = &s.stream else {
        return NOT_INITIALIZED;
    };
    if !s.event_mode {
        return 0x88890018u32 as H;
    }
    let Some(copy) = crate::api::threading::events::duplicate(handle) else {
        return ARG;
    };
    stream.set_event(copy);
    s.event = copy;
    0
}
unsafe extern "win64" fn service(this: *mut Object, iid: *const u8, out: *mut *mut c_void) -> H {
    if out.is_null() {
        return POINTER;
    }
    unsafe { *out = ptr::null_mut() };
    let shared = unsafe { state(this) };
    if shared.lock().unwrap().stream.is_none() {
        return NOT_INITIALIZED;
    }
    if unsafe { is(iid, &RENDER) } {
        unsafe { *out = object(&super::render::RENDER_TABLE, Kind::Render(shared.clone())) };
        0
    } else if unsafe { is(iid, &ADJUSTMENT) } && shared.lock().unwrap().rate_adjust {
        unsafe { *out = object(&super::adjustment::TABLE, Kind::Adjustment(shared.clone())) };
        0
    } else if unsafe { is(iid, &CLOCK) } {
        unsafe { *out = object(&super::render::CLOCK_TABLE, Kind::Clock(shared.clone())) };
        0
    } else {
        NO_INTERFACE
    }
}
table!(
    CLIENT_TABLE,
    qi,
    addref,
    release,
    initialize,
    buffer_size,
    latency,
    padding,
    supported,
    mix,
    period,
    start,
    stop,
    reset,
    event,
    service
);
