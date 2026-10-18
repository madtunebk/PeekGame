//! Windows long remains 32 bits even on x64; parsing uses host 64-bit routines.
use std::ffi::{c_char, CString};
unsafe extern "C" {
    #[link_name = "strtoll"]
    fn signed(s: *const c_char, end: *mut *mut c_char, base: i32) -> i64;
    #[link_name = "strtoull"]
    fn unsigned(s: *const c_char, end: *mut *mut c_char, base: i32) -> u64;
}
fn errno(code: i32) {
    unsafe {
        *super::stdio::errno() = code;
    }
}
pub unsafe extern "win64" fn strtoi64(s: *const c_char, end: *mut *mut c_char, base: i32) -> i64 {
    if s.is_null() || !(base == 0 || (2..=36).contains(&base)) {
        errno(22);
        if !end.is_null() {
            unsafe {
                *end = s.cast_mut();
            }
        }
        return 0;
    }
    unsafe { signed(s, end, base) }
}
pub unsafe extern "win64" fn strtoui64(s: *const c_char, end: *mut *mut c_char, base: i32) -> u64 {
    if s.is_null() || !(base == 0 || (2..=36).contains(&base)) {
        errno(22);
        if !end.is_null() {
            unsafe {
                *end = s.cast_mut();
            }
        }
        return 0;
    }
    unsafe { unsigned(s, end, base) }
}
pub unsafe extern "win64" fn strtoll(s: *const c_char, end: *mut *mut c_char, base: i32) -> i64 {
    unsafe { strtoi64(s, end, base) }
}
pub unsafe extern "win64" fn strtoull(s: *const c_char, end: *mut *mut c_char, base: i32) -> u64 {
    unsafe { strtoui64(s, end, base) }
}
pub unsafe extern "win64" fn atoll(s: *const c_char) -> i64 {
    unsafe { strtoi64(s, std::ptr::null_mut(), 10) }
}
pub unsafe extern "win64" fn strtol(s: *const c_char, end: *mut *mut c_char, base: i32) -> i32 {
    let v = unsafe { strtoi64(s, end, base) };
    if v > i32::MAX as i64 {
        errno(34);
        i32::MAX
    } else if v < i32::MIN as i64 {
        errno(34);
        i32::MIN
    } else {
        v as i32
    }
}
pub unsafe extern "win64" fn wcstol(s: *const u16, end: *mut *const u16, base: i32) -> i32 {
    if s.is_null() {
        errno(22);
        return 0;
    }
    // Numeric syntax in the C locale is ASCII; the first non-ASCII unit ends it.
    let bytes: Vec<u8> = unsafe { crate::api::text::units(s) }
        .iter()
        .take_while(|&&c| c < 128)
        .map(|&c| c as u8)
        .collect();
    let bytes = CString::new(bytes).unwrap();
    let mut finish = std::ptr::null_mut();
    let v = unsafe { strtol(bytes.as_ptr(), &mut finish, base) };
    if !end.is_null() {
        let n = unsafe { finish.offset_from(bytes.as_ptr()) };
        unsafe {
            *end = s.offset(n);
        }
    }
    v
}

pub unsafe extern "win64" fn wcstoi64(s: *const u16, end: *mut *const u16, base: i32) -> i64 {
    if s.is_null() {
        errno(22);
        return 0;
    }
    let bytes: Vec<u8> = unsafe { crate::api::text::units(s) }
        .iter()
        .take_while(|&&c| c < 128)
        .map(|&c| c as u8)
        .collect();
    let bytes = CString::new(bytes).unwrap_or_default();
    let mut finish = std::ptr::null_mut();
    let v = unsafe { strtoi64(bytes.as_ptr(), &mut finish, base) };
    if !end.is_null() {
        let n = unsafe { finish.offset_from(bytes.as_ptr()) };
        unsafe {
            *end = s.offset(n);
        }
    }
    v
}

pub unsafe extern "win64" fn wcstoui64(s: *const u16, end: *mut *const u16, base: i32) -> u64 {
    if s.is_null() {
        errno(22);
        return 0;
    }
    let bytes: Vec<u8> = unsafe { crate::api::text::units(s) }
        .iter()
        .take_while(|&&c| c < 128)
        .map(|&c| c as u8)
        .collect();
    let bytes = CString::new(bytes).unwrap_or_default();
    let mut finish = std::ptr::null_mut();
    let v = unsafe { strtoui64(bytes.as_ptr(), &mut finish, base) };
    if !end.is_null() {
        let n = unsafe { finish.offset_from(bytes.as_ptr()) };
        unsafe {
            *end = s.offset(n);
        }
    }
    v
}

pub unsafe extern "win64" fn wcstoul(s: *const u16, end: *mut *const u16, base: i32) -> u32 {
    let v = unsafe { wcstoui64(s, end, base) };
    if v > u32::MAX as u64 {
        errno(34);
        u32::MAX
    } else {
        v as u32
    }
}

pub unsafe extern "win64" fn wcstol_l(s: *const u16, end: *mut *const u16, base: i32, _locale: usize) -> i32 {
    unsafe { wcstol(s, end, base) }
}

pub unsafe extern "win64" fn wcstoul_l(s: *const u16, end: *mut *const u16, base: i32, _locale: usize) -> u32 {
    unsafe { wcstoul(s, end, base) }
}

pub unsafe extern "win64" fn wcstoi64_l(s: *const u16, end: *mut *const u16, base: i32, _locale: usize) -> i64 {
    unsafe { wcstoi64(s, end, base) }
}

pub unsafe extern "win64" fn wcstoui64_l(s: *const u16, end: *mut *const u16, base: i32, _locale: usize) -> u64 {
    unsafe { wcstoui64(s, end, base) }
}

pub unsafe extern "win64" fn strtol_l(s: *const c_char, end: *mut *mut c_char, base: i32, _locale: usize) -> i32 {
    unsafe { strtol(s, end, base) }
}

pub unsafe extern "win64" fn strtoi64_l(s: *const c_char, end: *mut *mut c_char, base: i32, _locale: usize) -> i64 {
    unsafe { strtoi64(s, end, base) }
}

pub unsafe extern "win64" fn strtoui64_l(s: *const c_char, end: *mut *mut c_char, base: i32, _locale: usize) -> u64 {
    unsafe { strtoui64(s, end, base) }
}

pub unsafe extern "win64" fn itoa_s(value: i32, out: *mut u8, size: usize, radix: i32) -> i32 {
    if out.is_null() || size == 0 {
        errno(22);
        return 22;
    }
    unsafe {
        *out = 0;
    }
    if !(2..=36).contains(&radix) {
        errno(22);
        return 22;
    }
    let negative = radix == 10 && value < 0;
    let mut v = if negative {
        -(value as i64) as u64
    } else {
        value as u32 as u64
    };
    let mut digits = Vec::new();
    loop {
        let d = (v % radix as u64) as u8;
        digits.push(if d < 10 { b'0' + d } else { b'a' + d - 10 });
        v /= radix as u64;
        if v == 0 {
            break;
        }
    }
    if negative {
        digits.push(b'-');
    }
    digits.reverse();
    if digits.len() >= size {
        errno(34);
        return 34;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(digits.as_ptr(), out, digits.len());
        *out.add(digits.len()) = 0;
    }
    0
}
// Deliberately use Microsoft's per-thread 15-bit generator, not glibc rand().
thread_local! {static RANDOM:std::cell::Cell<u32>=const {std::cell::Cell::new(1)};}
pub extern "win64" fn srand(seed: u32) {
    RANDOM.with(|r| r.set(seed));
}
pub extern "win64" fn rand() -> i32 {
    RANDOM.with(|r| {
        let n = r.get().wrapping_mul(214013).wrapping_add(2531011);
        r.set(n);
        ((n >> 16) & 0x7fff) as i32
    })
}
