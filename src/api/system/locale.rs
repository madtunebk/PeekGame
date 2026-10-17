//! The user's locale, code pages and string classification (ported from peek where
//! peek has them). The locale comes from LC_ALL, LC_MESSAGES, then LANG; C/POSIX is
//! en-US. Only the user's own locale exists. Unmapped GetLocaleInfo types stop with
//! their number.
use crate::api::{system::error, text};
mod metadata;
pub(crate) mod thread;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
const ERROR_INVALID_FLAGS: u32 = 1004;
const LOCALE_USER_DEFAULT: u32 = 0x400;
const LOCALE_SYSTEM_DEFAULT: u32 = 0x800;

fn user_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|n| std::env::var(n).ok())
        .find(|v| !v.is_empty())
        .filter(|v| v != "C" && !v.starts_with("C.") && v != "POSIX")
}
/// ("ro", "RO") from "ro_RO.UTF-8"; ("en", "US") for C/POSIX.
fn parts() -> (String, String) {
    let Some(l) = user_locale() else {
        return ("en".into(), "US".into());
    };
    let base = l.split(['.', '@']).next().unwrap_or("");
    match base.split_once('_') {
        Some((lang, country)) => (lang.into(), country.into()),
        None => (base.into(), String::new()),
    }
}
fn locale_name() -> String {
    match parts() {
        (lang, country) if country.is_empty() => lang,
        (lang, country) => format!("{lang}-{country}"),
    }
}
/// Languages PeekGame knows the LANGID of (peek's table), most specific first.
const TABLE: &[(&str, u16)] = &[
    ("en_US", 0x0409),
    ("en_GB", 0x0809),
    ("en_AU", 0x0c09),
    ("en_CA", 0x1009),
    ("en", 0x0409),
    ("pt_BR", 0x0416),
    ("pt_PT", 0x0816),
    ("pt", 0x0816),
    ("es_MX", 0x080a),
    ("es", 0x0c0a),
    ("zh_TW", 0x0404),
    ("zh_HK", 0x0c04),
    ("zh", 0x0804),
    ("fr_CA", 0x0c0c),
    ("fr", 0x040c),
    ("de", 0x0407),
    ("it", 0x0410),
    ("ro", 0x0418),
    ("ru", 0x0419),
    ("uk", 0x0422),
    ("pl", 0x0415),
    ("cs", 0x0405),
    ("sk", 0x041b),
    ("hu", 0x040e),
    ("bg", 0x0402),
    ("el", 0x0408),
    ("tr", 0x041f),
    ("nl", 0x0413),
    ("sv", 0x041d),
    ("da", 0x0406),
    ("nb", 0x0414),
    ("no", 0x0414),
    ("fi", 0x040b),
    ("ja", 0x0411),
    ("ko", 0x0412),
    ("ar", 0x0401),
    ("he", 0x040d),
    ("hr", 0x041a),
    ("sr", 0x081a),
    ("sl", 0x0424),
    ("lt", 0x0427),
    ("lv", 0x0426),
    ("et", 0x0425),
    ("vi", 0x042a),
    ("th", 0x041e),
    ("id", 0x0421),
    ("ca", 0x0403),
    ("hi", 0x0439),
];
/// LANGID of the user's language; unknown languages are en-US.
pub(crate) fn langid() -> u16 {
    let Some(l) = user_locale() else {
        return 0x0409;
    };
    TABLE
        .iter()
        .find(|(tag, _)| {
            l.strip_prefix(tag)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(['_', '.', '@']))
        })
        .map_or(0x0409, |(_, id)| *id)
}
fn info_text(kind: u32) -> String {
    let (lang, country) = parts();
    match kind & 0x0fff_ffff {
        0x0e..=0x10
        | 0x14..=0x57
        | 0x78
        | 0x79
        | 0x1003
        | 0x1005
        | 0x1006
        | 0x1007
        | 0x1008
        | 0x1009
        | 0x100c..=0x100f => {
            crate::host::nls::number(&locale_name(), kind & 0x0fff_ffff).unwrap_or_else(|| {
                eprintln!(
                    "STOP: GetLocaleInfo: ICU cannot supply locale currency metadata {kind:#x}"
                );
                std::process::exit(78)
            })
        }
        0x1004 => metadata::ansi_code_page().to_string(),
        0x1001 => metadata::english_language().into(),
        0x1002 => match metadata::english_country(&country) {
            Some(name) => name.into(),
            None => {
                eprintln!("STOP: GetLocaleInfo: English name for country {country} is not mapped");
                std::process::exit(78)
            }
        },
        0x5c => locale_name(),
        0x59 => lang,
        0x5a => country,
        0x0d => ((["US", "LR", "MM"].contains(&country.as_str())) as u8).to_string(),
        other => {
            eprintln!("STOP: GetLocaleInfo: information type {other:#x} is not mapped");
            std::process::exit(78)
        }
    }
}
unsafe fn info_out(kind: u32, out: *mut u16, capacity: i32) -> i32 {
    if capacity < 0 || (capacity > 0 && out.is_null()) {
        error::set(87);
        return 0;
    }
    let value = info_text(kind);
    if kind & 0x2000_0000 != 0 {
        // LOCALE_RETURN_NUMBER: a DWORD in two WCHAR slots.
        let Ok(number) = value.parse::<u32>() else {
            error::set(ERROR_INVALID_FLAGS);
            return 0;
        };
        if capacity == 0 {
            return 2;
        }
        if capacity < 2 {
            error::set(ERROR_INSUFFICIENT_BUFFER);
            return 0;
        }
        unsafe { out.cast::<u32>().write_unaligned(number) };
        return 2;
    }
    let value = text::wide(&value);
    let needed = value.len() as i32 + 1;
    if capacity == 0 {
        return needed;
    }
    if out.is_null() || capacity < needed {
        error::set(ERROR_INSUFFICIENT_BUFFER);
        return 0;
    }
    unsafe { text::copy_out(&value, out, capacity as u32) };
    needed
}
fn own_lcid(lcid: u32) -> bool {
    lcid == LOCALE_USER_DEFAULT || lcid == LOCALE_SYSTEM_DEFAULT || lcid == langid() as u32
}
/// LOCALE_NAME_USER_DEFAULT (NULL), "" or the user's own name.
unsafe fn own_name(name: *const u16) -> bool {
    let name = unsafe { text::units(name) };
    name.is_empty() || String::from_utf16_lossy(name).eq_ignore_ascii_case(&locale_name())
}
pub unsafe extern "win64" fn info_w(lcid: u32, kind: u32, out: *mut u16, capacity: i32) -> i32 {
    if !own_lcid(lcid) {
        eprintln!("GetLocaleInfoW({lcid:#x}): only the user's locale is available");
        error::set(87);
        return 0;
    }
    unsafe { info_out(kind, out, capacity) }
}
pub unsafe extern "win64" fn info_ex(
    name: *const u16,
    kind: u32,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if !unsafe { own_name(name) } {
        eprintln!("GetLocaleInfoEx: only the user's locale is available");
        error::set(87);
        return 0;
    }
    unsafe { info_out(kind, out, capacity) }
}
pub extern "win64" fn user_default_lcid() -> u32 {
    langid() as u32
}
/// GetUserDefaultLocaleName / GetSystemDefaultLocaleName: length with the NUL.
pub unsafe extern "win64" fn user_default_locale_name(out: *mut u16, capacity: i32) -> i32 {
    let name = text::wide(&locale_name());
    if out.is_null() || capacity < name.len() as i32 + 1 {
        error::set(ERROR_INSUFFICIENT_BUFFER);
        return 0;
    }
    unsafe { text::copy_out(&name, out, capacity as u32) };
    name.len() as i32 + 1
}
pub extern "win64" fn is_valid_locale(lcid: u32, _flags: u32) -> i32 {
    (own_lcid(lcid) || lcid == 0x0409) as i32
}
type EnumLocales = extern "win64" fn(*const u16) -> i32;
/// EnumSystemLocalesW: the user's locale, as an 8-digit hex LCID string.
pub unsafe extern "win64" fn enum_system_locales_w(
    callback: Option<EnumLocales>,
    _flags: u32,
) -> i32 {
    let Some(callback) = callback else {
        error::set(87);
        return 0;
    };
    let mut id = text::wide(&format!("{:08x}", langid()));
    id.push(0);
    callback(id.as_ptr());
    1
}
pub extern "win64" fn is_valid_code_page(cp: u32) -> i32 {
    (cp == 65001 || cp == 1252) as i32
}
/// CPINFO: MaxCharSize, DefaultChar[2], LeadByte[12].
pub unsafe extern "win64" fn cp_info(cp: u32, out: *mut u8) -> i32 {
    let cp = if cp == 0 { 65001 } else { cp };
    if is_valid_code_page(cp) == 0 || out.is_null() {
        error::set(87);
        return 0;
    }
    unsafe {
        out.write_bytes(0, 20);
        out.cast::<u32>()
            .write_unaligned(if cp == 65001 { 4 } else { 1 });
        *out.add(4) = b'?';
    }
    1
}
pub extern "win64" fn console_output_cp() -> u32 {
    65001
}

/// GetStringTypeW, CT_CTYPE1 only, one WORD per UTF-16 unit (-1: through the NUL).
/// Surrogate halves are not classified.
pub unsafe extern "win64" fn string_type_w(
    kind: u32,
    source: *const u16,
    count: i32,
    out: *mut u16,
) -> i32 {
    if kind != 1 {
        eprintln!("GetStringTypeW(type {kind}): only CT_CTYPE1 is supported");
        error::set(ERROR_INVALID_FLAGS);
        return 0;
    }
    let Some(units) =
        (unsafe { text::counted(source, count, true) }).filter(|_| count != 0 && !out.is_null())
    else {
        error::set(87);
        return 0;
    };
    for (i, &u) in units.iter().enumerate() {
        let bits = char::from_u32(u as u32).map_or(0, |c| {
            let mut b = 0u16;
            b |= c.is_uppercase() as u16;
            b |= (c.is_lowercase() as u16) << 1;
            b |= (c.is_ascii_digit() as u16) << 2;
            b |= (c.is_whitespace() as u16) << 3;
            b |= ((c.is_ascii_punctuation()
                || (!c.is_alphanumeric()
                    && !c.is_whitespace()
                    && !c.is_control()
                    && c as u32 > 0x7f)) as u16)
                << 4;
            b |= (c.is_control() as u16) << 5;
            b |= ((c == ' ' || c == '\t') as u16) << 6;
            b |= (c.is_ascii_hexdigit() as u16) << 7;
            b |= (c.is_alphabetic() as u16) << 8;
            if b & !0x20 != 0 || !c.is_control() {
                b |= 0x200; // C1_DEFINED; bare controls are C1_CNTRL only
            }
            b
        });
        unsafe { *out.add(i) = bits };
    }
    1
}

fn map_case(units: &[u16], upper: bool) -> Vec<u16> {
    units
        .iter()
        .map(|&u| {
            char::from_u32(u as u32)
                .and_then(|c| {
                    let mut mapped = if upper {
                        c.to_uppercase().collect::<Vec<_>>()
                    } else {
                        c.to_lowercase().collect()
                    };
                    // Only one-to-one mappings within the BMP keep the length.
                    (mapped.len() == 1 && (mapped[0] as u32) < 0x10000)
                        .then(|| mapped.pop().unwrap() as u16)
                })
                .unwrap_or(u)
        })
        .collect()
}
const LCMAP_LOWERCASE: u32 = 0x100;
const LCMAP_UPPERCASE: u32 = 0x200;
/// LCMapStringW: LCMAP_LOWERCASE / LCMAP_UPPERCASE (with or without
/// LCMAP_LINGUISTIC_CASING); sort keys and other mappings fail with
/// ERROR_INVALID_FLAGS.
unsafe fn lcmap(flags: u32, source: *const u16, count: i32, out: *mut u16, capacity: i32) -> i32 {
    let case = flags & !0x0100_0000;
    if case != LCMAP_LOWERCASE && case != LCMAP_UPPERCASE {
        eprintln!("LCMapString(flags {flags:#x}): only case mapping is supported");
        error::set(ERROR_INVALID_FLAGS);
        return 0;
    }
    let Some(units) = (unsafe { text::counted(source, count, true) }).filter(|_| count != 0) else {
        error::set(87);
        return 0;
    };
    let mapped = map_case(units, case == LCMAP_UPPERCASE);
    if capacity == 0 {
        return mapped.len() as i32;
    }
    if out.is_null() || capacity < mapped.len() as i32 {
        error::set(ERROR_INSUFFICIENT_BUFFER);
        return 0;
    }
    unsafe { std::ptr::copy_nonoverlapping(mapped.as_ptr(), out, mapped.len()) };
    mapped.len() as i32
}
pub unsafe extern "win64" fn lcmap_w(
    lcid: u32,
    flags: u32,
    source: *const u16,
    count: i32,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    if !own_lcid(lcid) && lcid != 0x0409 && lcid != 0x7f {
        error::set(87);
        return 0;
    }
    unsafe { lcmap(flags, source, count, out, capacity) }
}
pub unsafe extern "win64" fn lcmap_ex(
    _name: *const u16,
    flags: u32,
    source: *const u16,
    count: i32,
    out: *mut u16,
    capacity: i32,
    _version: usize,
    _reserved: usize,
    _handle: usize,
) -> i32 {
    unsafe { lcmap(flags, source, count, out, capacity) }
}
unsafe fn narrow_bytes<'a>(source: *const u8, count: i32) -> Option<&'a [u8]> {
    if source.is_null() || count == 0 {
        return None;
    }
    if count > 0 {
        Some(unsafe { std::slice::from_raw_parts(source, count as usize) })
    } else {
        let len = unsafe { std::ffi::CStr::from_ptr(source.cast()) }
            .to_bytes_with_nul()
            .len();
        Some(unsafe { std::slice::from_raw_parts(source, len) })
    }
}
pub unsafe extern "win64" fn info_a(lcid: u32, kind: u32, out: *mut u8, capacity: i32) -> i32 {
    if capacity < 0 || (capacity > 0 && out.is_null()) {
        error::set(87);
        return 0;
    }
    if !own_lcid(lcid) && lcid != 0x0409 && lcid != 0x7f {
        error::set(87);
        return 0;
    }
    let value = info_text(kind);
    if kind & 0x2000_0000 != 0 {
        let Ok(number) = value.parse::<u32>() else {
            error::set(ERROR_INVALID_FLAGS);
            return 0;
        };
        if capacity == 0 {
            return 4;
        }
        if capacity < 4 {
            error::set(ERROR_INSUFFICIENT_BUFFER);
            return 0;
        }
        unsafe { out.cast::<u32>().write_unaligned(number) };
        return 4;
    }
    let bytes = value.as_bytes();
    let needed = bytes.len() as i32 + 1;
    if capacity == 0 {
        return needed;
    }
    if out.is_null() || capacity < needed {
        error::set(ERROR_INSUFFICIENT_BUFFER);
        return 0;
    }
    unsafe { text::copy_out_narrow(bytes, out, capacity as u32) };
    needed
}
pub unsafe extern "win64" fn string_type_a(
    _lcid: u32,
    kind: u32,
    source: *const u8,
    count: i32,
    out: *mut u16,
) -> i32 {
    let Some(bytes) = (unsafe { narrow_bytes(source, count) }) else {
        error::set(87);
        return 0;
    };
    let wide: Vec<u16> = bytes.iter().map(|&b| b as u16).collect();
    unsafe { string_type_w(kind, wide.as_ptr(), wide.len() as i32, out) }
}
pub unsafe extern "win64" fn lcmap_a(
    lcid: u32,
    flags: u32,
    source: *const u8,
    count: i32,
    out: *mut u8,
    capacity: i32,
) -> i32 {
    let Some(bytes) = (unsafe { narrow_bytes(source, count) }) else {
        error::set(87);
        return 0;
    };
    let wide: Vec<u16> = bytes.iter().map(|&b| b as u16).collect();
    let mut mapped = vec![0u16; wide.len()];
    let n = unsafe {
        lcmap_w(
            lcid,
            flags,
            wide.as_ptr(),
            wide.len() as i32,
            mapped.as_mut_ptr(),
            mapped.len() as i32,
        )
    };
    if n <= 0 {
        return 0;
    }
    if capacity == 0 {
        return n;
    }
    if out.is_null() || capacity < n {
        error::set(ERROR_INSUFFICIENT_BUFFER);
        return 0;
    }
    for (i, &u) in mapped[..n as usize].iter().enumerate() {
        unsafe { *out.add(i) = u as u8 };
    }
    n
}
const NORM_IGNORECASE: u32 = 0x1;
/// CompareString: ordinal UTF-16 order, case-insensitive with NORM_IGNORECASE
/// (or LINGUISTIC_IGNORECASE); returns CSTR_LESS_THAN (1), CSTR_EQUAL (2) or
/// CSTR_GREATER_THAN (3).
unsafe fn compare(flags: u32, a: *const u16, a_count: i32, b: *const u16, b_count: i32) -> i32 {
    let (Some(a), Some(b)) = (unsafe { text::counted(a, a_count, false) }, unsafe {
        text::counted(b, b_count, false)
    }) else {
        error::set(87);
        return 0;
    };
    let ignore_case = flags & (NORM_IGNORECASE | 0x10) != 0;
    let (a, b) = if ignore_case {
        (map_case(a, false), map_case(b, false))
    } else {
        (a.to_vec(), b.to_vec())
    };
    match a.cmp(&b) {
        std::cmp::Ordering::Less => 1,
        std::cmp::Ordering::Equal => 2,
        std::cmp::Ordering::Greater => 3,
    }
}
pub unsafe extern "win64" fn compare_w(
    _lcid: u32,
    flags: u32,
    a: *const u16,
    a_count: i32,
    b: *const u16,
    b_count: i32,
) -> i32 {
    unsafe { compare(flags, a, a_count, b, b_count) }
}
pub unsafe extern "win64" fn compare_ex(
    _name: *const u16,
    flags: u32,
    a: *const u16,
    a_count: i32,
    b: *const u16,
    b_count: i32,
    _version: usize,
    _reserved: usize,
    _param: usize,
) -> i32 {
    unsafe { compare(flags, a, a_count, b, b_count) }
}

/// The LANGID a "ll-CC" or "ll" locale name stands for.
fn lcid_of(name: &str) -> Option<u32> {
    if name.is_empty() || name.eq_ignore_ascii_case(&locale_name()) {
        return Some(langid() as u32);
    }
    let tag = name.replace('-', "_");
    TABLE
        .iter()
        .find(|(t, _)| t.eq_ignore_ascii_case(&tag))
        .map(|(_, id)| *id as u32)
}
/// The locale name of `lcid`: the user's own, or a "ll-CC" entry of the table.
fn name_of(lcid: u32) -> Option<String> {
    if own_lcid(lcid) {
        return Some(locale_name());
    }
    TABLE
        .iter()
        .find(|(t, id)| *id as u32 == lcid && t.contains('_'))
        .map(|(t, _)| t.replace('_', "-"))
}
pub unsafe extern "win64" fn locale_name_to_lcid(name: *const u16, _flags: u32) -> u32 {
    let name = String::from_utf16_lossy(unsafe { text::units(name) });
    lcid_of(&name).unwrap_or_else(|| {
        error::set(87);
        0
    })
}
/// LCIDToLocaleName: length with the NUL; 0 asks for the length.
pub unsafe extern "win64" fn lcid_to_locale_name(
    lcid: u32,
    out: *mut u16,
    capacity: i32,
    _flags: u32,
) -> i32 {
    let Some(name) = name_of(lcid) else {
        error::set(87);
        return 0;
    };
    let name = text::wide(&name);
    let needed = name.len() as i32 + 1;
    if capacity == 0 {
        return needed;
    }
    if out.is_null() || capacity < needed {
        error::set(ERROR_INSUFFICIENT_BUFFER);
        return 0;
    }
    unsafe { text::copy_out(&name, out, capacity as u32) };
    needed
}
pub unsafe extern "win64" fn is_valid_locale_name(name: *const u16) -> i32 {
    let name = String::from_utf16_lossy(unsafe { text::units(name) });
    (!name.is_empty() && lcid_of(&name).is_some()) as i32
}
type EnumLocalesEx = extern "win64" fn(*const u16, u32, isize) -> i32;
/// EnumSystemLocalesEx: the user's locale (LOCALE_SPECIFICDATA when it names a
/// country, else LOCALE_NEUTRALDATA).
pub unsafe extern "win64" fn enum_system_locales_ex(
    callback: Option<EnumLocalesEx>,
    _flags: u32,
    parameter: isize,
    _reserved: usize,
) -> i32 {
    let Some(callback) = callback else {
        error::set(87);
        return 0;
    };
    let name = locale_name();
    let kind = if name.contains('-') { 2 } else { 0x10 };
    let mut wide = text::wide(&name);
    wide.push(0);
    callback(wide.as_ptr(), kind, parameter);
    1
}

/// CPINFOEXW uses Windows WCHARs: UnicodeDefaultChar at 18, CodePage at 20,
/// CodePageName[260] at 24. ACP/OEM/thread ACP select this runtime's UTF-8 page.
pub unsafe extern "win64" fn cp_info_ex_w(cp: u32, flags: u32, out: *mut u8) -> i32 {
    let cp = if matches!(cp, 0 | 1 | 3) { 65001 } else { cp };
    if flags != 0 || out.is_null() || is_valid_code_page(cp) == 0 {
        error::set(87);
        return 0;
    }
    unsafe {
        out.write_bytes(0, 544);
        cp_info(cp, out);
        out.add(18)
            .cast::<u16>()
            .write_unaligned(if cp == 65001 { 0xfffd } else { b'?' as u16 });
        out.add(20).cast::<u32>().write_unaligned(cp);
        let name = if cp == 65001 {
            "Unicode (UTF-8)"
        } else {
            "1252 (ANSI - Latin I)"
        };
        for (index, unit) in name.encode_utf16().enumerate() {
            out.add(24 + index * 2).cast::<u16>().write_unaligned(unit);
        }
    }
    1
}

/// UI language uses the same host-locale mapping as the existing locale APIs.
pub extern "win64" fn user_default_ui_language() -> u16 {
    langid()
}

pub unsafe extern "win64" fn preferred_ui_languages(
    flags: u32,
    count: *mut u32,
    buffer: *mut u16,
    size: *mut u32,
) -> i32 {
    if (flags & !0x0c) != 0 || (flags & 0x0c) == 0x0c || count.is_null() || size.is_null() {
        error::set(87);
        return 0;
    }
    let tag = if flags & 4 != 0 {
        format!("{:04X}", langid())
    } else {
        let env = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_else(|_| "en_US".into());
        let clean = env.split('.').next().unwrap_or("en-US").replace('_', "-");
        if clean.is_empty() { "en-US".into() } else { clean }
    };
    let utf16: Vec<u16> = tag.encode_utf16().chain(std::iter::once(0)).chain(std::iter::once(0)).collect();
    let need = utf16.len() as u32;
    unsafe { *count = 1 };
    if buffer.is_null() {
        unsafe { *size = need };
        return 1;
    }
    let cur_size = unsafe { *size };
    if cur_size < need {
        unsafe { *size = need };
        error::set(122); // ERROR_INSUFFICIENT_BUFFER
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), buffer, utf16.len());
        *size = need;
    }
    1
}

/// GetThreadPreferredUILanguages: no thread-specific list has been installed,
/// so merged queries use the existing user UI language and thread-only queries
/// return an empty multi-string.
pub unsafe extern "win64" fn thread_preferred_ui_languages(
    flags: u32,
    count: *mut u32,
    buffer: *mut u16,
    size: *mut u32,
) -> i32 {
    const MUI_LANGUAGE_ID: u32 = 0x04;
    const MUI_LANGUAGE_NAME: u32 = 0x08;
    const MUI_MERGE_SYSTEM_FALLBACK: u32 = 0x10;
    const MUI_MERGE_USER_FALLBACK: u32 = 0x20;
    const MUI_THREAD_LANGUAGES: u32 = 0x40;
    if count.is_null()
        || size.is_null()
        || flags & !0x7c != 0
        || flags & (MUI_LANGUAGE_ID | MUI_LANGUAGE_NAME) == (MUI_LANGUAGE_ID | MUI_LANGUAGE_NAME)
        || (flags & MUI_THREAD_LANGUAGES != 0
            && flags & (MUI_MERGE_SYSTEM_FALLBACK | MUI_MERGE_USER_FALLBACK) != 0)
        || (flags & MUI_MERGE_SYSTEM_FALLBACK != 0
            && flags & MUI_MERGE_USER_FALLBACK == 0)
    {
        error::set(87); // ERROR_INVALID_PARAMETER
        return 0;
    }
    if flags & MUI_THREAD_LANGUAGES != 0 {
        let available = unsafe { *size };
        unsafe {
            *count = 0;
            *size = 2; // empty double-null-terminated multi-string
        }
        if buffer.is_null() {
            return 1;
        }
        if available < 2 {
            error::set(122); // ERROR_INSUFFICIENT_BUFFER
            return 0;
        }
        unsafe {
            *buffer = 0;
            *buffer.add(1) = 0;
        }
        return 1;
    }
    unsafe { preferred_ui_languages(flags & (MUI_LANGUAGE_ID | MUI_LANGUAGE_NAME), count, buffer, size) }
}
