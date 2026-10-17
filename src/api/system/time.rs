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
pub(crate) struct Tm {
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
pub unsafe extern "win64" fn query_unbiased_interrupt_time(out: *mut u64) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let t = now(1); // CLOCK_MONOTONIC
    unsafe { out.write_unaligned(t.seconds as u64 * 10_000_000 + t.nanoseconds as u64 / 100) };
    1
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

// ---- Time zones (ported from peek) ---------------------------------------------

unsafe extern "C" {
    fn tzset();
    fn mktime(tm: *mut Tm) -> i64;
    static tzname: [*const std::ffi::c_char; 2];
}
fn local_tm(seconds: i64) -> Tm {
    let mut tm = Tm::default();
    unsafe { localtime_r(&seconds, &mut tm) };
    tm
}
/// A transition as Windows describes it: month, weekday, week of the month (5 =
/// last) and wall-clock time before the switch; the year is 0 (every year).
fn transition_rule(at: i64, offset_before: i64, out: &mut [u16; 8]) {
    let mut tm = Tm::default();
    unsafe { gmtime_r(&(at + offset_before), &mut tm) };
    *out = [0; 8];
    out[1] = (tm.month + 1) as u16;
    out[2] = tm.weekday as u16;
    let mut week = (tm.day - 1) / 7 + 1;
    let mut next = Tm {
        day: tm.day + 7,
        ..tm
    };
    let probe = unsafe { timegm(&mut next) };
    let mut next = Tm::default();
    unsafe { gmtime_r(&probe, &mut next) };
    if next.month != tm.month {
        week = 5;
    }
    out[3] = week as u16;
    out[4] = tm.hour as u16;
    out[5] = tm.minute as u16;
}
/// TIME_ZONE_INFORMATION (172 bytes) from the host zone database, the transitions
/// found by scanning this year.
#[repr(C)]
pub struct TimeZone {
    bias: i32,
    standard_name: [u16; 32],
    standard_date: [u16; 8],
    standard_bias: i32,
    daylight_name: [u16; 32],
    daylight_date: [u16; 8],
    daylight_bias: i32,
}
const _: () = assert!(size_of::<TimeZone>() == 172);
fn zone_name(index: usize) -> [u16; 32] {
    let mut out = [0u16; 32];
    let name = unsafe { tzname[index] };
    if !name.is_null() {
        let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
        for (o, u) in out.iter_mut().take(31).zip(name.encode_utf16()) {
            *o = u;
        }
    }
    out
}
fn current_zone() -> (TimeZone, u32) {
    unsafe { tzset() };
    let now = now(CLOCK_REALTIME).seconds;
    let today = local_tm(now);
    let mut january = Tm {
        year: today.year,
        day: 1,
        ..Tm::default()
    };
    let start = unsafe { timegm(&mut january) };
    let (mut standard, mut daylight) = (None, None);
    let (mut to_daylight, mut to_standard) = (0, 0);
    let first = local_tm(start);
    let mut was_dst = first.dst > 0;
    let mut previous = first.gmtoff;
    if was_dst {
        daylight = Some(previous);
    } else {
        standard = Some(previous);
    }
    let mut t = start + 3600;
    while t < start + 366 * 86400 {
        let probe = local_tm(t);
        if (probe.dst > 0) != was_dst {
            let (mut lo, mut hi) = (t - 3600, t);
            while hi - lo > 60 {
                let mid = lo + (hi - lo) / 2;
                if (local_tm(mid).dst > 0) == was_dst {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            if !was_dst {
                to_daylight = hi;
                standard = Some(previous);
                daylight = Some(probe.gmtoff);
            } else {
                to_standard = hi;
                daylight = Some(previous);
                standard = Some(probe.gmtoff);
            }
            was_dst = probe.dst > 0;
        }
        previous = probe.gmtoff;
        t += 3600;
    }
    let standard = standard.or(daylight).unwrap_or(0);
    let mut zone = TimeZone {
        bias: (-standard / 60) as i32,
        standard_name: zone_name(0),
        standard_date: [0; 8],
        standard_bias: 0,
        daylight_name: zone_name(1),
        daylight_date: [0; 8],
        daylight_bias: 0,
    };
    if to_daylight == 0 || to_standard == 0 {
        return (zone, 0); // TIME_ZONE_ID_UNKNOWN: no daylight saving time
    }
    let daylight = daylight.unwrap_or(standard);
    transition_rule(to_standard, daylight, &mut zone.standard_date);
    transition_rule(to_daylight, standard, &mut zone.daylight_date);
    zone.daylight_bias = (-(daylight - standard) / 60) as i32;
    (zone, if today.dst > 0 { 2 } else { 1 })
}
pub unsafe extern "win64" fn time_zone_information(out: *mut TimeZone) -> u32 {
    if out.is_null() {
        error::set(87);
        return u32::MAX;
    }
    let (zone, id) = current_zone();
    unsafe { out.write(zone) };
    id
}
/// GetDynamicTimeZoneInformation: TIME_ZONE_INFORMATION, TimeZoneKeyName[128] (the
/// standard name: there is no IANA-to-Windows table) and DynamicDaylightTimeDisabled.
pub unsafe extern "win64" fn dynamic_time_zone_information(out: *mut u8) -> u32 {
    if out.is_null() {
        error::set(87);
        return u32::MAX;
    }
    let (zone, id) = current_zone();
    unsafe {
        out.write_bytes(0, 432);
        std::ptr::copy_nonoverlapping(zone.standard_name.as_ptr().cast::<u8>(), out.add(172), 64);
        out.cast::<TimeZone>().write_unaligned(zone);
    }
    id
}
/// SystemTimeToTzSpecificLocalTime with the current zone (NULL); an explicit zone
/// is not supported.
pub unsafe extern "win64" fn system_time_to_tz_local(
    zone: *const TimeZone,
    utc: *const u16,
    local: *mut u16,
) -> i32 {
    if !zone.is_null() {
        eprintln!("SystemTimeToTzSpecificLocalTime: only the current time zone is supported");
        error::set(50);
        return 0;
    }
    let mut ft = 0u64;
    if utc.is_null() || local.is_null() || unsafe { system_time_to_file_time(utc, &mut ft) } == 0 {
        error::set(87);
        return 0;
    }
    let tm = local_tm((ft / 10_000_000) as i64 - EPOCH_1601);
    unsafe { store(&tm, *utc.add(7), local) };
    1
}
pub unsafe extern "win64" fn tz_local_to_system_time(
    zone: *const TimeZone,
    local: *const u16,
    utc: *mut u16,
) -> i32 {
    if !zone.is_null() {
        eprintln!("TzSpecificLocalTimeToSystemTime: only the current time zone is supported");
        error::set(50);
        return 0;
    }
    let mut ft = 0u64;
    if local.is_null() || utc.is_null() || unsafe { system_time_to_file_time(local, &mut ft) } == 0
    {
        error::set(87);
        return 0;
    }
    let l = unsafe { std::slice::from_raw_parts(local, 8) };
    let mut tm = Tm {
        year: l[0] as i32 - 1900,
        month: l[1] as i32 - 1,
        day: l[3] as i32,
        hour: l[4] as i32,
        minute: l[5] as i32,
        second: l[6] as i32,
        dst: -1,
        ..Tm::default()
    };
    let seconds = unsafe { mktime(&mut tm) };
    let out = ((seconds + EPOCH_1601) as u64) * 10_000_000 + l[7] as u64 * 10_000;
    unsafe { file_time_to_system_time(&out, utc) }
}
pub unsafe extern "win64" fn compare_file_time(a: *const u64, b: *const u64) -> i32 {
    match unsafe { (*a).cmp(&*b) } {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
/// FileTimeToLocalFileTime: the zone offset in effect at that time.
pub unsafe extern "win64" fn file_time_to_local(utc: *const u64, local: *mut u64) -> i32 {
    if utc.is_null() || local.is_null() {
        error::set(87);
        return 0;
    }
    let offset = local_tm((unsafe { *utc } / 10_000_000) as i64 - EPOCH_1601).gmtoff;
    unsafe { *local = (*utc as i64 + offset * 10_000_000) as u64 };
    1
}
pub unsafe extern "win64" fn local_to_file_time(local: *const u64, utc: *mut u64) -> i32 {
    if utc.is_null() || local.is_null() {
        error::set(87);
        return 0;
    }
    let guess = (unsafe { *local } / 10_000_000) as i64 - EPOCH_1601;
    let offset = local_tm(guess - local_tm(guess).gmtoff).gmtoff;
    unsafe { *utc = (*local as i64 - offset * 10_000_000) as u64 };
    1
}
/// SYSTEMTIME fields of the current local time.
pub(crate) fn local_now() -> [u16; 8] {
    let mut out = [0u16; 8];
    fill(out.as_mut_ptr(), true);
    out
}
