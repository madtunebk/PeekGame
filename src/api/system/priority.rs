//! Current-process priority classes mapped to Linux per-thread nice values.
use super::error;
unsafe extern "C" {
    fn setpriority(which: i32, who: u32, priority: i32) -> i32;
    fn getpriority(which: i32, who: u32) -> i32;
    fn __errno_location() -> *mut i32;
}
pub extern "win64" fn set(process: usize, class: u32) -> i32 {
    if process != usize::MAX {
        error::set(6);
        return 0;
    }
    let nice = match class {
        0x40 => 19,
        0x4000 => 5,
        0x20 => 0,
        0x8000 => -5,
        0x80 => -10,
        0x100 => {
            error::set(50);
            return 0;
        }
        _ => {
            error::set(87);
            return 0;
        }
    };
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        error::set(5);
        return 0;
    };
    let mut ok = true;
    for entry in tasks {
        let Ok(entry) = entry else {
            ok = false;
            continue;
        };
        let Some(id) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if unsafe { setpriority(0, id, nice) } != 0 {
            // A thread may exit while its /proc directory is being enumerated.
            if std::io::Error::last_os_error().raw_os_error() != Some(3) {
                ok = false;
            }
        }
    }
    if !ok {
        error::set(5);
        return 0;
    }
    1
}
pub extern "win64" fn get(process: usize) -> u32 {
    if process != usize::MAX {
        error::set(6);
        return 0;
    }
    unsafe {
        *__errno_location() = 0;
    }
    let nice = unsafe { getpriority(0, super::startup::process_id()) };
    if unsafe { *__errno_location() } != 0 {
        error::set(5);
        return 0;
    }
    match nice {
        15.. => 0x40,
        5..=14 => 0x4000,
        0..=4 => 0x20,
        -5..=-1 => 0x8000,
        _ => 0x80,
    }
}
