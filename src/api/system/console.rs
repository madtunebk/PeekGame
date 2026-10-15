use crate::api::system::error;
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
    // Querying terminal-backed stream flags is limited to processed output.
    unsafe {
        *out = 1;
    }
    1
}
pub extern "win64" fn file_type(handle: usize) -> u32 {
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
