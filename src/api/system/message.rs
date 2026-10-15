//! FormatMessage (ported from peek): system texts for common Win32 error codes;
//! unknown codes fail with ERROR_MR_MID_NOT_FOUND. FROM_STRING works when no inserts
//! are present; message tables in modules are not supported. Every formatted system
//! message is also printed, since a program formats errors when something failed.
use crate::api::{memory::local, system::error, text};
const ALLOCATE_BUFFER: u32 = 0x100;
const IGNORE_INSERTS: u32 = 0x200;
const FROM_STRING: u32 = 0x400;
const FROM_SYSTEM: u32 = 0x1000;

fn system_message(code: u32) -> Option<&'static str> {
    Some(match code {
        0 => "The operation completed successfully.",
        1 => "Incorrect function.",
        2 => "The system cannot find the file specified.",
        3 => "The system cannot find the path specified.",
        5 => "Access is denied.",
        6 => "The handle is invalid.",
        8 => "Not enough memory resources are available to process this command.",
        14 => "Not enough memory resources are available to complete this operation.",
        18 => "There are no more files.",
        50 => "The request is not supported.",
        87 => "The parameter is incorrect.",
        122 => "The data area passed to a system call is too small.",
        123 => "The filename, directory name, or volume label syntax is incorrect.",
        126 => "The specified module could not be found.",
        127 => "The specified procedure could not be found.",
        183 => "Cannot create a file when that file already exists.",
        203 => "The system could not find the environment option that was entered.",
        1400 => "Invalid window handle.",
        1407 => "Cannot find window class.",
        1410 => "Class already exists.",
        1460 => "This operation returned because the timeout period expired.",
        2000 => "The pixel format is invalid.",
        _ => return None,
    })
}
fn format(flags: u32, source: Option<String>, id: u32) -> Option<String> {
    if flags & FROM_STRING != 0 {
        let Some(source) = source else {
            error::set(87);
            return None;
        };
        if source.contains('%') && flags & IGNORE_INSERTS == 0 {
            eprintln!("FormatMessage: inserts are not supported");
            error::set(50);
            return None;
        }
        return Some(source);
    }
    if flags & FROM_SYSTEM != 0 {
        let Some(text) = system_message(id) else {
            eprintln!("FormatMessage: no text for message {id:#x}");
            error::set(317);
            return None;
        };
        eprintln!("FormatMessage({id}): {text}");
        // System messages end with CR LF, like Windows' message table entries.
        return Some(format!("{text}\r\n"));
    }
    eprintln!("FormatMessage: flags {flags:#x} not supported");
    error::set(50);
    None
}
pub unsafe extern "win64" fn format_w(
    flags: u32,
    source: *const u16,
    id: u32,
    _lang: u32,
    buffer: *mut u16,
    size: u32,
    _args: usize,
) -> u32 {
    let source = (flags & FROM_STRING != 0)
        .then(|| unsafe { text::string(source) })
        .flatten();
    let Some(message) = format(flags, source, id) else {
        return 0;
    };
    let mut wide = text::wide(&message);
    let length = wide.len() as u32;
    wide.push(0);
    if flags & ALLOCATE_BUFFER != 0 {
        let block = local::allocate(0, wide.len() * 2) as *mut u16;
        if block.is_null() {
            error::set(8);
            return 0;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), block, wide.len());
            *(buffer as *mut *mut u16) = block;
        }
        return length;
    }
    if buffer.is_null() || wide.len() as u32 > size {
        error::set(122);
        return 0;
    }
    unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), buffer, wide.len()) };
    length
}
pub unsafe extern "win64" fn format_a(
    flags: u32,
    source: *const u8,
    id: u32,
    _lang: u32,
    buffer: *mut u8,
    size: u32,
    _args: usize,
) -> u32 {
    let source = (flags & FROM_STRING != 0 && !source.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(source.cast()) }
            .to_string_lossy()
            .into_owned()
    });
    let Some(message) = format(flags, source, id) else {
        return 0;
    };
    let mut bytes = message.into_bytes();
    let length = bytes.len() as u32;
    bytes.push(0);
    if flags & ALLOCATE_BUFFER != 0 {
        let block = local::allocate(0, bytes.len()) as *mut u8;
        if block.is_null() {
            error::set(8);
            return 0;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), block, bytes.len());
            *(buffer as *mut *mut u8) = block;
        }
        return length;
    }
    if buffer.is_null() || bytes.len() as u32 > size {
        error::set(122);
        return 0;
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    length
}
