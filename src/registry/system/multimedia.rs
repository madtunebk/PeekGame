//! Export declarations for multimedia. Implementation stays in api/.
use crate::api::system::winmm;
use crate::registry::Export;

exports! {
    export!(
        "winmm.dll",
        "timeGetTime",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::time_get_time
    ),
    export!(
        "winmm.dll",
        "timeGetDevCaps",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::time_get_dev_caps
    ),
    export!(
        "winmm.dll",
        "timeBeginPeriod",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::time_period
    ),
    export!(
        "winmm.dll",
        "timeEndPeriod",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::time_period
    ),
    export!(
        "winmm.dll",
        "timeSetEvent",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::time_set_event
    ),
    export!(
        "winmm.dll",
        "timeKillEvent",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::time_kill_event
    ),
    export!(
        "winmm.dll",
        "waveOutGetNumDevs",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::no_devices
    ),
    export!(
        "winmm.dll",
        "waveInGetNumDevs",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::no_devices
    ),
    export!(
        "winmm.dll",
        "midiInGetNumDevs",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::no_devices
    ),
    export!(
        "winmm.dll",
        "midiOutGetNumDevs",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::no_devices
    ),
    export!(
        "winmm.dll",
        "waveOutOpen",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::bad_device
    ),
    export!(
        "winmm.dll",
        "waveInOpen",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::bad_device
    ),
    export!(
        "winmm.dll",
        "waveOutGetDevCapsW",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::bad_device
    ),
    export!(
        "winmm.dll",
        "waveOutGetDevCapsA",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::bad_device
    ),
    export!(
        "winmm.dll",
        "waveInGetDevCapsW",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::bad_device
    ),
    export!(
        "winmm.dll",
        "waveInGetDevCapsA",
        "system",
        "partial",
        "timers; no winmm audio devices",
        winmm::bad_device
    ),
}
