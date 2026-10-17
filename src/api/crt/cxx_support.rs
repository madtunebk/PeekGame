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
