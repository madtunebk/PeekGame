//! Extra FILE/descriptor helpers. Windows FILE pointers always go through stdio.
use super::stdio;
use std::ffi::{c_char, c_void, CStr, CString};
unsafe extern "C" {
    #[link_name = "fgets"]
    fn host_fgets(s: *mut c_char, n: i32, f: *mut c_void) -> *mut c_char;
    #[link_name = "rewind"]
    fn host_rewind(f: *mut c_void);
    #[link_name = "freopen"]
    fn host_freopen(path: *const c_char, mode: *const c_char, f: *mut c_void) -> *mut c_void;
    fn ftruncate(fd: i32, size: i64) -> i32;
}
pub unsafe extern "win64" fn fgets(out: *mut c_char, n: i32, file: *mut c_void) -> *mut c_char {
    if out.is_null() || n <= 0 {
        unsafe {
            *stdio::errno() = 22;
        }
        return std::ptr::null_mut();
    }
    unsafe { host_fgets(out, n, stdio::host(file)) }
}
pub extern "win64" fn rewind(file: *mut c_void) {
    unsafe {
        host_rewind(stdio::host(file));
    }
}
pub unsafe extern "win64" fn puts(s: *const c_char) -> i32 {
    let f = super::ucrt::acrt_iob_func(1);
    if unsafe { super::ucrt::fputs(s, f) } < 0 {
        return -1;
    }
    unsafe { super::ucrt::fputc(10, f) }
}
pub unsafe extern "win64" fn cputs(s: *const c_char) -> i32 {
    let f = super::ucrt::acrt_iob_func(1);
    if unsafe { super::ucrt::fputs(s, f) } < 0 {
        -1
    } else {
        0
    }
}
pub extern "win64" fn putchar(c: i32) -> i32 {
    unsafe { super::ucrt::fputc(c, super::ucrt::acrt_iob_func(1)) }
}
pub unsafe extern "win64" fn perror(prefix: *const c_char) {
    let code = unsafe { *stdio::errno() };
    let message = super::ucrt::strerror(code);
    let f = super::ucrt::acrt_iob_func(2);
    if !prefix.is_null() && !unsafe { CStr::from_ptr(prefix) }.is_empty() {
        unsafe {
            super::ucrt::fputs(prefix, f);
            super::ucrt::fputs(c": ".as_ptr(), f);
        }
    }
    unsafe {
        super::ucrt::fputs(message, f);
        super::ucrt::fputc(10, f);
        *stdio::errno() = code;
    }
}
pub extern "win64" fn chsize(fd: i32, size: i32) -> i32 {
    if size < 0 {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    unsafe { ftruncate(fd, size as i64) }
}
pub unsafe extern "win64" fn fstat(fd: i32, out: *mut u8) -> i32 {
    let mut full = [0u8; 56];
    if out.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    if unsafe { super::ucrt::fstat64(fd, full.as_mut_ptr()) } != 0 {
        return -1;
    }
    let size = i64::from_ne_bytes(full[24..32].try_into().unwrap());
    if size > i32::MAX as i64 {
        unsafe {
            *stdio::errno() = 132;
        }
        return -1;
    } // Windows EOVERFLOW
    unsafe {
        out.write_bytes(0, 48);
        std::ptr::copy_nonoverlapping(full.as_ptr(), out, 20);
        out.add(20).cast::<i32>().write_unaligned(size as i32);
    }
    for (from, to) in [(32, 24), (40, 32), (48, 40)] {
        unsafe {
            std::ptr::copy_nonoverlapping(full.as_ptr().add(from), out.add(to), 8);
        }
    }
    0
}
pub unsafe extern "win64" fn freopen_s(
    out: *mut *mut c_void,
    path: *const c_char,
    mode: *const c_char,
    file: *mut c_void,
) -> i32 {
    if out.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return 22;
    }
    unsafe {
        *out = std::ptr::null_mut();
    }
    if path.is_null() || mode.is_null() || file.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return 22;
    }
    let path = match unsafe { crate::host::paths::narrow(path) }
        .and_then(|p| crate::host::paths::host(&p))
    {
        Ok(p) => p,
        Err(_) => {
            unsafe {
                *stdio::errno() = 2;
            }
            return 2;
        }
    };
    let mode = unsafe { CStr::from_ptr(mode) }.to_bytes();
    if mode.is_empty() || !mode.iter().all(|b| b"rwa+btx".contains(b)) {
        unsafe {
            *stdio::errno() = 22;
        }
        return 22;
    }
    use std::os::unix::ffi::OsStrExt;
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let mode = CString::new(
        mode.iter()
            .copied()
            .filter(|&c| c != b't')
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let host = unsafe { host_freopen(path.as_ptr(), mode.as_ptr(), stdio::host(file)) };
    if host.is_null() {
        return unsafe { *stdio::errno() };
    }
    stdio::replace_host(file, host);
    unsafe {
        *out = file;
    }
    0
}
