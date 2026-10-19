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

pub extern "win64" fn get_console_window() -> usize {
    0
}

static CONSOLE_TITLE: Mutex<Option<Vec<u16>>> = Mutex::new(None);

fn has_console() -> bool {
    (0..3).any(|fd| unsafe { isatty(fd) } != 0) || std::env::var_os("TERM").is_some()
}

fn current_title() -> Vec<u16> {
    if let Some(ref t) = *CONSOLE_TITLE.lock().unwrap() {
        return t.clone();
    }
    crate::images::exe()
        .map(|m| {
            let win_path = if m.path.to_string_lossy().starts_with("C:")
                || m.path.to_string_lossy().starts_with(r"C:\")
            {
                m.path.to_string_lossy().into_owned()
            } else {
                crate::host::paths::windows(&m.path)
            };
            win_path.encode_utf16().collect()
        })
        .unwrap_or_else(|| "Console".encode_utf16().collect())
}

pub unsafe extern "win64" fn get_title_w(buffer: *mut u16, capacity: u32) -> u32 {
    if buffer.is_null() || capacity == 0 {
        error::set(87);
        return 0;
    }
    if !has_console() {
        error::set(6);
        return 0;
    }
    let title = current_title();
    let to_copy = (capacity as usize - 1).min(title.len());
    if to_copy > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(title.as_ptr(), buffer, to_copy);
        }
    }
    unsafe {
        *buffer.add(to_copy) = 0;
    }
    to_copy as u32
}

pub unsafe extern "win64" fn get_title_a(buffer: *mut u8, capacity: u32) -> u32 {
    if buffer.is_null() || capacity == 0 {
        error::set(87);
        return 0;
    }
    if !has_console() {
        error::set(6);
        return 0;
    }
    let title_w = current_title();
    let title = String::from_utf16_lossy(&title_w);
    let bytes = title.as_bytes();
    let to_copy = (capacity as usize - 1).min(bytes.len());
    if to_copy > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, to_copy);
        }
    }
    unsafe {
        *buffer.add(to_copy) = 0;
    }
    to_copy as u32
}

pub unsafe extern "win64" fn set_title_w(title: *const u16) -> i32 {
    if title.is_null() {
        error::set(87);
        return 0;
    }
    if !has_console() {
        error::set(6);
        return 0;
    }
    let mut len = 0;
    while unsafe { *title.add(len) } != 0 {
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(title, len) };
    *CONSOLE_TITLE.lock().unwrap() = Some(slice.to_vec());
    1
}

pub unsafe extern "win64" fn set_title_a(title: *const u8) -> i32 {
    if title.is_null() {
        error::set(87);
        return 0;
    }
    if !has_console() {
        error::set(6);
        return 0;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(title as *const std::ffi::c_char) };
    let s = c_str.to_string_lossy();
    *CONSOLE_TITLE.lock().unwrap() = Some(s.encode_utf16().collect());
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_title_w_and_a() {
        unsafe {
            // Null buffer or 0 capacity should fail with ERROR_INVALID_PARAMETER (87)
            assert_eq!(get_title_w(std::ptr::null_mut(), 100), 0);
            assert_eq!(error::get_last_error(), 87);

            let mut buf_w = [0u16; 64];
            assert_eq!(get_title_w(buf_w.as_mut_ptr(), 0), 0);
            assert_eq!(error::get_last_error(), 87);

            // Null pointer to set_title should fail with ERROR_INVALID_PARAMETER (87)
            assert_eq!(set_title_w(std::ptr::null()), 0);
            assert_eq!(error::get_last_error(), 87);
            assert_eq!(set_title_a(std::ptr::null()), 0);
            assert_eq!(error::get_last_error(), 87);

            // Set wide title
            let new_title_w: Vec<u16> = "DOOM Eternal\0".encode_utf16().collect();
            assert_eq!(set_title_w(new_title_w.as_ptr()), 1);

            // Get wide title with full buffer
            let len = get_title_w(buf_w.as_mut_ptr(), buf_w.len() as u32);
            assert_eq!(len, 12);
            assert_eq!(buf_w[12], 0);
            assert_eq!(String::from_utf16_lossy(&buf_w[..12]), "DOOM Eternal");

            // Get wide title with truncated capacity (capacity 5 -> 4 chars + null terminator)
            let len_trunc = get_title_w(buf_w.as_mut_ptr(), 5);
            assert_eq!(len_trunc, 4);
            assert_eq!(buf_w[4], 0);
            assert_eq!(String::from_utf16_lossy(&buf_w[..4]), "DOOM");

            // Set ANSI title
            let new_title_a = b"idTech Launcher\0";
            assert_eq!(set_title_a(new_title_a.as_ptr()), 1);

            // Get ANSI title
            let mut buf_a = [0u8; 64];
            let len_a = get_title_a(buf_a.as_mut_ptr(), buf_a.len() as u32);
            assert_eq!(len_a, 15);
            assert_eq!(buf_a[15], 0);
            assert_eq!(&buf_a[..15], b"idTech Launcher");

            // Truncated ANSI title (capacity 7 -> 6 chars + null terminator)
            let len_a_trunc = get_title_a(buf_a.as_mut_ptr(), 7);
            assert_eq!(len_a_trunc, 6);
            assert_eq!(buf_a[6], 0);
            assert_eq!(&buf_a[..6], b"idTech");
        }
    }
}



