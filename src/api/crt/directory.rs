//! CRT directory operations share the runtime's Windows-to-host path mapping.
use super::stdio;
fn failed(code: i32) -> i32 {
    unsafe {
        *stdio::errno() = code;
    }
    -1
}
pub unsafe extern "win64" fn change_w(path: *const u16) -> i32 {
    let text = match unsafe { crate::host::paths::wide(path) } {
        Ok(text) => text,
        Err(1113) => return failed(42), // Windows EILSEQ (Linux uses a different value).
        Err(_) => return failed(22),
    };
    let path = match crate::host::paths::host(&text) {
        Ok(path) => path,
        Err(_) => return failed(2),
    };
    match std::env::set_current_dir(path) {
        Ok(()) => 0,
        Err(e) => failed(match e.raw_os_error() {
            Some(13 | 1) => 13, // EACCES
            Some(2) => 2,       // ENOENT
            Some(20) => 20,     // ENOTDIR
            Some(12) => 12,     // ENOMEM
            Some(36) => 38,     // Windows ENAMETOOLONG
            _ => 22,
        }),
    }
}

pub unsafe extern "win64" fn getcwd_w(out: *mut u16, capacity: i32) -> *mut u16 {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            failed(e.raw_os_error().unwrap_or(22));
            return std::ptr::null_mut();
        }
    };
    let path = crate::host::paths::windows(&cwd);
    let mut wide: Vec<u16> = path.encode_utf16().collect();
    wide.push(0);
    if !out.is_null() && (capacity <= 0 || (capacity as usize) < wide.len()) {
        failed(if capacity <= 0 { 22 } else { 34 });
        return std::ptr::null_mut();
    }
    let out = if out.is_null() {
        if capacity < 0 {
            failed(22);
            return std::ptr::null_mut();
        }
        let bytes = (capacity as usize).max(wide.len()) * 2;
        let out = unsafe { super::simple::call_0(bytes) }.cast::<u16>();
        if out.is_null() {
            failed(12);
            return out;
        }
        out
    } else {
        out
    };
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), out, wide.len());
    }
    out
}
