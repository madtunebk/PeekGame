use crate::api::system::error;
use std::sync::Mutex;
// stdout and stderr address the same inherited console screen buffer.
static OUTPUT_MODE: Mutex<u32> = Mutex::new(3);
unsafe extern "C" {
    fn isatty(fd: i32) -> i32;
}
pub extern "win64" fn attach(process: u32) -> i32 {
    if process != u32::MAX {
        error::set(50);
        return 0;
    }
    if (0..3).any(|fd| unsafe { isatty(fd) } != 0) {
        1
    } else {
        error::set(6);
        0
    }
}
pub extern "win64" fn allocate() -> i32 {
    error::set(50);
    0
}
pub extern "win64" fn output_cp(codepage: u32) -> i32 {
    if codepage == 65001 {
        1
    } else {
        error::set(87);
        0
    }
}
pub unsafe extern "win64" fn mode(handle: usize, out: *mut u32) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    if !(0x100..=0x102).contains(&handle) || unsafe { isatty((handle - 0x100) as i32) } == 0 {
        error::set(6);
        return 0;
    }
    if handle == 0x100 {
        error::set(50);
        return 0;
    }
    unsafe {
        *out = *OUTPUT_MODE.lock().unwrap();
    }
    1
}
pub extern "win64" fn file_type(handle: usize) -> u32 {
    if let Some(kind) = crate::api::filesystem::files::file_type(handle) {
        return kind;
    }
    if !(0x100..=0x102).contains(&handle) {
        error::set(6);
        return 0;
    }
    use std::os::unix::fs::FileTypeExt;
    match std::fs::metadata(format!("/proc/self/fd/{}", handle - 0x100)) {
        Ok(m) => {
            let t = m.file_type();
            if t.is_char_device() {
                2
            } else if t.is_fifo() || t.is_socket() {
                3
            } else {
                1
            }
        }
        Err(_) => {
            error::set(6);
            0
        }
    }
}

/// WriteConsoleW counts UTF-16 code units, and rejects redirected file handles.
pub unsafe extern "win64" fn write_w(
    handle: usize,
    buffer: *const u16,
    count: u32,
    written: *mut u32,
    reserved: *mut std::ffi::c_void,
) -> i32 {
    if !written.is_null() {
        unsafe {
            written.write(0);
        }
    }
    if !reserved.is_null() || (buffer.is_null() && count != 0) {
        error::set(87);
        return 0;
    }
    let Some(fd) = crate::host::stdio::fd_for(handle) else {
        error::set(6);
        return 0;
    };
    if fd == 0 || unsafe { isatty(fd) } == 0 {
        error::set(6);
        return 0;
    }
    let units = if count == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(buffer, count as usize) }
    };
    let text = String::from_utf16_lossy(units);
    let mut offset = 0;
    while offset < text.len() {
        match unsafe {
            crate::host::stdio::write(fd, text.as_ptr().add(offset).cast(), text.len() - offset)
        } {
            Ok(0) => {
                error::set(29);
                return 0;
            }
            Ok(n) => offset += n,
            Err(code) => {
                error::set(code);
                return 0;
            }
        }
    }
    if !written.is_null() {
        unsafe {
            written.write(count);
        }
    }
    1
}

pub unsafe extern "win64" fn read_w(
    handle: usize,
    buffer: *mut u16,
    count: u32,
    read_out: *mut u32,
    _input_control: *mut std::ffi::c_void,
) -> i32 {
    if !read_out.is_null() {
        unsafe { read_out.write(0) };
    }
    if buffer.is_null() && count != 0 {
        error::set(87);
        return 0;
    }
    let Some(fd) = crate::host::stdio::fd_for(handle) else {
        error::set(6);
        return 0;
    };
    if fd != 0 || unsafe { isatty(fd) } == 0 {
        error::set(6);
        return 0;
    }
    if count == 0 {
        return 1;
    }
    let mut raw = vec![0u8; count as usize];
    match std::io::Read::read(&mut std::io::stdin(), &mut raw) {
        Ok(n) => {
            let units: Vec<u16> = String::from_utf8_lossy(&raw[..n]).encode_utf16().collect();
            let copied = units.len().min(count as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(units.as_ptr(), buffer, copied);
                if !read_out.is_null() {
                    read_out.write(copied as u32);
                }
            }
            1
        }
        Err(_) => {
            error::set(30);
            0
        }
    }
}

/// The host terminal handles processed output, wrapping and ANSI/VT sequences.
/// Input modes and modes needing an emulated console screen buffer are unsupported.
pub extern "win64" fn set_mode(handle: usize, flags: u32) -> i32 {
    let Some(fd) = crate::host::stdio::fd_for(handle) else {
        error::set(6);
        return 0;
    };
    if unsafe { isatty(fd) } == 0 {
        error::set(6);
        return 0;
    }
    if fd == 0 {
        error::set(50);
        return 0;
    }
    if flags & !0x1f != 0 {
        error::set(87);
        return 0;
    }
    // Disabling processing/wrapping, delayed newline and LVB grid attributes
    // need a screen-buffer implementation; never claim those were applied.
    if flags & 3 != 3 || flags & 0x18 != 0 {
        error::set(50);
        return 0;
    }
    *OUTPUT_MODE.lock().unwrap() = flags;
    1
}

/// A PTY exposes its dimensions but not a Win32 screen buffer, cursor and attributes.
/// Do not synthesize a successful CONSOLE_SCREEN_BUFFER_INFO from partial data.
pub extern "win64" fn screen_buffer_info(handle: usize, out: *mut std::ffi::c_void) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    let Some(fd) = crate::host::stdio::fd_for(handle) else {
        error::set(6);
        return 0;
    };
    if fd == 0 || unsafe { isatty(fd) } == 0 {
        error::set(6);
        return 0;
    }
    error::set(50);
    0
}

/// Windows BGR/intensity attributes become terminal ANSI SGR colours.
pub extern "win64" fn text_attribute(handle: usize, attributes: u16) -> i32 {
    let Some(fd) = crate::host::stdio::fd_for(handle) else {
        error::set(6);
        return 0;
    };
    if fd == 0 || unsafe { isatty(fd) } == 0 {
        error::set(6);
        return 0;
    }
    // COMMON_LVB grid, reverse and underline flags need additional emulation.
    if attributes & !0xff != 0 {
        error::set(50);
        return 0;
    }
    const ANSI: [u16; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
    let fg = if attributes & 0xf == 7 {
        39
    } else {
        (if attributes & 8 != 0 { 90 } else { 30 }) + ANSI[(attributes & 7) as usize]
    };
    let bg = if attributes & 0xf0 == 0 {
        49
    } else {
        (if attributes & 0x80 != 0 { 100 } else { 40 }) + ANSI[((attributes >> 4) & 7) as usize]
    };
    let sequence = if attributes == 7 {
        "\x1b[0m".to_owned()
    } else {
        format!("\x1b[0;{fg};{bg}m")
    };
    let mut offset = 0;
    while offset < sequence.len() {
        match unsafe {
            crate::host::stdio::write(
                fd,
                sequence.as_ptr().add(offset).cast(),
                sequence.len() - offset,
            )
        } {
            Ok(0) => {
                error::set(29);
                return 0;
            }
            Ok(n) => offset += n,
            Err(code) => {
                error::set(code);
                return 0;
            }
        }
    }
    1
}

static CTRL_HANDLERS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static IGNORE_CTRL_C: Mutex<bool> = Mutex::new(false);

pub extern "win64" fn set_ctrl_handler(
    handler: Option<extern "win64" fn(u32) -> i32>,
    add: i32,
) -> i32 {
    let mut handlers = CTRL_HANDLERS.lock().unwrap();
    match handler {
        None => {
            *IGNORE_CTRL_C.lock().unwrap() = add != 0;
            1
        }
        Some(h) => {
            let addr = h as usize;
            if add != 0 {
                if handlers.len() >= 32 {
                    error::set(8);
                    return 0;
                }
                if !handlers.contains(&addr) {
                    handlers.push(addr);
                }
                1
            } else {
                if let Some(pos) = handlers.iter().position(|&x| x == addr) {
                    handlers.remove(pos);
                    1
                } else {
                    error::set(87);
                    0
                }
            }
        }
    }
}

