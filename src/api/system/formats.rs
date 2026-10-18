//! GetDateFormat and GetTimeFormat: Windows format pictures (d dd ddd dddd, M MM MMM
//! MMMM, y yy yyyy, g gg, h hh H HH, m mm, s ss, t tt, 'quoted text') over a
//! SYSTEMTIME. Names are English and the default pictures en-US's, whatever the
//! user's locale: PeekGame has no per-locale calendar data.
use super::{error, locale, time};
use crate::api::text;
const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DATE_SHORTDATE: u32 = 1;
const DATE_LONGDATE: u32 = 2;
const DATE_YEARMONTH: u32 = 8;
const TIME_NOMINUTESORSECONDS: u32 = 1;
const TIME_NOSECONDS: u32 = 2;
const TIME_NOTIMEMARKER: u32 = 4;
const TIME_FORCE24HOURFORMAT: u32 = 8;

/// Expands `picture`. `date` selects which letters are fields (the others copy).
fn expand(picture: &[u16], st: &[u16; 8], date: bool, flags: u32) -> String {
    let picture = String::from_utf16_lossy(picture);
    let chars: Vec<char> = picture.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        out.push('\'');
                        i += 2;
                        continue;
                    }
                    break;
                }
                out.push(chars[i]);
                i += 1;
            }
            i += 1;
            continue;
        }
        let mut n = 1;
        while chars.get(i + n) == Some(&c) {
            n += 1;
        }
        let (year, month, weekday, day, hour, minute, second) =
            (st[0], st[1], st[2], st[3], st[4], st[5], st[6]);
        let field = match (date, c) {
            (true, 'd') => Some(match n {
                1 => day.to_string(),
                2 => format!("{day:02}"),
                3 => DAYS[weekday as usize % 7][..3].to_string(),
                _ => DAYS[weekday as usize % 7].to_string(),
            }),
            (true, 'M') => Some(match n {
                1 => month.to_string(),
                2 => format!("{month:02}"),
                3 => MONTHS[(month as usize + 11) % 12][..3].to_string(),
                _ => MONTHS[(month as usize + 11) % 12].to_string(),
            }),
            (true, 'y') => Some(match n {
                1 => (year % 100).to_string(),
                2 => format!("{:02}", year % 100),
                _ => year.to_string(),
            }),
            (true, 'g') => Some("A.D.".to_string()),
            (false, 'h') | (false, 'H') => {
                let h = if c == 'h' && flags & TIME_FORCE24HOURFORMAT == 0 {
                    (hour + 11) % 12 + 1
                } else {
                    hour
                };
                Some(if n >= 2 {
                    format!("{h:02}")
                } else {
                    h.to_string()
                })
            }
            (false, 'm') if flags & TIME_NOMINUTESORSECONDS == 0 => Some(if n >= 2 {
                format!("{minute:02}")
            } else {
                minute.to_string()
            }),
            (false, 's') if flags & (TIME_NOMINUTESORSECONDS | TIME_NOSECONDS) == 0 => {
                Some(if n >= 2 {
                    format!("{second:02}")
                } else {
                    second.to_string()
                })
            }
            (false, 'm') | (false, 's') => {
                // Dropped with the separator before it.
                while out.ends_with([':', '.', ' ']) {
                    out.pop();
                }
                Some(String::new())
            }
            (false, 't') if flags & TIME_NOTIMEMARKER == 0 => {
                let marker = if hour < 12 { "AM" } else { "PM" };
                Some(if n >= 2 {
                    marker.to_string()
                } else {
                    marker[..1].to_string()
                })
            }
            (false, 't') => {
                while out.ends_with(' ') {
                    out.pop();
                }
                Some(String::new())
            }
            _ => None,
        };
        match field {
            Some(f) => out.push_str(&f),
            None => (0..n).for_each(|_| out.push(c)),
        }
        i += n;
    }
    out
}
unsafe fn system_time(st: *const u16) -> Option<[u16; 8]> {
    if st.is_null() {
        return Some(time::local_now());
    }
    let mut value = [0u16; 8];
    unsafe { std::ptr::copy_nonoverlapping(st, value.as_mut_ptr(), 8) };
    let mut ft = 0u64;
    (unsafe { time::system_time_to_file_time(value.as_ptr(), &mut ft) } != 0).then_some(value)
}
unsafe fn output(value: &str, out: *mut u16, capacity: i32) -> i32 {
    let value = text::wide(value);
    let needed = value.len() as i32 + 1;
    if capacity == 0 {
        return needed;
    }
    if out.is_null() || capacity < needed {
        error::set(122);
        return 0;
    }
    unsafe { text::copy_out(&value, out, capacity as u32) };
    needed
}
unsafe fn date(
    flags: u32,
    st: *const u16,
    picture: *const u16,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    let Some(st) = (unsafe { system_time(st) }) else {
        error::set(87);
        return 0;
    };
    let default = if flags & DATE_LONGDATE != 0 {
        "dddd, MMMM d, yyyy"
    } else if flags & DATE_YEARMONTH != 0 {
        "MMMM yyyy"
    } else {
        let _ = DATE_SHORTDATE;
        "M/d/yyyy"
    };
    let picture = if picture.is_null() {
        text::wide(default)
    } else {
        unsafe { text::units(picture) }.to_vec()
    };
    unsafe { output(&expand(&picture, &st, true, flags), out, capacity) }
}
unsafe fn time_of_day(
    flags: u32,
    st: *const u16,
    picture: *const u16,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    let Some(st) = (unsafe { system_time(st) }) else {
        error::set(87);
        return 0;
    };
    let picture = if picture.is_null() {
        text::wide("h:mm:ss tt")
    } else {
        unsafe { text::units(picture) }.to_vec()
    };
    unsafe { output(&expand(&picture, &st, false, flags), out, capacity) }
}
fn locale_ok(lcid: u32) -> bool {
    lcid == 0x400
        || lcid == 0x800
        || lcid == 0
        || lcid == locale::langid() as u32
        || lcid == 0x409
        || lcid == 0x7f
}
unsafe fn name_ok(name: *const u16) -> bool {
    let name = String::from_utf16_lossy(unsafe { text::units(name) });
    name.is_empty()
        || name == "!x-sys-default-locale"
        || unsafe { locale::is_valid_locale_name(text::wide(&format!("{name}\0")).as_ptr()) } != 0
}
pub unsafe extern "win64" fn date_format_w(
    lcid: u32,
    flags: u32,
    st: *const u16,
    picture: *const u16,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if !locale_ok(lcid) {
        error::set(87);
        return 0;
    }
    unsafe { date(flags, st, picture, out, capacity) }
}
pub unsafe extern "win64" fn date_format_ex(
    name: *const u16,
    flags: u32,
    st: *const u16,
    picture: *const u16,
    out: *mut u16,
    capacity: i32,
    _calendar: *const u16,
) -> i32 {
    if !unsafe { name_ok(name) } {
        error::set(87);
        return 0;
    }
    unsafe { date(flags, st, picture, out, capacity) }
}
pub unsafe extern "win64" fn time_format_w(
    lcid: u32,
    flags: u32,
    st: *const u16,
    picture: *const u16,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if !locale_ok(lcid) {
        error::set(87);
        return 0;
    }
    unsafe { time_of_day(flags, st, picture, out, capacity) }
}
pub unsafe extern "win64" fn time_format_ex(
    name: *const u16,
    flags: u32,
    st: *const u16,
    picture: *const u16,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if !unsafe { name_ok(name) } {
        error::set(87);
        return 0;
    }
    unsafe { time_of_day(flags, st, picture, out, capacity) }
}
pub unsafe extern "win64" fn number_format_ex(
    _name: *const u16,
    _flags: u32,
    value: *const u16,
    _format: *const u8,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if value.is_null() {
        error::set(87);
        return 0;
    }
    let s = String::from_utf16_lossy(unsafe { text::units(value) });
    unsafe { output(&s, out, capacity) }
}
pub unsafe extern "win64" fn currency_format_ex(
    _name: *const u16,
    _flags: u32,
    value: *const u16,
    _format: *const u8,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if value.is_null() {
        error::set(87);
        return 0;
    }
    let s = format!("${}", String::from_utf16_lossy(unsafe { text::units(value) }));
    unsafe { output(&s, out, capacity) }
}

