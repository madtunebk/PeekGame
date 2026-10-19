//! C-locale character classification and bounded narrow/wide strings.
pub unsafe extern "win64" fn mbscmp(a: *const u8, b: *const u8) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    let mut index = 0;
    loop {
        let left = unsafe { *a.add(index) };
        let right = unsafe { *b.add(index) };
        if left != right || left == 0 {
            return left as i32 - right as i32;
        }
        index += 1;
    }
}
use std::ffi::{c_char, CStr};
fn err(code: i32) -> i32 {
    unsafe {
        *super::stdio::errno() = code;
    }
    code
}
fn ascii(c: i32) -> Option<u8> {
    (0..128).contains(&c).then_some(c as u8)
}
pub extern "win64" fn isascii(c: i32) -> i32 {
    (0..=127).contains(&c) as i32
}
/// The supported CRT C locale is single-byte, so multibyte length equals byte length.
pub unsafe extern "win64" fn mbstrlen(text: *const c_char) -> usize {
    if text.is_null() {
        err(22);
        return usize::MAX;
    }
    unsafe { CStr::from_ptr(text) }.to_bytes().len()
}
pub extern "win64" fn isalnum(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_alphanumeric()) as i32
}
pub extern "win64" fn isalpha(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_alphabetic()) as i32
}
pub extern "win64" fn isdigit(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_digit()) as i32
}
pub extern "win64" fn isspace(c: i32) -> i32 {
    ascii(c).is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12)) as i32
}
pub extern "win64" fn isxdigit(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_hexdigit()) as i32
}
pub extern "win64" fn isprint(c: i32) -> i32 {
    ascii(c).is_some_and(|c| (0x20..=0x7e).contains(&c)) as i32
}
pub extern "win64" fn iswprint(c: u16) -> i32 {
    (c >= 0x20 && c != 0x7f) as i32
}
pub extern "win64" fn iswalnum(c: u16) -> i32 {
    isalnum(c as i32)
}
pub extern "win64" fn iswalpha(c: u16) -> i32 {
    isalpha(c as i32)
}
pub extern "win64" fn iswdigit(c: u16) -> i32 {
    isdigit(c as i32)
}
pub extern "win64" fn iswxdigit(c: u16) -> i32 {
    isxdigit(c as i32)
}
pub extern "win64" fn iswspace(c: u16) -> i32 {
    isspace(c as i32)
}
pub extern "win64" fn tolower(c: i32) -> i32 {
    ascii(c).map_or(c, |c| c.to_ascii_lowercase() as i32)
}
pub extern "win64" fn toupper(c: i32) -> i32 {
    ascii(c).map_or(c, |c| c.to_ascii_uppercase() as i32)
}
pub unsafe extern "win64" fn memicmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let d = tolower(unsafe { *a.add(i) } as i32) - tolower(unsafe { *b.add(i) } as i32);
        if d != 0 {
            return d;
        }
    }
    0
}
pub unsafe extern "win64" fn stricmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let (x, y) = unsafe { (*a.add(i), *b.add(i)) };
        let d = tolower(x as i32) - tolower(y as i32);
        if d != 0 || x == 0 {
            return d;
        }
        i += 1;
    }
}
pub unsafe extern "win64" fn stricmp_l(a: *const u8, b: *const u8, _locale: usize) -> i32 {
    unsafe { stricmp(a, b) }
}
pub unsafe extern "win64" fn strnicmp(a: *const u8, b: *const u8, count: usize) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    for i in 0..count {
        let (x, y) = unsafe { (*a.add(i), *b.add(i)) };
        let d = tolower(x as i32) - tolower(y as i32);
        if d != 0 || x == 0 {
            return d;
        }
    }
    0
}
pub unsafe extern "win64" fn strnicmp_l(a: *const u8, b: *const u8, count: usize, _locale: usize) -> i32 {
    unsafe { strnicmp(a, b, count) }
}

unsafe extern "C" {
    fn strspn(s: *const c_char, accept: *const c_char) -> usize;
}

pub unsafe extern "win64" fn crt_strspn(s: *const c_char, accept: *const c_char) -> usize {
    if s.is_null() || accept.is_null() {
        return 0;
    }
    unsafe { strspn(s, accept) }
}

pub unsafe extern "win64" fn wcscpy(out: *mut u16, input: *const u16) -> *mut u16 {
    let mut i = 0;
    loop {
        let c = unsafe { *input.add(i) };
        unsafe {
            *out.add(i) = c;
        }
        if c == 0 {
            break;
        }
        i += 1;
    }
    out
}
pub unsafe extern "win64" fn strcpy_s(out: *mut u8, n: usize, input: *const c_char) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    if input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let bytes = unsafe { CStr::from_ptr(input) }.to_bytes_with_nul();
    if bytes.len() > n {
        unsafe {
            *out = 0;
        }
        return err(34);
    }
    unsafe {
        std::ptr::copy(bytes.as_ptr(), out, bytes.len());
    }
    0
}
pub unsafe extern "win64" fn strncpy_s(
    out: *mut c_char,
    dest_size: usize,
    input: *const c_char,
    count: usize,
) -> i32 {
    if out.is_null() || dest_size == 0 {
        return err(22);
    }
    if input.is_null() {
        unsafe { *out = 0 };
        return err(22);
    }
    if count == 0 {
        unsafe { *out = 0 };
        return 0;
    }
    let truncate = count == usize::MAX;
    let mut i = 0;
    while (!truncate && i < count && i < dest_size) || (truncate && i < dest_size - 1) {
        let c = unsafe { *input.add(i) };
        if c == 0 {
            break;
        }
        unsafe { *out.add(i) = c };
        i += 1;
    }
    if i < dest_size {
        unsafe { *out.add(i) = 0 };
        0
    } else {
        unsafe { *out = 0 };
        err(34)
    }
}
pub unsafe extern "win64" fn memcpy_s(
    dest: *mut u8,
    dest_size: usize,
    src: *const u8,
    count: usize,
) -> i32 {
    if count == 0 {
        return 0;
    }
    if dest.is_null() || src.is_null() {
        if !dest.is_null() && dest_size != 0 {
            unsafe { std::ptr::write_bytes(dest, 0, dest_size) };
        }
        return err(22);
    }
    if count > dest_size {
        unsafe { std::ptr::write_bytes(dest, 0, dest_size) };
        return err(34);
    }
    unsafe {
        std::ptr::copy(src, dest, count);
    }
    0
}
pub unsafe extern "win64" fn strcat_s(out: *mut u8, n: usize, input: *const c_char) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    let mut len = 0;
    while len < n && unsafe { *out.add(len) } != 0 {
        len += 1;
    }
    if len == n || input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let r = unsafe { strcpy_s(out.add(len), n - len, input) };
    if r != 0 {
        unsafe {
            *out = 0;
        }
    }
    r
}
pub unsafe extern "win64" fn strncat_s(
    out: *mut u8,
    dest_size: usize,
    input: *const c_char,
    count: usize,
) -> i32 {
    if out.is_null() || dest_size == 0 {
        return err(22);
    }
    let mut cur_len = 0;
    while cur_len < dest_size && unsafe { *out.add(cur_len) } != 0 {
        cur_len += 1;
    }
    if cur_len == dest_size {
        unsafe { *out = 0 };
        return err(22);
    }
    if input.is_null() {
        if count == 0 {
            return 0;
        }
        unsafe { *out = 0 };
        return err(22);
    }
    if count == 0 {
        return 0;
    }

    let truncate = count == usize::MAX;
    let available = dest_size - cur_len;
    let mut src_len = 0;
    while (!truncate && src_len < count) || (truncate && unsafe { *input.add(src_len) } != 0) {
        if unsafe { *input.add(src_len) } == 0 {
            break;
        }
        src_len += 1;
    }

    if !truncate {
        if src_len >= available {
            unsafe { *out = 0 };
            return err(34);
        }
        for j in 0..src_len {
            unsafe { *out.add(cur_len + j) = *input.add(j) as u8 };
        }
        unsafe { *out.add(cur_len + src_len) = 0 };
        0
    } else {
        let to_copy = src_len.min(available.saturating_sub(1));
        for j in 0..to_copy {
            unsafe { *out.add(cur_len + j) = *input.add(j) as u8 };
        }
        unsafe { *out.add(cur_len + to_copy) = 0 };
        if to_copy < src_len {
            80
        } else {
            0
        }
    }
}
pub unsafe extern "win64" fn wcscat(out: *mut u16, input: *const u16) -> *mut u16 {
    let mut p = out;
    while unsafe { *p } != 0 {
        p = unsafe { p.add(1) };
    }
    let mut q = input;
    while unsafe { *q } != 0 {
        unsafe {
            *p = *q;
            p = p.add(1);
            q = q.add(1);
        }
    }
    unsafe { *p = 0 };
    out
}
pub unsafe extern "win64" fn wcsncat(out: *mut u16, input: *const u16, n: usize) -> *mut u16 {
    let mut p = out;
    while unsafe { *p } != 0 {
        p = unsafe { p.add(1) };
    }
    let mut i = 0;
    while i < n && unsafe { *input.add(i) } != 0 {
        unsafe { *p.add(i) = *input.add(i) };
        i += 1;
    }
    unsafe { *p.add(i) = 0 };
    out
}
pub unsafe extern "win64" fn wcscat_s(out: *mut u16, n: usize, input: *const u16) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    let mut len = 0;
    while len < n && unsafe { *out.add(len) } != 0 {
        len += 1;
    }
    if len == n || input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let r = unsafe { wcscpy_s(out.add(len), n - len, input) };
    if r != 0 {
        unsafe {
            *out = 0;
        }
    }
    r
}
pub unsafe extern "win64" fn wcsncat_s(
    out: *mut u16,
    dest_size: usize,
    input: *const u16,
    count: usize,
) -> i32 {
    if out.is_null() || dest_size == 0 {
        return err(22);
    }
    let mut cur_len = 0;
    while cur_len < dest_size && unsafe { *out.add(cur_len) } != 0 {
        cur_len += 1;
    }
    if cur_len == dest_size {
        unsafe { *out = 0 };
        return err(22);
    }
    if input.is_null() {
        if count == 0 {
            return 0;
        }
        unsafe { *out = 0 };
        return err(22);
    }
    if count == 0 {
        return 0;
    }

    let truncate = count == usize::MAX;
    let available = dest_size - cur_len;
    let mut src_len = 0;
    while (!truncate && src_len < count) || (truncate && unsafe { *input.add(src_len) } != 0) {
        if unsafe { *input.add(src_len) } == 0 {
            break;
        }
        src_len += 1;
    }

    if !truncate {
        if src_len >= available {
            unsafe { *out = 0 };
            return err(34);
        }
        for j in 0..src_len {
            unsafe { *out.add(cur_len + j) = *input.add(j) };
        }
        unsafe { *out.add(cur_len + src_len) = 0 };
        0
    } else {
        let to_copy = src_len.min(available.saturating_sub(1));
        for j in 0..to_copy {
            unsafe { *out.add(cur_len + j) = *input.add(j) };
        }
        unsafe { *out.add(cur_len + to_copy) = 0 };
        if to_copy < src_len {
            80
        } else {
            0
        }
    }
}
pub unsafe extern "win64" fn wcscpy_s(out: *mut u16, n: usize, input: *const u16) -> i32 {
    if out.is_null() || n == 0 {
        return err(22);
    }
    if input.is_null() {
        unsafe {
            *out = 0;
        }
        return err(22);
    }
    let len = unsafe { crate::api::text::units(input) }.len();
    if len >= n {
        unsafe {
            *out = 0;
        }
        return err(34);
    }
    unsafe {
        std::ptr::copy(input, out, len + 1);
    }
    0
}

pub unsafe extern "win64" fn wcsncpy(out: *mut u16, input: *const u16, n: usize) -> *mut u16 {
    let mut i = 0;
    while i < n {
        let c = unsafe { *input.add(i) };
        if c == 0 {
            while i < n {
                unsafe { *out.add(i) = 0 };
                i += 1;
            }
            break;
        }
        unsafe { *out.add(i) = c };
        i += 1;
    }
    out
}

pub unsafe extern "win64" fn wcsncpy_s(out: *mut u16, dest_size: usize, input: *const u16, count: usize) -> i32 {
    if out.is_null() || dest_size == 0 {
        return err(22);
    }
    if input.is_null() {
        unsafe { *out = 0 };
        return err(22);
    }
    if count == 0 {
        unsafe { *out = 0 };
        return 0;
    }
    let mut i = 0;
    while i < count && i < dest_size {
        let c = unsafe { *input.add(i) };
        if c == 0 {
            break;
        }
        unsafe { *out.add(i) = c };
        i += 1;
    }
    if i < dest_size {
        unsafe { *out.add(i) = 0 };
        0
    } else {
        unsafe { *out = 0 };
        err(34)
    }
}

pub unsafe extern "win64" fn wcsncmp(a: *const u16, b: *const u16, n: usize) -> i32 {
    for i in 0..n {
        let x = unsafe { *a.add(i) };
        let y = unsafe { *b.add(i) };
        if x != y || x == 0 {
            return (x as i32) - (y as i32);
        }
    }
    0
}

pub unsafe extern "win64" fn wcsnicmp(a: *const u16, b: *const u16, n: usize) -> i32 {
    for i in 0..n {
        let x = unsafe { *a.add(i) };
        let y = unsafe { *b.add(i) };
        let dx = if (b'A' as u16..=b'Z' as u16).contains(&x) { x + 32 } else { x };
        let dy = if (b'A' as u16..=b'Z' as u16).contains(&y) { y + 32 } else { y };
        if dx != dy || x == 0 {
            return (dx as i32) - (dy as i32);
        }
    }
    0
}

pub unsafe extern "win64" fn mbschr(s: *const u8, c: u32) -> *mut u8 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let target = (c & 0xff) as u8;
    let mut p = s;
    loop {
        let b = unsafe { *p };
        if b == target {
            return p as *mut u8;
        }
        if b == 0 {
            return std::ptr::null_mut();
        }
        p = unsafe { p.add(1) };
    }
}

pub unsafe extern "win64" fn mbsnbcpy(dest: *mut u8, src: *const u8, count: usize) -> *mut u8 {
    if dest.is_null() || src.is_null() || count == 0 {
        return dest;
    }
    let mut i = 0;
    while i < count {
        let b = unsafe { *src.add(i) };
        unsafe { *dest.add(i) = b };
        if b == 0 {
            break;
        }
        i += 1;
    }
    while i < count {
        unsafe { *dest.add(i) = 0 };
        i += 1;
    }
    dest
}

pub unsafe extern "win64" fn mbsnbcmp(s1: *const u8, s2: *const u8, count: usize) -> i32 {
    if s1.is_null() || s2.is_null() || count == 0 {
        return 0;
    }
    for i in 0..count {
        let x = unsafe { *s1.add(i) };
        let y = unsafe { *s2.add(i) };
        if x != y || x == 0 {
            return (x as i32) - (y as i32);
        }
    }
    0
}

pub unsafe extern "win64" fn mbspbrk(s: *const u8, accept: *const u8) -> *mut u8 {
    if s.is_null() || accept.is_null() {
        return std::ptr::null_mut();
    }
    let mut p = s;
    while unsafe { *p } != 0 {
        let b = unsafe { *p };
        let mut a = accept;
        while unsafe { *a } != 0 {
            if unsafe { *a } == b {
                return p as *mut u8;
            }
            a = unsafe { a.add(1) };
        }
        p = unsafe { p.add(1) };
    }
    std::ptr::null_mut()
}
pub extern "win64" fn iscntrl(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_control()) as i32
}
pub extern "win64" fn ispunct(c: i32) -> i32 {
    ascii(c).is_some_and(|c| c.is_ascii_punctuation()) as i32
}
pub extern "win64" fn iswcntrl(c: u16) -> i32 {
    iscntrl(c as i32)
}
pub extern "win64" fn iswpunct(c: u16) -> i32 {
    ispunct(c as i32)
}
pub extern "win64" fn iswlower(c: u16) -> i32 {
    ascii(c as i32).is_some_and(|c| c.is_ascii_lowercase()) as i32
}
pub extern "win64" fn iswupper(c: u16) -> i32 {
    ascii(c as i32).is_some_and(|c| c.is_ascii_uppercase()) as i32
}
pub unsafe extern "win64" fn wcschr(s: *const u16, c: u16) -> *mut u16 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let mut p = s;
    loop {
        let u = unsafe { *p };
        if u == c {
            return p as *mut u16;
        }
        if u == 0 {
            return std::ptr::null_mut();
        }
        p = unsafe { p.add(1) };
    }
}
pub unsafe extern "win64" fn wcsrchr(s: *const u16, c: u16) -> *mut u16 {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    let mut p = s;
    let mut last = std::ptr::null_mut();
    loop {
        let u = unsafe { *p };
        if u == c {
            last = p as *mut u16;
        }
        if u == 0 {
            return last;
        }
        p = unsafe { p.add(1) };
    }
}
pub unsafe extern "win64" fn wcsstr(haystack: *const u16, needle: *const u16) -> *mut u16 {
    if haystack.is_null() || needle.is_null() {
        return std::ptr::null_mut();
    }
    if unsafe { *needle } == 0 {
        return haystack as *mut u16;
    }
    let mut h = haystack;
    while unsafe { *h } != 0 {
        let mut i = 0;
        loop {
            let n = unsafe { *needle.add(i) };
            if n == 0 {
                return h as *mut u16;
            }
            if unsafe { *h.add(i) } != n {
                break;
            }
            i += 1;
        }
        h = unsafe { h.add(1) };
    }
    std::ptr::null_mut()
}
pub unsafe extern "win64" fn wcsupr(s: *mut u16) -> *mut u16 {
    if s.is_null() {
        return s;
    }
    let mut p = s;
    while unsafe { *p } != 0 {
        let u = unsafe { *p };
        if (b'a' as u16..=b'z' as u16).contains(&u) {
            unsafe { *p = u - 32 };
        }
        p = unsafe { p.add(1) };
    }
    s
}
pub unsafe extern "win64" fn wcslwr(s: *mut u16) -> *mut u16 {
    if s.is_null() {
        return s;
    }
    let mut p = s;
    while unsafe { *p } != 0 {
        let u = unsafe { *p };
        if (b'A' as u16..=b'Z' as u16).contains(&u) {
            unsafe { *p = u + 32 };
        }
        p = unsafe { p.add(1) };
    }
    s
}


