use std::{cell::UnsafeCell, ffi::c_void, sync::Once};
struct Iob(UnsafeCell<[u64; 18]>);
// Windows FILE layouts contain no host FILE pointers. Initialization is once-only.
unsafe impl Sync for Iob {}
static IOB: Iob = Iob(UnsafeCell::new([0; 18]));
static INIT: Once = Once::new();
unsafe extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;
    static mut stderr: *mut c_void;
    fn setbuf(file: *mut c_void, buffer: *mut u8);
    fn fflush(file: *mut c_void) -> i32;
    fn fileno(file: *mut c_void) -> i32;
}
pub extern "win64" fn iob() -> *mut c_void {
    INIT.call_once(|| {
        for index in 0..3 {
            unsafe {
                IOB.0
                    .get()
                    .cast::<u8>()
                    .add(index * 48 + 24)
                    .cast::<i32>()
                    .write(index as i32);
            }
        }
    });
    IOB.0.get().cast()
}
struct Stream {
    _layout: Box<[u64; 6]>,
    host: usize,
}
static STREAMS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<usize, Stream>>> =
    std::sync::OnceLock::new();
fn streams() -> &'static std::sync::Mutex<std::collections::HashMap<usize, Stream>> {
    STREAMS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
fn host(file: *mut c_void) -> *mut c_void {
    let base = iob() as usize;
    let value = file as usize;
    let index = value
        .checked_sub(base)
        .filter(|offset| offset % 48 == 0)
        .map(|offset| offset / 48);
    match index {
        Some(0) => unsafe { stdin },
        Some(1) => unsafe { stdout },
        Some(2) => unsafe { stderr },
        _ if streams().lock().unwrap().contains_key(&value) => {
            streams().lock().unwrap()[&value].host as *mut c_void
        }
        _ => {
            eprintln!("STOP: CRT stream {file:p} is not registered");
            std::process::exit(78)
        }
    }
}
pub unsafe extern "win64" fn set_buffer(file: *mut c_void, buffer: *mut u8) {
    if !buffer.is_null() {
        eprintln!("STOP: CRT setbuf with caller-provided buffer not supported yet");
        std::process::exit(78);
    }
    unsafe {
        setbuf(host(file), buffer);
    }
}
pub extern "win64" fn flush(file: *mut c_void) -> i32 {
    unsafe { fflush(if file.is_null() { file } else { host(file) }) }
}
pub extern "win64" fn descriptor(file: *mut c_void) -> i32 {
    unsafe { fileno(host(file)) }
}
pub extern "win64" fn os_handle(fd: i32) -> isize {
    if !(0..=2).contains(&fd) {
        return -1;
    }
    crate::host::process::PROCESS
        .get()
        .map(|p| p.standard_handles[fd as usize] as isize)
        .unwrap_or(-1)
}

unsafe extern "C" {
    #[link_name = "fopen"]
    fn host_fopen(path: *const std::ffi::c_char, mode: *const std::ffi::c_char) -> *mut c_void;
    #[link_name = "fclose"]
    fn host_fclose(file: *mut c_void) -> i32;
    #[link_name = "fread"]
    fn host_fread(out: *mut c_void, size: usize, count: usize, file: *mut c_void) -> usize;
    #[link_name = "fwrite"]
    fn host_fwrite(out: *const c_void, size: usize, count: usize, file: *mut c_void) -> usize;
    #[link_name = "fseeko"]
    fn host_seek(file: *mut c_void, offset: i64, whence: i32) -> i32;
    #[link_name = "ftello"]
    fn host_tell(file: *mut c_void) -> i64;
    #[link_name = "ferror"]
    fn host_error(file: *mut c_void) -> i32;
    #[link_name = "feof"]
    fn host_eof(file: *mut c_void) -> i32;
    fn __errno_location() -> *mut i32;
}
pub extern "win64" fn errno() -> *mut i32 {
    unsafe { __errno_location() }
}
fn failure(code: i32) -> *mut c_void {
    unsafe {
        *errno() = code;
    }
    std::ptr::null_mut()
}
pub unsafe extern "win64" fn open(
    path: *const std::ffi::c_char,
    mode: *const std::ffi::c_char,
) -> *mut c_void {
    let path = match unsafe { crate::host::paths::narrow(path) }
        .and_then(|s| crate::host::paths::host(&s))
    {
        Ok(p) => p,
        Err(_) => return failure(22),
    };
    let mode = match unsafe { crate::host::paths::narrow(mode) } {
        Ok(s) => s,
        Err(_) => return failure(22),
    };
    if !mode.bytes().all(|b| b"rwa+bx".contains(&b)) || !mode.contains('b') {
        eprintln!("CRT fopen: only binary stream modes are supported ({mode})");
        return failure(22);
    }
    use std::os::unix::ffi::OsStrExt;
    let Ok(path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return failure(22);
    };
    let mode = std::ffi::CString::new(mode).unwrap();
    let file = unsafe { host_fopen(path.as_ptr(), mode.as_ptr()) };
    if file.is_null() {
        return file;
    }
    let mut layout = Box::new([0u64; 6]);
    unsafe {
        layout
            .as_mut_ptr()
            .cast::<u8>()
            .add(24)
            .cast::<i32>()
            .write(fileno(file));
    }
    let pointer = layout.as_mut_ptr() as usize;
    streams().lock().unwrap().insert(
        pointer,
        Stream {
            _layout: layout,
            host: file as usize,
        },
    );
    pointer as *mut c_void
}
pub unsafe extern "win64" fn open_w(path: *const u16, mode: *const u16) -> *mut c_void {
    let (Ok(path), Ok(mode)) = (unsafe { crate::host::paths::wide(path) }, unsafe {
        crate::host::paths::wide(mode)
    }) else {
        return failure(22);
    };
    let (Ok(path), Ok(mode)) = (std::ffi::CString::new(path), std::ffi::CString::new(mode)) else {
        return failure(22);
    };
    unsafe { open(path.as_ptr(), mode.as_ptr()) }
}
pub unsafe extern "win64" fn open_w_s(
    out: *mut *mut c_void,
    path: *const u16,
    mode: *const u16,
) -> i32 {
    if out.is_null() {
        return 22;
    }
    let file = unsafe { open_w(path, mode) };
    unsafe {
        *out = file;
    }
    if file.is_null() {
        unsafe { *errno() }
    } else {
        0
    }
}
pub unsafe extern "win64" fn shared_open_w(
    path: *const u16,
    mode: *const u16,
    share: i32,
) -> *mut c_void {
    if share != 0x40 {
        return failure(22);
    }
    unsafe { open_w(path, mode) }
}
pub extern "win64" fn close(file: *mut c_void) -> i32 {
    let mut map = streams().lock().unwrap();
    if let Some(stream) = map.remove(&(file as usize)) {
        unsafe { host_fclose(stream.host as *mut c_void) }
    } else {
        unsafe {
            *errno() = 9;
        }
        -1
    }
}
pub unsafe extern "win64" fn read(
    out: *mut c_void,
    size: usize,
    count: usize,
    file: *mut c_void,
) -> usize {
    unsafe { host_fread(out, size, count, host(file)) }
}
pub unsafe extern "win64" fn write(
    out: *const c_void,
    size: usize,
    count: usize,
    file: *mut c_void,
) -> usize {
    unsafe { host_fwrite(out, size, count, host(file)) }
}
pub extern "win64" fn seek(file: *mut c_void, offset: i32, whence: i32) -> i32 {
    unsafe { host_seek(host(file), offset as i64, whence) }
}
pub extern "win64" fn seek64(file: *mut c_void, offset: i64, whence: i32) -> i32 {
    unsafe { host_seek(host(file), offset, whence) }
}
pub extern "win64" fn tell(file: *mut c_void) -> i32 {
    let value = unsafe { host_tell(host(file)) };
    if value > i32::MAX as i64 {
        unsafe {
            *errno() = 22;
        }
        -1
    } else {
        value as i32
    }
}
pub extern "win64" fn tell64(file: *mut c_void) -> i64 {
    unsafe { host_tell(host(file)) }
}
pub extern "win64" fn file_error(file: *mut c_void) -> i32 {
    unsafe { host_error(host(file)) }
}
pub extern "win64" fn eof(file: *mut c_void) -> i32 {
    unsafe { host_eof(host(file)) }
}
pub unsafe extern "win64" fn getpos(file: *mut c_void, pos: *mut i64) -> i32 {
    if pos.is_null() {
        return -1;
    }
    let value = tell64(file);
    if value < 0 {
        return -1;
    }
    unsafe {
        *pos = value;
    }
    0
}
pub unsafe extern "win64" fn setpos(file: *mut c_void, pos: *const i64) -> i32 {
    if pos.is_null() {
        return -1;
    }
    seek64(file, unsafe { *pos }, 0)
}
