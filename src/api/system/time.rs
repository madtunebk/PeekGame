//! Tick counts, SYSTEMTIME and FILETIME (ported from peek). FILETIME counts 100 ns
//! since 1601-01-01 UTC; SYSTEMTIME is year, month, day of week (0 = Sunday), day,
//! hour, minute, second, milliseconds, and is validated as Windows does.
use crate::api::system::error;
#[repr(C)]
#[derive(Default)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}
#[repr(C)]
#[derive(Default)]
struct Tm {
    second: i32,
    minute: i32,
    hour: i32,
    day: i32,
    month: i32,
    year: i32,
    weekday: i32,
    yearday: i32,
    dst: i32,
    gmtoff: i64,
    zone: usize,
}
unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    fn gmtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    fn timegm(tm: *mut Tm) -> i64;
}
const CLOCK_REALTIME: i32 = 0;
const CLOCK_BOOTTIME: i32 = 7;
/// Seconds between 1601-01-01 and 1970-01-01.
const EPOCH_1601: i64 = 11_644_473_600;

fn now(clock: i32) -> Timespec {
    let mut t = Timespec::default();
    unsafe { clock_gettime(clock, &mut t) };
    t
}
pub extern "win64" fn tick_count64() -> u64 {
    let t = now(CLOCK_BOOTTIME);
    t.seconds as u64 * 1000 + t.nanoseconds as u64 / 1_000_000
}
pub extern "win64" fn tick_count() -> u32 {
    tick_count64() as u32
}
/// GetSystemTimePreciseAsFileTime.
pub unsafe extern "win64" fn precise_file_time(out: *mut u64) {
    let t = now(CLOCK_REALTIME);
    unsafe { *out = ((t.seconds + EPOCH_1601) as u64) * 10_000_000 + t.nanoseconds as u64 / 100 };
}
unsafe fn store(tm: &Tm, milliseconds: u16, out: *mut u16) {
    let fields = [
        tm.year + 1900,
        tm.month + 1,
        tm.weekday,
        tm.day,
        tm.hour,
        tm.minute,
        tm.second.min(59),
    ];
    for (i, v) in fields.iter().enumerate() {
        unsafe { *out.add(i) = *v as u16 };
    }
    unsafe { *out.add(7) = milliseconds };
}
fn fill(out: *mut u16, local: bool) {
    let t = now(CLOCK_REALTIME);
    let mut tm = Tm::default();
    unsafe {
        if local {
            localtime_r(&t.seconds, &mut tm);
        } else {
            gmtime_r(&t.seconds, &mut tm);
        }
        store(&tm, (t.nanoseconds / 1_000_000) as u16, out);
    }
}
pub extern "win64" fn local_time(out: *mut u16) {
    fill(out, true);
}
pub extern "win64" fn system_time(out: *mut u16) {
    fill(out, false);
}
pub unsafe extern "win64" fn file_time_to_system_time(ft: *const u64, st: *mut u16) -> i32 {
    if ft.is_null() || st.is_null() || unsafe { *ft } >> 63 != 0 {
        error::set(87);
        return 0;
    }
    let ft = unsafe { *ft };
    let seconds = (ft / 10_000_000) as i64 - EPOCH_1601;
    let mut tm = Tm::default();
    if unsafe { gmtime_r(&seconds, &mut tm) }.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { store(&tm, (ft / 10_000 % 1000) as u16, st) };
    1
}
pub unsafe extern "win64" fn system_time_to_file_time(st: *const u16, ft: *mut u64) -> i32 {
    if st.is_null() || ft.is_null() {
        error::set(87);
        return 0;
    }
    let s = unsafe { std::slice::from_raw_parts(st, 8) };
    let (year, month, day) = (s[0] as i32, s[1] as usize, s[3] as u32);
    const DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    if !(1601..=30827).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > DAYS[month - 1] + (month == 2 && leap) as u32
        || s[4] > 23
        || s[5] > 59
        || s[6] > 59
        || s[7] > 999
    {
        error::set(87);
        return 0;
    }
    let mut tm = Tm {
        year: year - 1900,
        month: month as i32 - 1,
        day: day as i32,
        hour: s[4] as i32,
        minute: s[5] as i32,
        second: s[6] as i32,
        ..Tm::default()
    };
    let seconds = unsafe { timegm(&mut tm) };
    unsafe { *ft = ((seconds + EPOCH_1601) as u64) * 10_000_000 + s[7] as u64 * 10_000 };
    1
}
