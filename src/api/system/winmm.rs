//! winmm.dll: multimedia timers (Linux timers already have ~1 ms resolution) and
//! the waveOut/waveIn/MIDI device counts. No winmm audio devices are offered:
//! programs use WASAPI for sound.
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};
pub extern "win64" fn time_get_time() -> u32 {
    crate::host::time::milliseconds()
}
/// TIMECAPS: wPeriodMin, wPeriodMax.
pub unsafe extern "win64" fn time_get_dev_caps(caps: *mut u32, size: u32) -> u32 {
    if caps.is_null() || size < 8 {
        return 97; // TIMERR_NOCANDO
    }
    unsafe {
        *caps = 1;
        *caps.add(1) = 1_000_000;
    }
    0
}
pub extern "win64" fn time_period(period: u32) -> u32 {
    if (1..=1_000_000).contains(&period) {
        0
    } else {
        97
    }
}
type TimerProc = unsafe extern "win64" fn(u32, u32, usize, usize, usize);
static TIMERS: Mutex<Vec<(u32, Arc<AtomicBool>)>> = Mutex::new(Vec::new());
static NEXT: AtomicU32 = AtomicU32::new(1);
/// timeSetEvent: TIME_ONESHOT 0 / TIME_PERIODIC 1 with a callback (TIME_CALLBACK_FUNCTION);
/// event handles (TIME_CALLBACK_EVENT_SET 0x10 / PULSE 0x20) are signalled.
pub extern "win64" fn time_set_event(
    delay: u32,
    _resolution: u32,
    callback: usize,
    user: usize,
    flags: u32,
) -> u32 {
    if callback == 0 {
        return 0;
    }
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let alive = Arc::new(AtomicBool::new(true));
    TIMERS.lock().unwrap().push((id, alive.clone()));
    let periodic = flags & 1 != 0;
    let event = flags & 0x30 != 0;
    let spawned = std::thread::Builder::new()
        .name("winmm-timer".into())
        .spawn(move || {
            if crate::host::thread_template::attach().is_err() {
                return;
            }
            loop {
                std::thread::sleep(std::time::Duration::from_millis(delay.max(1) as u64));
                if !alive.load(Ordering::Acquire) {
                    break;
                }
                if event {
                    crate::api::threading::events::set(callback);
                } else {
                    let f: TimerProc = unsafe { std::mem::transmute(callback) };
                    unsafe { f(id, 0, user, 0, 0) };
                }
                if !periodic {
                    break;
                }
            }
            crate::host::thread_template::detach();
        });
    if spawned.is_err() {
        return 0;
    }
    id
}
pub extern "win64" fn time_kill_event(id: u32) -> u32 {
    let mut timers = TIMERS.lock().unwrap();
    match timers.iter().position(|(t, _)| *t == id) {
        Some(i) => {
            timers.remove(i).1.store(false, Ordering::Release);
            0
        }
        None => 97,
    }
}
pub extern "win64" fn no_devices() -> u32 {
    0
}
/// waveOutOpen/waveInOpen and the capability queries: MMSYSERR_BADDEVICEID.
pub extern "win64" fn bad_device() -> u32 {
    2
}
