//! UCRT functions Microsoft's C++ standard library (msvcp140.dll) and runtime
//! (vcruntime140.dll) import: the C-locale tables and lock, string and number
//! conversions, the new handler, floating-point classification, and wide-path file
//! operations. Locale content is the C locale, as elsewhere in this CRT.
use super::stdio::errno;
use std::ffi::{c_char, c_void};
use std::sync::OnceLock;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn strtod(text: *const c_char, end: *mut *mut c_char) -> f64;
    fn strtof(text: *const c_char, end: *mut *mut c_char) -> f32;
    fn lgamma(x: f64) -> f64;
    fn lgammaf(x: f32) -> f32;
    fn pthread_mutexattr_init(a: *mut u64) -> i32;
    fn pthread_mutexattr_settype(a: *mut u64, kind: i32) -> i32;
    fn pthread_mutex_init(m: *mut u64, a: *const u64) -> i32;
    fn pthread_mutex_lock(m: *mut u64) -> i32;
    fn pthread_mutex_unlock(m: *mut u64) -> i32;
}

// ---- locale ---------------------------------------------------------------------

/// The global locale lock: recursive, as UCRT's (locale changes nest inside it).
fn locale_lock() -> *mut u64 {
    struct Lock(std::cell::UnsafeCell<[u64; 8]>);
    unsafe impl Sync for Lock {}
    unsafe impl Send for Lock {}
    static LOCK: OnceLock<Lock> = OnceLock::new();
    LOCK.get_or_init(|| {
        let lock = Lock(std::cell::UnsafeCell::new([0; 8]));
        let mut attr = 0u64;
        unsafe {
            pthread_mutexattr_init(&mut attr);
            pthread_mutexattr_settype(&mut attr, 1); // PTHREAD_MUTEX_RECURSIVE
            pthread_mutex_init(lock.0.get().cast(), &attr);
        }
        lock
    })
    .0
    .get()
    .cast()
}
pub extern "win64" fn lock_locales() {
    unsafe { pthread_mutex_lock(locale_lock()) };
}
pub extern "win64" fn unlock_locales() {
    unsafe { pthread_mutex_unlock(locale_lock()) };
}
/// ___lc_locale_name_func: the locale name per category (LC_ALL..LC_TIME); all NULL
/// for the C locale, which callers read as "C".
pub extern "win64" fn locale_names() -> *const *const u16 {
    static NAMES: [usize; 6] = [0; 6];
    NAMES.as_ptr().cast()
}
/// ___lc_collate_cp_func: the collation code page; the C locale's is CP_ACP (0).
pub extern "win64" fn collate_codepage() -> u32 {
    0
}
/// __pctype_func: the C locale's character classification table (_UPPER 1,
/// _LOWER 2, _DIGIT 4, _SPACE 8, _PUNCT 0x10, _CONTROL 0x20, _BLANK 0x40, _HEX 0x80,
/// _ALPHA 0x100), indexable from -1 (EOF) to 255; bytes above 127 are unclassified.
pub extern "win64" fn ctype_table() -> *const u16 {
    static TABLE: OnceLock<[u16; 257]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u16; 257];
        for c in 0u8..=127 {
            let mut v = 0u16;
            if c.is_ascii_uppercase() {
                v |= 0x1 | 0x100;
            }
            if c.is_ascii_lowercase() {
                v |= 0x2 | 0x100;
            }
            if c.is_ascii_digit() {
                v |= 0x4;
            }
            if c.is_ascii_hexdigit() {
                v |= 0x80;
            }
            if matches!(c, b' ' | b'\t'..=b'\r') {
                v |= 0x8;
            }
            if matches!(c, b' ' | b'\t') {
                v |= 0x40;
            }
            if c.is_ascii_punctuation() {
                v |= 0x10;
            }
            if c.is_ascii_control() {
                v |= 0x20;
            }
            t[c as usize + 1] = v;
        }
        t
    });
    table[1..].as_ptr()
}
/// _wsetlocale: the C locale only. A query (NULL), "C", "POSIX" and "" (the
/// environment's locale, which is C here) return L"C"; other locales fail.
pub unsafe extern "win64" fn wsetlocale(category: i32, locale: *const u16) -> *const u16 {
    static C: [u16; 2] = [b'C' as u16, 0];
    if !(0..=5).contains(&category) {
        return std::ptr::null();
    }
    if locale.is_null() {
        return C.as_ptr();
    }
    let name = unsafe { crate::api::text::string(locale) }.unwrap_or_default();
    if matches!(name.as_str(), "C" | "POSIX" | "") {
        C.as_ptr()
    } else {
        std::ptr::null()
    }
}

// ---- strings and conversions --------------------------------------------------------

pub extern "win64" fn is_upper(c: i32) -> i32 {
    (0x41..=0x5a).contains(&c) as i32
}
pub extern "win64" fn is_lower(c: i32) -> i32 {
    (0x61..=0x7a).contains(&c) as i32
}
/// __strncnt: strnlen.
pub unsafe extern "win64" fn strncnt(text: *const c_char, count: usize) -> usize {
    (0..count).position(|i| unsafe { *text.add(i) } == 0).unwrap_or(count)
}
pub unsafe extern "win64" fn wcsnlen(text: *const u16, count: usize) -> usize {
    if text.is_null() {
        return 0;
    }
    (0..count).position(|i| unsafe { *text.add(i) } == 0).unwrap_or(count)
}
/// _wcsdup: a copy in CRT (host malloc) memory, freed with free().
pub unsafe extern "win64" fn wcsdup(text: *const u16) -> *mut u16 {
    if text.is_null() {
        unsafe { *errno() = 22 };
        return std::ptr::null_mut();
    }
    let len = unsafe { wcsnlen(text, usize::MAX) } + 1;
    let copy = unsafe { malloc(len * 2) }.cast::<u16>();
    if copy.is_null() {
        unsafe { *errno() = 12 };
    } else {
        unsafe { std::ptr::copy_nonoverlapping(text, copy, len) };
    }
    copy
}
pub unsafe extern "win64" fn str_to_double(text: *const c_char, end: *mut *mut c_char) -> f64 {
    unsafe { strtod(text, end) }
}
pub unsafe extern "win64" fn str_to_float(text: *const c_char, end: *mut *mut c_char) -> f32 {
    unsafe { strtof(text, end) }
}
/// The ASCII prefix of a wide number, converted narrow; `end` maps back by count
/// (one byte per unit for every character a number can contain).
unsafe fn wide_number<T>(text: *const u16, end: *mut *mut u16, convert: impl FnOnce(*const c_char, *mut *mut c_char) -> T) -> T {
    let mut narrow = Vec::new();
    let mut i = 0;
    while let Some(&u) = unsafe { text.add(i).as_ref() } {
        if u == 0 || u > 0x7f {
            break;
        }
        narrow.push(u as u8);
        i += 1;
    }
    narrow.push(0);
    let mut stop: *mut c_char = std::ptr::null_mut();
    let value = convert(narrow.as_ptr().cast(), &mut stop);
    if !end.is_null() {
        let used = unsafe { stop.offset_from(narrow.as_ptr().cast()) } as usize;
        unsafe { *end = text.add(used) as *mut u16 };
    }
    value
}
pub unsafe extern "win64" fn wide_to_double(text: *const u16, end: *mut *mut u16) -> f64 {
    unsafe { wide_number(text, end, |t, e| strtod(t, e)) }
}
pub unsafe extern "win64" fn wide_to_float(text: *const u16, end: *mut *mut u16) -> f32 {
    unsafe { wide_number(text, end, |t, e| strtof(t, e)) }
}
/// atol: Windows long is 32 bits.
pub unsafe extern "win64" fn atol(text: *const c_char) -> i32 {
    let value = unsafe { super::numbers::strtol(text, std::ptr::null_mut(), 10) };
    value as i32
}

pub unsafe extern "win64" fn atol_l(text: *const c_char, _locale: usize) -> i32 {
    unsafe { atol(text) }
}

pub unsafe extern "win64" fn atoi_l(text: *const c_char, _locale: usize) -> i32 {
    let value = unsafe { super::numbers::strtol(text, std::ptr::null_mut(), 10) };
    value as i32
}

pub unsafe extern "win64" fn atof_l(text: *const c_char, _locale: usize) -> f64 {
    unsafe { strtod(text, std::ptr::null_mut()) }
}

pub unsafe extern "win64" fn atoi64(text: *const c_char) -> i64 {
    unsafe { super::numbers::strtoi64(text, std::ptr::null_mut(), 10) }
}

pub unsafe extern "win64" fn atoi64_l(text: *const c_char, _locale: usize) -> i64 {
    unsafe { atoi64(text) }
}

pub unsafe extern "win64" fn wtof(text: *const u16) -> f64 {
    if text.is_null() {
        return 0.0;
    }
    unsafe { wide_to_double(text, std::ptr::null_mut()) }
}

pub unsafe extern "win64" fn wtof_l(text: *const u16, _locale: usize) -> f64 {
    unsafe { wtof(text) }
}

pub unsafe extern "win64" fn wtoi(text: *const u16) -> i32 {
    if text.is_null() {
        return 0;
    }
    unsafe { super::numbers::wcstol(text, std::ptr::null_mut(), 10) }
}

pub unsafe extern "win64" fn wtoi_l(text: *const u16, _locale: usize) -> i32 {
    unsafe { wtoi(text) }
}

pub unsafe extern "win64" fn wtol(text: *const u16) -> i32 {
    if text.is_null() {
        return 0;
    }
    unsafe { super::numbers::wcstol(text, std::ptr::null_mut(), 10) }
}

pub unsafe extern "win64" fn wtol_l(text: *const u16, _locale: usize) -> i32 {
    unsafe { wtol(text) }
}

pub unsafe extern "win64" fn wtoi64(text: *const u16) -> i64 {
    if text.is_null() {
        return 0;
    }
    unsafe { super::numbers::wcstoi64(text, std::ptr::null_mut(), 10) }
}

pub unsafe extern "win64" fn wtoi64_l(text: *const u16, _locale: usize) -> i64 {
    unsafe { wtoi64(text) }
}

// ---- runtime ------------------------------------------------------------------------

type NewHandler = unsafe extern "win64" fn(usize) -> i32;
static NEW_HANDLER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// _set_new_handler: returns the previous handler.
pub extern "win64" fn set_new_handler(handler: usize) -> usize {
    NEW_HANDLER.swap(handler, std::sync::atomic::Ordering::AcqRel)
}
/// _callnewh: runs the new handler after a failed allocation; 1 if it ran and
/// reported success (retry), else 0.
pub unsafe extern "win64" fn call_new_handler(size: usize) -> i32 {
    let handler = NEW_HANDLER.load(std::sync::atomic::Ordering::Acquire);
    if handler == 0 {
        return 0;
    }
    let handler: NewHandler = unsafe { std::mem::transmute::<usize, NewHandler>(handler) };
    (unsafe { handler(size) } != 0) as i32
}
/// _seh_filter_dll: the filter of the DLL entry wrapper: let the exception through.
pub extern "win64" fn seh_filter_dll(_code: u32, _pointers: usize) -> i32 {
    0 // EXCEPTION_CONTINUE_SEARCH
}
type InvalidParameterHandler = unsafe extern "win64" fn(
    expression: *const u16,
    function: *const u16,
    file: *const u16,
    line: u32,
    reserved: usize,
);
static INVALID_PARAMETER_HANDLER: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub extern "win64" fn set_invalid_parameter_handler(handler: usize) -> usize {
    INVALID_PARAMETER_HANDLER.swap(handler, std::sync::atomic::Ordering::AcqRel)
}

pub extern "win64" fn get_invalid_parameter_handler() -> usize {
    INVALID_PARAMETER_HANDLER.load(std::sync::atomic::Ordering::Acquire)
}

pub extern "win64" fn set_thread_local_invalid_parameter_handler(handler: usize) -> usize {
    set_invalid_parameter_handler(handler)
}

pub extern "win64" fn get_thread_local_invalid_parameter_handler() -> usize {
    get_invalid_parameter_handler()
}

pub unsafe extern "win64" fn invalid_parameter(
    expression: *const u16,
    function: *const u16,
    file: *const u16,
    line: u32,
    reserved: usize,
) {
    let handler = INVALID_PARAMETER_HANDLER.load(std::sync::atomic::Ordering::Acquire);
    if handler != 0 {
        let handler: InvalidParameterHandler = unsafe { std::mem::transmute(handler) };
        unsafe { handler(expression, function, file, line, reserved) };
    }
}

pub unsafe extern "win64" fn invalid_parameter_noinfo() {
    unsafe { invalid_parameter(std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, 0) };
}

pub unsafe extern "win64" fn invalid_parameter_noinfo_noreturn() -> ! {
    unsafe { invalid_parameter(std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, 0) };
    std::process::abort();
}
/// __stdio_common_vsprintf_s: like vsprintf, but output that does not fit empties
/// the buffer, sets ERANGE and returns -1.
pub unsafe extern "win64" fn common_vsprintf_s(
    options: u64,
    buffer: *mut c_char,
    count: usize,
    format: *const c_char,
    locale: usize,
    args: *const u64,
) -> i32 {
    if buffer.is_null() || count == 0 || format.is_null() {
        unsafe { *errno() = 22 };
        return -1;
    }
    let needed = unsafe { super::ucrt::common_vsprintf(options | 2, std::ptr::null_mut(), 0, format, locale, args) };
    if needed < 0 || needed as usize >= count {
        unsafe {
            *buffer = 0;
            *errno() = 34;
        }
        return -1;
    }
    unsafe { super::ucrt::common_vsprintf(options, buffer, count, format, locale, args) }
}
/// vsprintf(buffer, format, argptr): standard C vsprintf for msvcrt.dll.
pub unsafe extern "win64" fn vsprintf(
    buffer: *mut c_char,
    format: *const c_char,
    args: *const u64,
) -> i32 {
    unsafe { super::ucrt::common_vsprintf(0, buffer, usize::MAX, format, 0, args) }
}

// ---- math ---------------------------------------------------------------------------

/// _dclass/_fdclass: FP_INFINITE 1, FP_NAN 2, FP_NORMAL -1, FP_SUBNORMAL -2, FP_ZERO 0.
fn classify(category: std::num::FpCategory) -> i16 {
    match category {
        std::num::FpCategory::Infinite => 1,
        std::num::FpCategory::Nan => 2,
        std::num::FpCategory::Normal => -1,
        std::num::FpCategory::Subnormal => -2,
        std::num::FpCategory::Zero => 0,
    }
}
pub extern "win64" fn dclass(x: f64) -> i16 {
    classify(x.classify())
}
pub extern "win64" fn fdclass(x: f32) -> i16 {
    classify(x.classify())
}
/// _dsign/_fdsign: nonzero (0x8000) when the sign bit is set.
pub extern "win64" fn dsign(x: f64) -> i32 {
    if x.is_sign_negative() { 0x8000 } else { 0 }
}
pub extern "win64" fn fdsign(x: f32) -> i32 {
    if x.is_sign_negative() { 0x8000 } else { 0 }
}
pub extern "win64" fn copy_sign(x: f64, y: f64) -> f64 {
    x.copysign(y)
}
pub unsafe extern "win64" fn frexp(x: f64, exponent: *mut i32) -> f64 {
    let (mantissa, e) = if x == 0.0 || !x.is_finite() {
        (x, 0)
    } else {
        let bits = x.to_bits();
        let raw = ((bits >> 52) & 0x7ff) as i32;
        if raw == 0 {
            // Subnormal: scale into the normal range first.
            let scaled = x * f64::from_bits(0x4350_0000_0000_0000); // 2^54
            let raw = ((scaled.to_bits() >> 52) & 0x7ff) as i32;
            (f64::from_bits((scaled.to_bits() & !(0x7ff << 52)) | (1022 << 52)), raw - 1022 - 54)
        } else {
            (f64::from_bits((bits & !(0x7ff << 52)) | (1022 << 52)), raw - 1022)
        }
    };
    if !exponent.is_null() {
        unsafe { *exponent = e };
    }
    mantissa
}
pub extern "win64" fn log_gamma(x: f64) -> f64 {
    unsafe { lgamma(x) }
}
pub extern "win64" fn log_gamma_f(x: f32) -> f32 {
    unsafe { lgammaf(x) }
}

// ---- wide-path file operations --------------------------------------------------------

/// The Win32 result of a file call as CRT errno: 0 on success, else -1 with errno.
fn errno_result(ok: bool) -> i32 {
    if ok {
        return 0;
    }
    let code = crate::api::system::error::get_last_error();
    unsafe {
        *errno() = match code {
            2 | 3 => 2,       // ENOENT
            5 | 32 => 13,     // EACCES
            80 | 183 => 17,   // EEXIST
            145 => 41,        // ENOTEMPTY
            17 => 18,         // EXDEV
            _ => 22,
        }
    };
    -1
}
pub unsafe extern "win64" fn wremove(path: *const u16) -> i32 {
    errno_result(unsafe { crate::api::filesystem::files::delete_w(path) } != 0)
}
pub unsafe extern "win64" fn wrename(from: *const u16, to: *const u16) -> i32 {
    errno_result(unsafe { crate::api::filesystem::files::move_w(from, to) } != 0)
}
pub unsafe extern "win64" fn wrmdir(path: *const u16) -> i32 {
    errno_result(unsafe { crate::api::filesystem::files::remove_directory_w(path) } != 0)
}
/// _fsopen: fopen; the share flag has no host equivalent (share modes are ignored).
pub unsafe extern "win64" fn fsopen(path: *const c_char, mode: *const c_char, _share: i32) -> *mut c_void {
    unsafe { super::stdio::open(path, mode) }
}
/// _get_stream_buffer_pointers: pointers to a FILE's _base (@8), _ptr (@0) and
/// _cnt (@16), as UCRT returns them. The host C library buffers our streams, so
/// these stay empty (count 0) and C++ streams fall back to fgetc/fputc/fread/fwrite.
pub unsafe extern "win64" fn stream_buffer_pointers(
    file: *mut u8,
    base: *mut *mut *mut c_char,
    pointer: *mut *mut *mut c_char,
    count: *mut *mut i32,
) -> i32 {
    if file.is_null() {
        unsafe { *errno() = 22 };
        return 22;
    }
    unsafe {
        if !base.is_null() {
            *base = file.add(8).cast();
        }
        if !pointer.is_null() {
            *pointer = file.cast();
        }
        if !count.is_null() {
            *count = file.add(16).cast();
        }
    }
    0
}

/// _get_heap_handle returns the CRT heap handle (the process heap).
pub extern "win64" fn get_heap_handle() -> usize {
    crate::api::memory::heap::get_process_heap()
}

pub extern "win64" fn purecall() -> ! {
    eprintln!("Pure virtual function call");
    std::process::exit(78);
}

// ---- C-locale time names (msvcp140 _Locinfo) ------------------------------------
const DAYS_ABBR: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS_ABBR: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
    "December",
];
const AMPM: [&str; 2] = ["AM", "PM"];
const SHORT_DATE: &str = "MM/dd/yy";
const LONG_DATE: &str = "dddd, MMMM dd, yyyy";
const TIME: &str = "HH:mm:ss";
/// ":Sun:Sunday:Mon:Monday..." (UCRT's _Getdays/_Getmonths format).
fn names(abbr: &[&str], full: &[&str]) -> String {
    abbr.iter().zip(full).map(|(a, f)| format!(":{a}:{f}")).collect()
}
fn malloc_narrow(text: &str) -> *mut c_char {
    let out = unsafe { malloc(text.len() + 1) }.cast::<u8>();
    if !out.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
            *out.add(text.len()) = 0;
        }
    }
    out.cast()
}
fn malloc_wide(text: &str) -> *mut u16 {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let out = unsafe { malloc(wide.len() * 2) }.cast::<u16>();
    if !out.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), out, wide.len()) };
    }
    out
}
/// _Getdays: the C locale's day names in CRT memory, freed with free().
pub extern "win64" fn get_days() -> *mut c_char {
    malloc_narrow(&names(&DAYS_ABBR, &DAYS))
}
pub extern "win64" fn get_months() -> *mut c_char {
    malloc_narrow(&names(&MONTHS_ABBR, &MONTHS))
}
pub extern "win64" fn get_days_w() -> *mut u16 {
    malloc_wide(&names(&DAYS_ABBR, &DAYS))
}
pub extern "win64" fn get_months_w() -> *mut u16 {
    malloc_wide(&names(&MONTHS_ABBR, &MONTHS))
}
/// _Gettnames/_W_Gettnames: one CRT block holding UCRT's __crt_lc_time_data for the
/// C locale followed by its strings; the caller frees it with free(). Layout (x64):
/// narrow pointers wday_abbr[7] @0, wday[7] @56, month_abbr[12] @112, month[12] @208,
/// ampm[2] @304, sdatefmt @320, ldatefmt @328, timefmt @336; ww_caltype @344,
/// refcount @348; the same 43 wide pointers from @352; locale name @696; 704 bytes.
pub extern "win64" fn get_time_names() -> *mut c_void {
    const HEADER: usize = 704;
    let narrow: Vec<&str> = DAYS_ABBR
        .iter()
        .chain(&DAYS)
        .chain(&MONTHS_ABBR)
        .chain(&MONTHS)
        .chain(&AMPM)
        .copied()
        .chain([SHORT_DATE, LONG_DATE, TIME])
        .collect();
    let narrow_bytes: usize = narrow.iter().map(|s| s.len() + 1).sum();
    let wide_units: usize = narrow.iter().map(|s| s.len() + 1).sum::<usize>() + 1; // + empty locale name
    let block = unsafe { malloc(HEADER + narrow_bytes + wide_units * 2) }.cast::<u8>();
    if block.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::write_bytes(block, 0, HEADER);
        let mut text = block.add(HEADER);
        for (i, s) in narrow.iter().enumerate() {
            block.add(i * 8).cast::<usize>().write_unaligned(text as usize);
            std::ptr::copy_nonoverlapping(s.as_ptr(), text, s.len());
            *text.add(s.len()) = 0;
            text = text.add(s.len() + 1);
        }
        block.add(344).cast::<i32>().write_unaligned(1); // CAL_GREGORIAN
        let mut wide = text.cast::<u16>();
        for (i, s) in narrow.iter().chain(std::iter::once(&"")).enumerate() {
            block.add(352 + i * 8).cast::<usize>().write_unaligned(wide as usize);
            for (j, unit) in s.encode_utf16().enumerate() {
                wide.add(j).write_unaligned(unit);
            }
            wide.add(s.len()).write_unaligned(0);
            wide = wide.add(s.len() + 1);
        }
    }
    block.cast()
}
/// _Strftime/_Wcsftime: strftime with an explicit __crt_lc_time_data. Only the C locale
/// exists here, and every time-data block this CRT hands out holds it.
pub unsafe extern "win64" fn strftime_l(
    out: *mut c_char, max: usize, format: *const c_char, tm: *const i32, _time_data: *const c_void,
) -> usize {
    unsafe { super::ucrt::strftime(out, max, format, tm) }
}
pub unsafe extern "win64" fn wcsftime_l(
    out: *mut u16, max: usize, format: *const u16, tm: *const i32, _time_data: *const c_void,
) -> usize {
    unsafe { super::ucrt::wcsftime(out, max, format, tm) }
}

// ---- UCRT imports of PlayFab Party (PartyWin.dll) -------------------------------
use crate::api::system::time::Tm;
unsafe extern "C" {
    fn timegm(tm: *mut Tm) -> i64;
    fn gmtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    fn sscanf(input: *const c_char, format: *const c_char, ...) -> i32;
}
/// _dtest: the FP_* class of *x (as _dclass).
pub unsafe extern "win64" fn dtest(x: *const f64) -> i16 {
    classify(unsafe { *x }.classify())
}
/// fmax: the larger argument; a NaN argument loses to a number.
pub extern "win64" fn fmax(a: f64, b: f64) -> f64 {
    a.max(b)
}
/// _difftime64: t1 - t0 in seconds; negative times are invalid (EINVAL, 0).
pub extern "win64" fn difftime64(t1: i64, t0: i64) -> f64 {
    if t1 < 0 || t0 < 0 {
        unsafe { *errno() = 22 };
        return 0.0;
    }
    (t1 - t0) as f64
}
/// _mkgmtime64: a UTC struct tm (normalised in place, tm_wday/tm_yday filled,
/// tm_isdst 0) to time_t. The UCRT range is 1970-01-01 through 3000-12-31 23:59:59;
/// outside it the result is -1 with EINVAL.
pub unsafe extern "win64" fn mkgmtime64(tm: *mut i32) -> i64 {
    const MAX: i64 = 32_535_215_999;
    if tm.is_null() {
        unsafe { *errno() = 22 };
        return -1;
    }
    // Linux struct tm starts with the same nine ints; tm_gmtoff/tm_zone stay zero.
    let mut host = Tm::default();
    unsafe { std::ptr::copy_nonoverlapping(tm, (&mut host as *mut Tm).cast::<i32>(), 8) };
    let time = unsafe { timegm(&mut host) };
    if !(0..=MAX).contains(&time) {
        unsafe { *errno() = 22 };
        return -1;
    }
    let mut out = Tm::default();
    if unsafe { gmtime_r(&time, &mut out) }.is_null() {
        unsafe { *errno() = 22 };
        return -1;
    }
    unsafe { std::ptr::copy_nonoverlapping((&out as *const Tm).cast::<i32>(), tm, 9) };
    time
}
/// __stdio_common_vsnprintf_s: at most `count` characters (or as many as fit with
/// _TRUNCATE), always terminated. Output cut by `count`/_TRUNCATE returns -1; output
/// that does not fit the buffer otherwise empties it with ERANGE and returns -1.
pub unsafe extern "win64" fn common_vsnprintf_s(
    options: u64,
    buffer: *mut c_char,
    size: usize,
    count: usize,
    format: *const c_char,
    locale: usize,
    args: *const u64,
) -> i32 {
    const TRUNCATE: usize = usize::MAX;
    if buffer.is_null() || size == 0 || format.is_null() {
        if !buffer.is_null() && size != 0 {
            unsafe { *buffer = 0 };
        }
        unsafe { *errno() = 22 };
        return -1;
    }
    let needed = unsafe { super::ucrt::common_vsprintf(options | 2, std::ptr::null_mut(), 0, format, locale, args) };
    if needed < 0 {
        unsafe { *buffer = 0 };
        return -1;
    }
    let needed = needed as usize;
    let limit = if count == TRUNCATE { size - 1 } else { count };
    if needed <= limit && needed < size {
        return unsafe { super::ucrt::common_vsprintf(options, buffer, size, format, locale, args) };
    }
    if count != TRUNCATE && count >= size {
        unsafe {
            *buffer = 0;
            *errno() = 34;
        }
        return -1;
    }
    // Truncated to `limit` characters.
    let mut full = vec![0 as c_char; needed + 1];
    unsafe {
        super::ucrt::common_vsprintf(options, full.as_mut_ptr(), full.len(), format, locale, args);
        std::ptr::copy_nonoverlapping(full.as_ptr(), buffer, limit);
        *buffer.add(limit) = 0;
    }
    -1
}
/// __stdio_common_vsscanf for narrow conversions. The Windows format is translated
/// for the host sscanf (all scanf arguments are pointers): I64/I/ll sizes, `l` on
/// integers (32-bit long), `L` on floats (double); with the secure-CRT option the
/// buffer size after each %s/%c/%[ becomes a field width. Wide conversions
/// (%ls, %lc, %S, %C, %w...) fail with EINVAL.
pub unsafe extern "win64" fn common_vsscanf(
    options: u64,
    input: *const c_char,
    length: usize,
    format: *const c_char,
    _locale: usize,
    args: *const usize,
) -> i32 {
    const SECURE: u64 = 1;
    if input.is_null() || format.is_null() {
        unsafe { *errno() = 22 };
        return -1;
    }
    let text: Vec<u8> = {
        let mut n = 0;
        while n < length && unsafe { *input.add(n) } != 0 {
            n += 1;
        }
        let mut v = unsafe { std::slice::from_raw_parts(input.cast::<u8>(), n) }.to_vec();
        v.push(0);
        v
    };
    let format = unsafe { std::ffi::CStr::from_ptr(format) }.to_bytes();
    let mut out = Vec::with_capacity(format.len() + 16);
    let mut pointers: Vec<usize> = Vec::new();
    let mut next = 0usize;
    let mut arg = || {
        let value = unsafe { *args.add(next) };
        next += 1;
        value
    };
    let mut i = 0;
    while i < format.len() {
        let c = format[i];
        out.push(c);
        i += 1;
        if c != b'%' {
            continue;
        }
        if format.get(i) == Some(&b'%') {
            out.push(b'%');
            i += 1;
            continue;
        }
        let suppressed = format.get(i) == Some(&b'*');
        if suppressed {
            out.push(b'*');
            i += 1;
        }
        let width_start = i;
        while format.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let width = &format[width_start..i];
        // Size prefix (Windows spelling), resolved against the conversion below.
        let mut size: &[u8] = b"";
        for prefix in [&b"I64"[..], b"I32", b"ll", b"hh", b"h", b"l", b"L", b"I", b"j", b"z", b"t", b"w"] {
            if format[i..].starts_with(prefix) {
                size = prefix;
                i += prefix.len();
                break;
            }
        }
        let Some(&conversion) = format.get(i) else {
            unsafe { *errno() = 22 };
            return -1;
        };
        i += 1;
        let integer = b"diouxXn".contains(&conversion);
        let float = b"eEfFgGaA".contains(&conversion);
        let text_conversion = matches!(conversion, b's' | b'c' | b'[');
        let wide = (text_conversion && matches!(size, b"l" | b"w")) || matches!(conversion, b'S' | b'C');
        if wide {
            eprintln!("__stdio_common_vsscanf: wide conversions are not supported");
            unsafe { *errno() = 22 };
            return -1;
        }
        let host_size: &[u8] = match (size, integer, float) {
            (b"I64" | b"ll" | b"I" | b"j" | b"z" | b"t", true, _) => b"ll",
            (b"I32" | b"l", true, _) => b"",
            (b"hh", true, _) => b"hh",
            (b"h", true, _) => b"h",
            (b"l" | b"L", _, true) => b"l",
            (b"", _, _) => b"",
            _ if text_conversion => b"",
            _ => {
                unsafe { *errno() = 22 };
                return -1;
            }
        };
        if !suppressed && text_conversion && options & SECURE != 0 {
            let pointer = arg();
            let buffer_size = arg() as u32 as usize;
            if buffer_size == 0 {
                unsafe { *errno() = 12 }; // ENOMEM, as UCRT for a zero-sized buffer
                return 0;
            }
            let room = if conversion == b'c' { buffer_size } else { buffer_size - 1 };
            let given: usize = std::str::from_utf8(width).ok().and_then(|w| w.parse().ok()).unwrap_or(0);
            let w = if width.is_empty() { if conversion == b'c' { 1 } else { room } } else { given.min(room) };
            if w == 0 {
                unsafe { *errno() = 12 };
                return 0;
            }
            out.extend_from_slice(w.to_string().as_bytes());
            pointers.push(pointer);
        } else {
            out.extend_from_slice(width);
            if !suppressed {
                pointers.push(arg());
            }
        }
        out.extend_from_slice(host_size);
        out.push(conversion);
        if conversion == b'[' {
            // The scanset: optional '^', a leading ']' is literal, then up to ']'.
            let begin = i;
            if format.get(i) == Some(&b'^') {
                i += 1;
            }
            if format.get(i) == Some(&b']') {
                i += 1;
            }
            while format.get(i).is_some_and(|&s| s != b']') {
                i += 1;
            }
            if i >= format.len() {
                unsafe { *errno() = 22 };
                return -1;
            }
            i += 1;
            out.extend_from_slice(&format[begin..i]);
        }
        if pointers.len() > 32 {
            eprintln!("__stdio_common_vsscanf: more than 32 conversions are not supported");
            unsafe { *errno() = 22 };
            return -1;
        }
    }
    out.push(0);
    let mut p = [0usize; 32];
    p[..pointers.len()].copy_from_slice(&pointers);
    unsafe {
        sscanf(
            text.as_ptr().cast(), out.as_ptr().cast(),
            p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15],
            p[16], p[17], p[18], p[19], p[20], p[21], p[22], p[23], p[24], p[25], p[26], p[27], p[28], p[29], p[30], p[31],
        )
    }
}
