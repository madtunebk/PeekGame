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
        clock_gettime(1 /* CLOCK_MONOTONIC */, &mut time)
    } != 0
    {
        return 0;
    }
    (time.seconds as u64 * 1000 + time.nanoseconds as u64 / 1_000_000) as u32
}
