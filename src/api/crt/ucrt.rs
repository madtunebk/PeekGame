//! The Universal CRT (ucrtbase.dll and its api-ms-win-crt-* sets) beyond what
//! msvcrt already provides: what MinGW-built Windows libraries such as DXVK's
//! d3d11.dll and dxgi.dll import. printf-family formatting reads the Windows x64
//! va_list (a pointer to 8-byte argument slots).
use super::stdio;
use std::{
    ffi::{c_char, c_void, CStr},
    sync::{Mutex, OnceLock},
};
unsafe extern "C" {
    fn fetestexcept(excepts: i32) -> i32;
    fn feclearexcept(excepts: i32) -> i32;
    #[link_name = "fputc"]
    fn host_fputc(c: i32, file: *mut c_void) -> i32;
    #[link_name = "fputs"]
    fn host_fputs(s: *const c_char, file: *mut c_void) -> i32;
    #[link_name = "getc"]
    fn host_getc(file: *mut c_void) -> i32;
    #[link_name = "ungetc"]
    fn host_ungetc(c: i32, file: *mut c_void) -> i32;
    #[link_name = "setvbuf"]
    fn host_setvbuf(file: *mut c_void, buffer: *mut c_char, mode: i32, size: usize) -> i32;
    #[link_name = "fwrite"]
    fn host_fwrite(data: *const c_void, size: usize, count: usize, file: *mut c_void) -> usize;
    #[link_name = "fdopen"]
    fn host_fdopen(fd: i32, mode: *const c_char) -> *mut c_void;
    #[link_name = "flockfile"]
    fn host_flockfile(file: *mut c_void);
    #[link_name = "funlockfile"]
    fn host_funlockfile(file: *mut c_void);
    #[link_name = "strtoul"]
    fn host_strtoul(s: *const c_char, end: *mut *mut c_char, base: i32) -> u64;
    #[link_name = "strerror"]
    fn host_strerror(code: i32) -> *mut c_char;
    #[link_name = "strxfrm"]
    fn host_strxfrm(out: *mut c_char, s: *const c_char, n: usize) -> usize;
    #[link_name = "posix_memalign"]
    fn host_memalign(out: *mut *mut c_void, align: usize, size: usize) -> i32;
    #[link_name = "free"]
    fn host_free(p: *mut c_void);
    #[link_name = "realloc"]
    fn host_realloc(p: *mut c_void, size: usize) -> *mut c_void;
    #[link_name = "lseek"]
    fn host_lseek(fd: i32, offset: i64, whence: i32) -> i64;
    #[link_name = "read"]
    fn host_read(fd: i32, buffer: *mut c_void, n: usize) -> isize;
    #[link_name = "write"]
    fn host_write(fd: i32, buffer: *const c_void, n: usize) -> isize;
    #[link_name = "fstat"]
    fn host_fstat(fd: i32, out: *mut u8) -> i32;
    #[link_name = "remove"]
    fn host_remove(path: *const c_char) -> i32;
    #[link_name = "strftime"]
    fn host_strftime(
        out: *mut c_char,
        max: usize,
        format: *const c_char,
        tm: *const HostTm,
    ) -> usize;
    #[link_name = "tzset"]
    fn host_tzset();
    #[link_name = "getrandom"]
    fn host_getrandom(buffer: *mut u8, length: usize, flags: u32) -> isize;
    #[link_name = "_exit"]
    fn host_exit(code: i32) -> !;
    static daylight: i32;
    static timezone: i64;
    static tzname: [*mut c_char; 2];
}

pub extern "win64" fn clearfp() -> u32 {
    // glibc x86 FE_* flags use the same low six bit positions as x87 status.
    let flags = unsafe { fetestexcept(0x3f) };
    unsafe { feclearexcept(0x3f) };
    ((flags & 0x20 != 0) as u32)
        | (((flags & 0x10 != 0) as u32) << 1)
        | (((flags & 0x08 != 0) as u32) << 2)
        | (((flags & 0x04 != 0) as u32) << 3)
        | (((flags & 0x01 != 0) as u32) << 4)
        | (((flags & 0x02 != 0) as u32) << 18)
}
fn set_errno(code: i32) {
    unsafe { *stdio::errno() = code };
}

// ---- printf -----------------------------------------------------------------------
/// The Windows x64 va_list: 8-byte slots, floating values as their bits.
struct Args(*const u64);
impl Args {
    fn next(&mut self) -> u64 {
        let v = unsafe { self.0.read_unaligned() };
        self.0 = unsafe { self.0.add(1) };
        v
    }
}
fn c_exponent(text: String) -> String {
    // Rust "1.5e3" -> C "1.5e+03"
    match text.split_once('e') {
        Some((m, e)) => {
            let (sign, digits) = if let Some(d) = e.strip_prefix('-') {
                ('-', d)
            } else {
                ('+', e)
            };
            format!("{m}e{sign}{digits:0>2}")
        }
        None => text,
    }
}
fn general(v: f64, precision: usize, alternate: bool) -> String {
    let p = precision.max(1);
    if v == 0.0 {
        return "0".into();
    }
    let exponent = v.abs().log10().floor() as i32;
    let mut s = if exponent < -4 || exponent >= p as i32 {
        c_exponent(format!("{:.*e}", p - 1, v))
    } else {
        format!("{:.*}", (p as i32 - 1 - exponent).max(0) as usize, v)
    };
    if !alternate {
        if let Some((mantissa, rest)) = s.clone().split_once('e') {
            let m = if mantissa.contains('.') {
                mantissa.trim_end_matches('0').trim_end_matches('.')
            } else {
                mantissa
            };
            s = format!("{m}e{rest}");
        } else if s.contains('.') {
            s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
        }
    }
    s
}
/// Formats a printf format string (bytes or UTF-16 units) into UTF-8 text.
fn format(spec: &[u32], args: &mut Args, wide: bool) -> String {
    let mut out = String::new();
    let mut i = 0;
    let at = |i: usize| spec.get(i).copied().unwrap_or(0);
    while i < spec.len() {
        let c = spec[i];
        if c != '%' as u32 {
            out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
            i += 1;
            continue;
        }
        i += 1;
        let (mut left, mut plus, mut space, mut zero, mut alternate) =
            (false, false, false, false, false);
        loop {
            match char::from_u32(at(i)) {
                Some('-') => left = true,
                Some('+') => plus = true,
                Some(' ') => space = true,
                Some('0') => zero = true,
                Some('#') => alternate = true,
                _ => break,
            }
            i += 1;
        }
        let mut width = 0usize;
        if at(i) == '*' as u32 {
            let w = args.next() as i32;
            if w < 0 {
                left = true;
            }
            width = w.unsigned_abs() as usize;
            i += 1;
        } else {
            while (b'0' as u32..=b'9' as u32).contains(&at(i)) {
                width = width * 10 + (at(i) - b'0' as u32) as usize;
                i += 1;
            }
        }
        let mut precision = None;
        if at(i) == '.' as u32 {
            i += 1;
            let mut p = 0usize;
            if at(i) == '*' as u32 {
                let v = args.next() as i32;
                p = v.max(0) as usize;
                i += 1;
            } else {
                while (b'0' as u32..=b'9' as u32).contains(&at(i)) {
                    p = p * 10 + (at(i) - b'0' as u32) as usize;
                    i += 1;
                }
            }
            precision = Some(p);
        }
        // Length: hh h l ll L z j t I I32 I64 w.
        let mut size = 4usize; // int
        let mut long_text = false;
        let mut short_text = false;
        loop {
            match char::from_u32(at(i)) {
                Some('h') => {
                    size = if size == 2 { 1 } else { 2 };
                    short_text = true;
                }
                Some('l') => {
                    if long_text {
                        size = 8;
                    }
                    long_text = true;
                }
                Some('L') | Some('j') | Some('z') | Some('t') => size = 8,
                Some('w') => long_text = true,
                Some('I') => {
                    if at(i + 1) == '6' as u32 && at(i + 2) == '4' as u32 {
                        size = 8;
                        i += 2;
                    } else if at(i + 1) == '3' as u32 && at(i + 2) == '2' as u32 {
                        size = 4;
                        i += 2;
                    } else {
                        size = 8;
                    }
                }
                _ => break,
            }
            i += 1;
        }
        let conversion = char::from_u32(at(i)).unwrap_or('%');
        i += 1;
        let signed = |v: u64| -> i64 {
            match size {
                1 => v as i8 as i64,
                2 => v as i16 as i64,
                4 => v as i32 as i64,
                _ => v as i64,
            }
        };
        let unsigned = |v: u64| -> u64 {
            match size {
                1 => v as u8 as u64,
                2 => v as u16 as u64,
                4 => v as u32 as u64,
                _ => v,
            }
        };
        let mut sign = String::new();
        let mut prefix = String::new();
        let body = match conversion {
            'd' | 'i' => {
                let v = signed(args.next());
                if v < 0 {
                    sign.push('-');
                } else if plus {
                    sign.push('+');
                } else if space {
                    sign.push(' ');
                }
                let digits = v.unsigned_abs().to_string();
                match precision {
                    Some(0) if v == 0 => String::new(),
                    Some(p) => format!("{digits:0>p$}"),
                    None => digits,
                }
            }
            'u' | 'x' | 'X' | 'o' => {
                let v = unsigned(args.next());
                let digits = match conversion {
                    'u' => v.to_string(),
                    'x' => format!("{v:x}"),
                    'X' => format!("{v:X}"),
                    _ => format!("{v:o}"),
                };
                if alternate && v != 0 {
                    prefix = match conversion {
                        'x' => "0x".into(),
                        'X' => "0X".into(),
                        'o' => "0".into(),
                        _ => String::new(),
                    };
                }
                match precision {
                    Some(0) if v == 0 => String::new(),
                    Some(p) => format!("{digits:0>p$}"),
                    None => digits,
                }
            }
            'p' => format!("{:016X}", args.next()),
            'f' | 'F' | 'e' | 'E' | 'g' | 'G' | 'a' | 'A' => {
                let v = f64::from_bits(args.next());
                if v.is_sign_negative() && !v.is_nan() {
                    sign.push('-');
                } else if plus {
                    sign.push('+');
                } else if space {
                    sign.push(' ');
                }
                let v = v.abs();
                let p = precision.unwrap_or(6);
                let text = if v.is_nan() {
                    "nan".into()
                } else if v.is_infinite() {
                    "inf".into()
                } else {
                    match conversion {
                        'f' | 'F' => format!("{v:.p$}"),
                        'e' | 'E' => c_exponent(format!("{v:.p$e}")),
                        'a' | 'A' => format!("{v:e}"),
                        _ => general(v, p, alternate),
                    }
                };
                if conversion.is_ascii_uppercase() {
                    text.to_uppercase()
                } else {
                    text
                }
            }
            'c' | 'C' => {
                let v = args.next() as u32;
                // %c is the format's own width, %C the other; l/w force wide.
                let wide_arg = long_text || (!short_text && (conversion == 'c') == wide);
                char::from_u32(if wide_arg { v & 0xffff } else { v & 0xff })
                    .unwrap_or('?')
                    .to_string()
            }
            's' | 'S' => {
                let p = args.next();
                // %s is the format's own width, %S the other; h/l force narrow/wide.
                let wide_arg = if long_text {
                    true
                } else if short_text {
                    false
                } else {
                    (conversion == 's') == wide
                };
                if p == 0 {
                    "(null)".into()
                } else if wide_arg {
                    let text = unsafe { crate::api::text::units(p as *const u16) };
                    let n = precision.map_or(text.len(), |m| m.min(text.len()));
                    String::from_utf16_lossy(&text[..n])
                } else {
                    let bytes = unsafe { CStr::from_ptr(p as *const c_char) }.to_bytes();
                    let n = precision.map_or(bytes.len(), |m| m.min(bytes.len()));
                    String::from_utf8_lossy(&bytes[..n]).into_owned()
                }
            }
            'n' => {
                let p = args.next() as *mut i32;
                if !p.is_null() {
                    unsafe { *p = out.chars().count() as i32 };
                }
                String::new()
            }
            '%' => "%".into(),
            other => format!("%{other}"),
        };
        let length = sign.chars().count() + prefix.chars().count() + body.chars().count();
        let pad = width.saturating_sub(length);
        let numeric = matches!(
            conversion,
            'd' | 'i' | 'u' | 'x' | 'X' | 'o' | 'f' | 'F' | 'e' | 'E' | 'g' | 'G'
        );
        if left {
            out += &sign;
            out += &prefix;
            out += &body;
            out.extend(std::iter::repeat_n(' ', pad));
        } else if zero
            && numeric
            && (precision.is_none() || matches!(conversion, 'f' | 'F' | 'e' | 'E' | 'g' | 'G'))
        {
            out += &sign;
            out += &prefix;
            out.extend(std::iter::repeat_n('0', pad));
            out += &body;
        } else {
            out.extend(std::iter::repeat_n(' ', pad));
            out += &sign;
            out += &prefix;
            out += &body;
        }
    }
    out
}
fn narrow_spec(format: *const c_char) -> Vec<u32> {
    if format.is_null() {
        return Vec::new();
    }
    let bytes = unsafe { CStr::from_ptr(format) }.to_bytes();
    String::from_utf8_lossy(bytes)
        .chars()
        .map(|c| c as u32)
        .collect()
}
fn wide_spec(format: *const u16) -> Vec<u32> {
    let units = unsafe { crate::api::text::units(format) };
    char::decode_utf16(units.iter().copied())
        .map(|c| c.unwrap_or('\u{fffd}') as u32)
        .collect()
}
/// __stdio_common_vfprintf(options, FILE*, format, locale, va_list).
pub unsafe extern "win64" fn common_vfprintf(
    _options: u64,
    file: *mut c_void,
    format: *const c_char,
    _locale: usize,
    args: *const u64,
) -> i32 {
    let text = self::format(&narrow_spec(format), &mut Args(args), false);
    unsafe { host_fwrite(text.as_ptr().cast(), 1, text.len(), stdio::host(file)) };
    text.len() as i32
}
pub unsafe extern "win64" fn common_vfwprintf(
    _options: u64,
    file: *mut c_void,
    format: *const u16,
    _locale: usize,
    args: *const u64,
) -> i32 {
    let text = self::format(&wide_spec(format), &mut Args(args), true);
    unsafe { host_fwrite(text.as_ptr().cast(), 1, text.len(), stdio::host(file)) };
    text.encode_utf16().count() as i32
}
/// __stdio_common_vsprintf(options, buffer, count, format, locale, va_list). With
/// _CRT_INTERNAL_PRINTF_STANDARD_SNPRINTF_BEHAVIOR (2) the full length is returned
/// when truncating, else -1.
pub unsafe extern "win64" fn common_vsprintf(
    options: u64,
    buffer: *mut c_char,
    count: usize,
    format: *const c_char,
    _locale: usize,
    args: *const u64,
) -> i32 {
    let text = self::format(&narrow_spec(format), &mut Args(args), false);
    let bytes = text.as_bytes();
    if !buffer.is_null() && count > 0 {
        let n = bytes.len().min(count - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), buffer, n);
            *buffer.add(n) = 0;
        }
        if bytes.len() >= count && options & 2 == 0 {
            return -1;
        }
    }
    bytes.len() as i32
}
pub unsafe extern "win64" fn common_vswprintf(
    options: u64,
    buffer: *mut u16,
    count: usize,
    format: *const u16,
    _locale: usize,
    args: *const u64,
) -> i32 {
    let text: Vec<u16> = self::format(&wide_spec(format), &mut Args(args), true)
        .encode_utf16()
        .collect();
    if !buffer.is_null() && count > 0 {
        let n = text.len().min(count - 1);
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), buffer, n);
            *buffer.add(n) = 0;
        }
        if text.len() >= count && options & 2 == 0 {
            return -1;
        }
    }
    text.len() as i32
}
pub extern "win64" fn acrt_iob_func(index: u32) -> *mut c_void {
    (stdio::iob() as usize + 48 * index.min(2) as usize) as *mut c_void
}

// ---- stdio by character ----------------------------------------------------------------
pub unsafe extern "win64" fn fputc(c: i32, file: *mut c_void) -> i32 {
    unsafe { host_fputc(c, stdio::host(file)) }
}
pub unsafe extern "win64" fn fputs(s: *const c_char, file: *mut c_void) -> i32 {
    unsafe { host_fputs(s, stdio::host(file)) }
}
pub unsafe extern "win64" fn getc(file: *mut c_void) -> i32 {
    unsafe { host_getc(stdio::host(file)) }
}
pub unsafe extern "win64" fn ungetc(c: i32, file: *mut c_void) -> i32 {
    unsafe { host_ungetc(c, stdio::host(file)) }
}
/// Wide characters on byte streams are UTF-8.
pub unsafe extern "win64" fn putwc(c: u16, file: *mut c_void) -> u16 {
    let mut buffer = [0u8; 4];
    let s = char::from_u32(c as u32)
        .unwrap_or('\u{fffd}')
        .encode_utf8(&mut buffer);
    unsafe { host_fwrite(s.as_ptr().cast(), 1, s.len(), stdio::host(file)) };
    c
}
pub unsafe extern "win64" fn getwc(file: *mut c_void) -> u16 {
    let first = unsafe { host_getc(stdio::host(file)) };
    if first < 0 {
        return 0xffff; // WEOF
    }
    let extra = match first {
        0xc0..=0xdf => 1,
        0xe0..=0xef => 2,
        0xf0..=0xf7 => 3,
        _ => 0,
    };
    let mut bytes = vec![first as u8];
    for _ in 0..extra {
        let b = unsafe { host_getc(stdio::host(file)) };
        if b < 0 {
            break;
        }
        bytes.push(b as u8);
    }
    std::str::from_utf8(&bytes)
        .ok()
        .and_then(|s| s.chars().next())
        .map_or(0xfffd, |c| c as u32 as u16)
}
pub unsafe extern "win64" fn ungetwc(c: u16, file: *mut c_void) -> u16 {
    if c < 0x80 {
        unsafe { host_ungetc(c as i32, stdio::host(file)) };
        c
    } else {
        0xffff
    }
}
/// setvbuf: _IOFBF 0, _IOLBF 0x40, _IONBF 4 on Windows.
pub unsafe extern "win64" fn setvbuf(
    file: *mut c_void,
    buffer: *mut c_char,
    mode: i32,
    size: usize,
) -> i32 {
    let mode = match mode {
        0 => 0,
        0x40 => 1,
        _ => 2,
    };
    unsafe { host_setvbuf(stdio::host(file), buffer, mode, size) }
}
pub unsafe extern "win64" fn lock_file(file: *mut c_void) {
    unsafe { host_flockfile(stdio::host(file)) }
}
pub unsafe extern "win64" fn unlock_file(file: *mut c_void) {
    unsafe { host_funlockfile(stdio::host(file)) }
}
pub unsafe extern "win64" fn fdopen(fd: i32, mode: *const c_char) -> *mut c_void {
    let mode: String = unsafe { CStr::from_ptr(mode) }
        .to_string_lossy()
        .chars()
        .filter(|&c| c != 't')
        .collect();
    let mode = std::ffi::CString::new(mode).unwrap_or_default();
    let host = unsafe { host_fdopen(fd, mode.as_ptr()) };
    if host.is_null() {
        return host;
    }
    stdio::register(host)
}
/// Descriptor metadata query: does not seek or disturb a buffered stream.
pub extern "win64" fn filelengthi64(fd: i32) -> i64 {
    // Linux x86-64 struct stat, shared with fstat64 below: st_size at byte 48.
    let mut st = [0u64; 18];
    if unsafe { host_fstat(fd, st.as_mut_ptr().cast()) } != 0 {
        return -1; // host errno is the thread's CRT errno (EBADF is 9 on both).
    }
    st[6] as i64
}
pub extern "win64" fn lseeki64(fd: i32, offset: i64, whence: i32) -> i64 {
    unsafe { host_lseek(fd, offset, whence) }
}
pub unsafe extern "win64" fn read(fd: i32, buffer: *mut c_void, n: u32) -> i32 {
    unsafe { host_read(fd, buffer, n as usize) as i32 }
}
pub unsafe extern "win64" fn write(fd: i32, buffer: *const c_void, n: u32) -> i32 {
    unsafe { host_write(fd, buffer, n as usize) as i32 }
}
/// _fstat64: struct _stat64 (dev u32, ino u16, mode u16, nlink i16, uid i16, gid
/// i16, rdev u32 @16, size @24, atime @32, mtime @40, ctime @48).
pub unsafe extern "win64" fn fstat64(fd: i32, out: *mut u8) -> i32 {
    let mut st = [0u8; 144];
    if out.is_null() || unsafe { host_fstat(fd, st.as_mut_ptr()) } != 0 {
        set_errno(9);
        return -1;
    }
    let u64_at = |o: usize| u64::from_le_bytes(st[o..o + 8].try_into().unwrap());
    let u32_at = |o: usize| u32::from_le_bytes(st[o..o + 4].try_into().unwrap());
    unsafe {
        out.write_bytes(0, 56);
        out.cast::<u32>().write_unaligned(u64_at(0) as u32);
        out.add(4).cast::<u16>().write_unaligned(u64_at(8) as u16);
        out.add(6)
            .cast::<u16>()
            .write_unaligned((u32_at(24) & 0xf1ff) as u16);
        out.add(8).cast::<i16>().write_unaligned(u64_at(16) as i16);
        out.add(16).cast::<u32>().write_unaligned(u64_at(40) as u32);
        out.add(24).cast::<i64>().write_unaligned(u64_at(48) as i64);
        out.add(32).cast::<i64>().write_unaligned(u64_at(72) as i64);
        out.add(40).cast::<i64>().write_unaligned(u64_at(88) as i64);
        out.add(48)
            .cast::<i64>()
            .write_unaligned(u64_at(104) as i64);
    }
    0
}
pub unsafe extern "win64" fn remove(path: *const c_char) -> i32 {
    let Ok(path) =
        (unsafe { crate::host::paths::narrow(path) }).and_then(|s| crate::host::paths::host(&s))
    else {
        set_errno(22);
        return -1;
    };
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap_or_default();
    unsafe { host_remove(path.as_ptr()) }
}

// ---- runtime --------------------------------------------------------------------------------
type Callback = unsafe extern "win64" fn();
/// _onexit_table_t: first, last, end.
#[repr(C)]
pub struct OnexitTable {
    first: *mut usize,
    last: *mut usize,
    end: *mut usize,
}
pub unsafe extern "win64" fn initialize_onexit_table(table: *mut OnexitTable) -> i32 {
    if table.is_null() {
        return -1;
    }
    unsafe {
        (*table).first = std::ptr::null_mut();
        (*table).last = std::ptr::null_mut();
        (*table).end = std::ptr::null_mut();
    }
    0
}
pub unsafe extern "win64" fn register_onexit_function(
    table: *mut OnexitTable,
    function: usize,
) -> i32 {
    if table.is_null() {
        return -1;
    }
    let t = unsafe { &mut *table };
    unsafe {
        if t.last == t.end {
            let count = if t.first.is_null() {
                0
            } else {
                t.end.offset_from(t.first) as usize
            };
            let capacity = (count * 2).max(8);
            let grown = host_realloc(t.first.cast(), capacity * 8).cast::<usize>();
            if grown.is_null() {
                return -1;
            }
            t.first = grown;
            t.last = grown.add(count);
            t.end = grown.add(capacity);
        }
        *t.last = function;
        t.last = t.last.add(1);
    }
    0
}
/// Runs the table's functions, last registered first, and empties it.
pub unsafe extern "win64" fn execute_onexit_table(table: *mut OnexitTable) -> i32 {
    if table.is_null() {
        return -1;
    }
    let t = unsafe { &mut *table };
    let (first, last) = (t.first, t.last);
    unsafe { initialize_onexit_table(table) };
    if first.is_null() {
        return 0;
    }
    let mut p = last;
    while p > first {
        p = unsafe { p.sub(1) };
        let f = unsafe { *p };
        if f != 0 {
            let f: Callback = unsafe { std::mem::transmute(f) };
            unsafe { f() };
        }
    }
    unsafe { host_free(first.cast()) };
    0
}
pub extern "win64" fn crt_atexit(function: Option<Callback>) -> i32 {
    super::startup::register(function.map(|f| unsafe { std::mem::transmute(f) }))
}
pub extern "win64" fn crt_at_quick_exit(_function: usize) -> i32 {
    0
}
pub extern "win64" fn configure_argv(_mode: i32) -> i32 {
    0
}
pub extern "win64" fn initialize_environment() -> i32 {
    0
}
struct MainArgs {
    argc: i32,
    argv: *const *const c_char,
    env: *const *const c_char,
}
unsafe impl Send for MainArgs {}
unsafe impl Sync for MainArgs {}
fn main_args() -> &'static MainArgs {
    static ARGS: OnceLock<MainArgs> = OnceLock::new();
    ARGS.get_or_init(|| {
        let (mut argc, mut argv, mut env) = (0, std::ptr::null(), std::ptr::null());
        unsafe {
            super::startup::mainargs(&mut argc, &mut argv, &mut env, 0, std::ptr::null_mut())
        };
        MainArgs { argc, argv, env }
    })
}
pub extern "win64" fn p_argc() -> *const i32 {
    &main_args().argc
}
pub extern "win64" fn p_argv() -> *const *const *const c_char {
    &main_args().argv
}
pub extern "win64" fn p_environ() -> *const *const *const c_char {
    &main_args().env
}
struct Wide(
    #[allow(dead_code)] Vec<Vec<u16>>,
    #[allow(dead_code)] Vec<*const u16>,
    *const *const u16,
);
unsafe impl Send for Wide {}
unsafe impl Sync for Wide {}
fn wide_list(items: Vec<String>) -> Wide {
    let strings: Vec<Vec<u16>> = items
        .into_iter()
        .map(|s| s.encode_utf16().chain([0]).collect())
        .collect();
    let mut pointers: Vec<*const u16> = strings.iter().map(|s| s.as_ptr()).collect();
    pointers.push(std::ptr::null());
    let base = pointers.as_ptr();
    Wide(strings, pointers, base)
}
pub extern "win64" fn p_wargv() -> *const *const *const u16 {
    static ARGS: OnceLock<Wide> = OnceLock::new();
    let a = ARGS.get_or_init(|| {
        let m = main_args();
        let items = (0..m.argc as usize)
            .map(|i| {
                unsafe { CStr::from_ptr(*m.argv.add(i)) }
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        wide_list(items)
    });
    &a.2
}
pub extern "win64" fn p_wenviron() -> *const *const *const u16 {
    static ENV: OnceLock<Wide> = OnceLock::new();
    let e =
        ENV.get_or_init(|| wide_list(std::env::vars().map(|(k, v)| format!("{k}={v}")).collect()));
    &e.2
}
pub extern "win64" fn end_thread(code: u32) -> ! {
    crate::host::thread_entry::exit(code)
}
pub extern "win64" fn exit_now(code: i32) -> ! {
    eprintln!("CRT _exit called with code {code}");
    let rsp_val: usize;
    unsafe { std::arch::asm!("mov {}, rsp", out(reg) rsp_val); }
    let ptr = rsp_val as *const usize;
    for i in 0..32 {
        let val = unsafe { ptr.add(i).read() };
        if let Some(m) = crate::images::try_by_address(val) {
            eprintln!("  exit_stack[{i}] = {val:#x} ({}+{:#x})", m.name, val - m.base);
        } else if val >= 0x140000000 && val < 0x150000000 {
            eprintln!("  exit_stack[{i}] = {val:#x} (Palworld+{:#x})", val - 0x140000000);
        }
    }
    unsafe { host_exit(code) }
}
pub extern "win64" fn abort() -> ! {
    eprintln!("CRT abort() called");
    unsafe { host_exit(3) }
}
pub extern "win64" fn terminate() -> ! {
    eprintln!("CRT terminate() called");
    unsafe { host_exit(3) }
}
/// signal: handlers are accepted and never called (SIG_DFL returned).
pub extern "win64" fn signal(_number: i32, _handler: usize) -> usize {
    0
}
pub extern "win64" fn strerror(code: i32) -> *mut c_char {
    unsafe { host_strerror(code) }
}

pub unsafe extern "win64" fn strerror_s(buffer: *mut c_char, size: usize, code: i32) -> i32 {
    if buffer.is_null() || size == 0 {
        return 22; // EINVAL
    }
    let msg = unsafe { host_strerror(code) };
    if msg.is_null() {
        unsafe { *buffer = 0 };
        return 22;
    }
    let bytes = unsafe { std::ffi::CStr::from_ptr(msg) }.to_bytes();
    if bytes.len() >= size {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, size - 1);
            *buffer.add(size - 1) = 0;
        }
        return 34; // ERANGE
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, bytes.len());
        *buffer.add(bytes.len()) = 0;
    }
    0
}

pub unsafe extern "win64" fn wcserror_s(buffer: *mut u16, size: usize, code: i32) -> i32 {
    if buffer.is_null() || size == 0 {
        return 22;
    }
    let msg = unsafe { host_strerror(code) };
    if msg.is_null() {
        unsafe { *buffer = 0 };
        return 22;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(msg) }.to_string_lossy();
    let units: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    if units.len() > size {
        unsafe {
            std::ptr::copy_nonoverlapping(units.as_ptr(), buffer, size - 1);
            *buffer.add(size - 1) = 0;
        }
        return 34;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(units.as_ptr(), buffer, units.len());
    }
    0
}

pub unsafe extern "win64" fn strtoul(s: *const c_char, end: *mut *mut c_char, base: i32) -> u32 {
    let v = unsafe { host_strtoul(s, end, base) };
    if v > u32::MAX as u64 {
        set_errno(34);
        u32::MAX
    } else {
        v as u32
    }
}
pub unsafe extern "win64" fn rand_s(out: *mut u32) -> i32 {
    if out.is_null() || unsafe { host_getrandom(out.cast(), 4, 0) } != 4 {
        return 22;
    }
    0
}
pub unsafe extern "win64" fn aligned_malloc(size: usize, align: usize) -> *mut c_void {
    let mut p = std::ptr::null_mut();
    if !align.is_power_of_two() || unsafe { host_memalign(&mut p, align.max(8), size) } != 0 {
        set_errno(12);
        return std::ptr::null_mut();
    }
    p
}
pub unsafe extern "win64" fn aligned_free(p: *mut c_void) {
    unsafe { host_free(p) }
}
pub unsafe extern "win64" fn aligned_realloc(
    p: *mut c_void,
    size: usize,
    align: usize,
) -> *mut c_void {
    if p.is_null() {
        return unsafe { aligned_malloc(size, align) };
    }
    if size == 0 {
        unsafe { host_free(p) };
        return std::ptr::null_mut();
    }
    unsafe { host_realloc(p, size) }
}
pub extern "win64" fn heapchk() -> i32 {
    -2 // _HEAPOK
}
pub extern "win64" fn set_new_mode(_mode: i32) -> i32 {
    0
}
pub extern "win64" fn control87(_new_val: u32, _mask: u32) -> u32 {
    0x0008_001f
}
pub unsafe extern "win64" fn controlfp_s(
    current: *mut u32,
    _new_val: u32,
    _mask: u32,
) -> i32 {
    if !current.is_null() {
        unsafe { *current = 0x0008_001f };
    }
    0
}
pub extern "win64" fn doserrno() -> *mut u32 {
    static mut DOSERR: u32 = 0;
    &raw mut DOSERR
}
pub extern "win64" fn sys_nerr() -> *mut i32 {
    static mut NERR: i32 = 1;
    &raw mut NERR
}
pub extern "win64" fn sys_errlist() -> *const *const c_char {
    static MSG: &[u8] = b"Success\0";
    static mut LIST: [*const c_char; 2] = [std::ptr::null(); 2];
    unsafe {
        LIST[0] = MSG.as_ptr().cast();
        (&raw const LIST).cast()
    }
}
pub extern "win64" fn wassert(_expr: *const u16, _file: *const u16, _line: u32) {}
pub extern "win64" fn raise(_sig: i32) -> i32 {
    0
}
pub extern "win64" fn div(numer: i32, denom: i32) -> u64 {
    if denom == 0 {
        return 0;
    }
    let quot = (numer / denom) as u32 as u64;
    let rem = (numer % denom) as u32 as u64;
    quot | (rem << 32)
}
pub extern "win64" fn putenv(_s: *const c_char) -> i32 {
    0
}
pub extern "win64" fn wgetenv(_name: *const u16) -> *const u16 {
    std::ptr::null()
}
pub unsafe extern "win64" fn wcstombs_s(
    ret: *mut usize,
    dst: *mut u8,
    dst_sz: usize,
    src: *const u16,
    count: usize,
) -> i32 {
    if src.is_null() {
        if !ret.is_null() {
            unsafe { *ret = 0 };
        }
        return 22;
    }
    let units = unsafe { wstr(src) };
    let take = if count == usize::MAX {
        units.len()
    } else {
        units.len().min(count)
    };
    let s = String::from_utf16_lossy(&units[..take]);
    let bytes = s.as_bytes();
    let needed = bytes.len() + 1;
    if dst.is_null() && dst_sz == 0 {
        if !ret.is_null() {
            unsafe { *ret = needed };
        }
        return 0;
    }
    if dst.is_null() || dst_sz == 0 {
        return 22;
    }
    if needed > dst_sz {
        if count == usize::MAX && dst_sz > 0 {
            let n = dst_sz - 1;
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, n);
                *dst.add(n) = 0;
                if !ret.is_null() {
                    *ret = dst_sz;
                }
            }
            return 80; // STRUNCATE
        }
        unsafe { *dst = 0 };
        if !ret.is_null() {
            unsafe { *ret = 0 };
        }
        return 34;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        *dst.add(bytes.len()) = 0;
        if !ret.is_null() {
            *ret = needed;
        }
    }
    0
}
// Math with float arguments.
pub extern "win64" fn fmaxf(a: f32, b: f32) -> f32 {
    a.max(b)
}
pub extern "win64" fn fminf(a: f32, b: f32) -> f32 {
    a.min(b)
}
pub extern "win64" fn powf(a: f32, b: f32) -> f32 {
    a.powf(b)
}

// ---- strings (wchar_t is 16-bit on Windows) ------------------------------------------------------
unsafe fn wstr<'a>(p: *const u16) -> &'a [u16] {
    unsafe { crate::api::text::units(p) }
}
fn lower(c: u16) -> u16 {
    char::from_u32(c as u32)
        .and_then(|ch| ch.to_lowercase().next())
        .filter(|l| (*l as u32) < 0x10000)
        .map_or(c, |l| l as u32 as u16)
}
fn upper(c: u16) -> u16 {
    char::from_u32(c as u32)
        .and_then(|ch| ch.to_uppercase().next())
        .filter(|l| (*l as u32) < 0x10000)
        .map_or(c, |l| l as u32 as u16)
}
pub extern "win64" fn towlower(c: u16) -> u16 {
    lower(c)
}
pub extern "win64" fn towupper(c: u16) -> u16 {
    upper(c)
}
pub unsafe extern "win64" fn wcslen(s: *const u16) -> usize {
    unsafe { wstr(s) }.len()
}
pub unsafe extern "win64" fn wcscmp(a: *const u16, b: *const u16) -> i32 {
    match unsafe { wstr(a) }.cmp(unsafe { wstr(b) }) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
pub unsafe extern "win64" fn wcsicmp(a: *const u16, b: *const u16) -> i32 {
    let a: Vec<u16> = unsafe { wstr(a) }.iter().map(|&c| lower(c)).collect();
    let b: Vec<u16> = unsafe { wstr(b) }.iter().map(|&c| lower(c)).collect();
    match a.cmp(&b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}
/// wcsxfrm in the C locale: a copy.
pub unsafe extern "win64" fn wcsxfrm(out: *mut u16, s: *const u16, n: usize) -> usize {
    let s = unsafe { wstr(s) };
    if !out.is_null() && n > s.len() {
        unsafe {
            std::ptr::copy_nonoverlapping(s.as_ptr(), out, s.len());
            *out.add(s.len()) = 0;
        }
    }
    s.len()
}
pub unsafe extern "win64" fn strcoll(a: *const c_char, b: *const c_char) -> i32 {
    let (a, b) = unsafe { (CStr::from_ptr(a), CStr::from_ptr(b)) };
    a.cmp(b) as i32
}
pub unsafe extern "win64" fn strxfrm(out: *mut c_char, s: *const c_char, n: usize) -> usize {
    unsafe { host_strxfrm(out, s, n) }
}
/// iswctype with the Windows ctype masks (_UPPER 1, _LOWER 2, _DIGIT 4, _SPACE 8,
/// _PUNCT 0x10, _CONTROL 0x20, _BLANK 0x40, _HEX 0x80, _ALPHA 0x100).
pub extern "win64" fn iswctype(c: u16, mask: u16) -> i32 {
    let Some(ch) = char::from_u32(c as u32) else {
        return 0;
    };
    let mut m = 0u16;
    if ch.is_uppercase() {
        m |= 1;
    }
    if ch.is_lowercase() {
        m |= 2;
    }
    if ch.is_ascii_digit() {
        m |= 4;
    }
    if ch.is_whitespace() {
        m |= 8;
    }
    if ch.is_ascii_punctuation() {
        m |= 0x10;
    }
    if ch.is_control() {
        m |= 0x20;
    }
    if ch == ' ' || ch == '\t' {
        m |= 0x40;
    }
    if ch.is_ascii_hexdigit() {
        m |= 0x80;
    }
    if ch.is_alphabetic() {
        m |= 0x100;
    }
    (m & mask != 0) as i32
}
/// mbrtowc on UTF-8 (the code page here); characters beyond the BMP give their
/// replacement.
pub unsafe extern "win64" fn mbrtowc(
    out: *mut u16,
    s: *const c_char,
    n: usize,
    _state: *mut c_void,
) -> usize {
    if s.is_null() {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(s.cast::<u8>(), n.min(4)) };
    if bytes.is_empty() {
        return usize::MAX - 1; // incomplete
    }
    if bytes[0] == 0 {
        if !out.is_null() {
            unsafe { *out = 0 };
        }
        return 0;
    }
    let length = match bytes[0] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => {
            set_errno(42);
            return usize::MAX;
        }
    };
    if bytes.len() < length {
        return usize::MAX - 1;
    }
    let Some(c) = std::str::from_utf8(&bytes[..length])
        .ok()
        .and_then(|s| s.chars().next())
    else {
        set_errno(42);
        return usize::MAX;
    };
    if !out.is_null() {
        unsafe {
            *out = if (c as u32) < 0x10000 {
                c as u32 as u16
            } else {
                0xfffd
            }
        };
    }
    length
}
pub unsafe extern "win64" fn wcrtomb(out: *mut c_char, c: u16, _state: *mut c_void) -> usize {
    let Some(ch) = char::from_u32(c as u32) else {
        set_errno(42);
        return usize::MAX;
    };
    let mut buffer = [0u8; 4];
    let s = ch.encode_utf8(&mut buffer);
    if !out.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr().cast(), out, s.len()) };
    }
    s.len()
}
/// lconv (Windows layout): ten char* fields, eight char fields, then ten wchar_t*.
pub extern "win64" fn localeconv() -> *const u8 {
    struct Lconv([u8; 168]);
    unsafe impl Send for Lconv {}
    unsafe impl Sync for Lconv {}
    static L: OnceLock<Lconv> = OnceLock::new();
    static TEXT: [&[u8]; 10] = [
        b".\0", b"\0", b"\0", b"\0", b"\0", b"\0", b"\0", b"\0", b"\0", b"\0",
    ];
    static WIDE: [[u16; 2]; 10] = [
        [b'.' as u16, 0],
        [0; 2],
        [0; 2],
        [0; 2],
        [0; 2],
        [0; 2],
        [0; 2],
        [0; 2],
        [0; 2],
        [0; 2],
    ];
    &L.get_or_init(|| {
        let mut b = [0u8; 168];
        for (i, t) in TEXT.iter().enumerate() {
            b[8 * i..8 * i + 8].copy_from_slice(&(t.as_ptr() as usize).to_le_bytes());
        }
        b[80..88].fill(127); // CHAR_MAX: not available
        for (i, w) in WIDE.iter().enumerate() {
            b[88 + 8 * i..96 + 8 * i].copy_from_slice(&(w.as_ptr() as usize).to_le_bytes());
        }
        Lconv(b)
    })
    .0 as *const u8
}

// ---- time -------------------------------------------------------------------------------------
#[repr(C)]
struct HostTm {
    fields: [i32; 9],
    gmtoff: i64,
    zone: *const c_char,
}
fn host_tm(tm: *const i32) -> HostTm {
    let mut fields = [0; 9];
    if !tm.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(tm, fields.as_mut_ptr(), 9) };
    }
    HostTm {
        fields,
        gmtoff: 0,
        zone: std::ptr::null(),
    }
}
pub unsafe extern "win64" fn strftime(
    out: *mut c_char,
    max: usize,
    format: *const c_char,
    tm: *const i32,
) -> usize {
    let t = host_tm(tm);
    unsafe { host_strftime(out, max, format, &t) }
}
pub unsafe extern "win64" fn wcsftime(
    out: *mut u16,
    max: usize,
    format: *const u16,
    tm: *const i32,
) -> usize {
    let format = String::from_utf16_lossy(unsafe { wstr(format) });
    let format = std::ffi::CString::new(format).unwrap_or_default();
    let mut buffer = vec![0 as c_char; max.max(1) * 4];
    let t = host_tm(tm);
    let n = unsafe { host_strftime(buffer.as_mut_ptr(), buffer.len(), format.as_ptr(), &t) };
    let text: Vec<u16> = String::from_utf8_lossy(unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), n)
    })
    .encode_utf16()
    .collect();
    if text.len() >= max || out.is_null() {
        return 0;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
        *out.add(text.len()) = 0;
    }
    text.len()
}
struct Zone {
    daylight: i32,
    timezone: i32,
    names: [*mut c_char; 2],
}
unsafe impl Send for Zone {}
#[export_name = "peekgame.msvcrt.dll._timezone"]
pub static CRT_TIMEZONE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
#[export_name = "peekgame.msvcrt.dll._tzname"]
pub static CRT_TZNAME: [std::sync::atomic::AtomicUsize; 2] = [
    std::sync::atomic::AtomicUsize::new(0),
    std::sync::atomic::AtomicUsize::new(0),
];
static ZONE: Mutex<Zone> = Mutex::new(Zone {
    daylight: 0,
    timezone: 0,
    names: [std::ptr::null_mut(); 2],
});
pub extern "win64" fn tzset() {
    unsafe { host_tzset() };
    let mut z = ZONE.lock().unwrap();
    unsafe {
        z.daylight = daylight;
        z.timezone = timezone as i32;
        z.names = tzname;
        CRT_TIMEZONE.store(z.timezone, std::sync::atomic::Ordering::Relaxed);
        CRT_TZNAME[0].store(z.names[0] as usize, std::sync::atomic::Ordering::Relaxed);
        CRT_TZNAME[1].store(z.names[1] as usize, std::sync::atomic::Ordering::Relaxed);
    }
}
fn zone_field<T>(f: impl FnOnce(&mut Zone) -> *mut T) -> *mut T {
    let mut z = ZONE.lock().unwrap();
    if z.names[0].is_null() {
        drop(z);
        tzset();
        z = ZONE.lock().unwrap();
    }
    f(&mut z)
}
pub extern "win64" fn p_daylight() -> *mut i32 {
    zone_field(|z| &mut z.daylight as *mut i32)
}
pub extern "win64" fn p_timezone() -> *mut i32 {
    {
        zone_field(|z| &mut z.timezone as *mut i32);
        CRT_TIMEZONE.as_ptr()
    }
}
pub extern "win64" fn p_tzname() -> *mut *mut c_char {
    {
        zone_field(|z| z.names.as_mut_ptr());
        CRT_TZNAME.as_ptr().cast_mut().cast()
    }
}

// ---- setjmp/longjmp (_JUMP_BUFFER layout) ------------------------------------------------------
std::arch::global_asm!(
    ".text",
    ".p2align 4",
    ".globl peekgame_crt_setjmp",
    "peekgame_crt_setjmp:",
    "mov %rdx, 0x00(%rcx)",
    "mov %rbx, 0x08(%rcx)",
    "lea 8(%rsp), %rax",
    "mov %rax, 0x10(%rcx)",
    "mov %rbp, 0x18(%rcx)",
    "mov %rsi, 0x20(%rcx)",
    "mov %rdi, 0x28(%rcx)",
    "mov %r12, 0x30(%rcx)",
    "mov %r13, 0x38(%rcx)",
    "mov %r14, 0x40(%rcx)",
    "mov %r15, 0x48(%rcx)",
    "mov (%rsp), %rax",
    "mov %rax, 0x50(%rcx)",
    "stmxcsr 0x58(%rcx)",
    "fnstcw 0x5c(%rcx)",
    "movdqu %xmm6, 0x60(%rcx)",
    "movdqu %xmm7, 0x70(%rcx)",
    "movdqu %xmm8, 0x80(%rcx)",
    "movdqu %xmm9, 0x90(%rcx)",
    "movdqu %xmm10, 0xa0(%rcx)",
    "movdqu %xmm11, 0xb0(%rcx)",
    "movdqu %xmm12, 0xc0(%rcx)",
    "movdqu %xmm13, 0xd0(%rcx)",
    "movdqu %xmm14, 0xe0(%rcx)",
    "movdqu %xmm15, 0xf0(%rcx)",
    "xor %eax, %eax",
    "ret",
    ".p2align 4",
    ".globl peekgame_crt_longjmp",
    "peekgame_crt_longjmp:",
    "mov 0x08(%rcx), %rbx",
    "mov 0x18(%rcx), %rbp",
    "mov 0x20(%rcx), %rsi",
    "mov 0x28(%rcx), %rdi",
    "mov 0x30(%rcx), %r12",
    "mov 0x38(%rcx), %r13",
    "mov 0x40(%rcx), %r14",
    "mov 0x48(%rcx), %r15",
    "ldmxcsr 0x58(%rcx)",
    "fldcw 0x5c(%rcx)",
    "movdqu 0x60(%rcx), %xmm6",
    "movdqu 0x70(%rcx), %xmm7",
    "movdqu 0x80(%rcx), %xmm8",
    "movdqu 0x90(%rcx), %xmm9",
    "movdqu 0xa0(%rcx), %xmm10",
    "movdqu 0xb0(%rcx), %xmm11",
    "movdqu 0xc0(%rcx), %xmm12",
    "movdqu 0xd0(%rcx), %xmm13",
    "movdqu 0xe0(%rcx), %xmm14",
    "movdqu 0xf0(%rcx), %xmm15",
    "mov %edx, %eax",
    "test %eax, %eax",
    "jnz 1f",
    "mov $1, %eax",
    "1:",
    "mov 0x10(%rcx), %rsp",
    "jmp *0x50(%rcx)",
    options(att_syntax)
);
unsafe extern "win64" {
    #[link_name = "peekgame_crt_setjmp"]
    pub fn setjmp(buffer: *mut u8, frame: usize) -> i32;
    #[link_name = "peekgame_crt_longjmp"]
    pub fn longjmp(buffer: *mut u8, value: i32) -> !;
}

pub(super) unsafe fn render_narrow(fmt: *const c_char, args: *const u64) -> String {
    self::format(&narrow_spec(fmt), &mut Args(args), false)
}
pub(super) unsafe fn render_wide(fmt: *const u16, args: *const u64) -> String {
    self::format(&wide_spec(fmt), &mut Args(args), true)
}
