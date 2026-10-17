#[repr(C)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}
unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}
pub(crate) fn milliseconds() -> u32 {
    let mut time = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    if unsafe {
        clock_gettime(
            7, /* CLOCK_BOOTTIME: same epoch as GetTickCount */
            &mut time,
        )
    } != 0
    {
        return 0;
    }
    (time.seconds as u64 * 1000 + time.nanoseconds as u64 / 1_000_000) as u32
}

/// X11 timestamps use the server's monotonic milliseconds, while Win32 MSG.time
/// uses GetTickCount (including suspend). Preserve the event's age and u32 wrap.
pub(crate) fn x11_timestamp(timestamp: u32) -> u32 {
    let mut monotonic = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    if unsafe { clock_gettime(1, &mut monotonic) } != 0 {
        return milliseconds();
    }
    let mono = (monotonic.seconds as u64 * 1000 + monotonic.nanoseconds as u64 / 1_000_000) as u32;
    timestamp.wrapping_add(milliseconds().wrapping_sub(mono))
}
