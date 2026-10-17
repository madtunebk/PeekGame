use std::{
    ffi::c_void,
    time::{SystemTime, UNIX_EPOCH},
};
pub extern "win64" fn set_filter(filter: usize) -> usize {
    crate::seh::set_unhandled_filter(filter)
}
unsafe extern "C" {
    fn getpid() -> i32;
    fn syscall(number: i64, ...) -> i64;
}
pub extern "win64" fn process_id() -> u32 {
    unsafe { getpid() as u32 }
}
pub extern "win64" fn thread_id() -> u32 {
    unsafe { syscall(186) as u32 }
}
pub extern "win64" fn process() -> usize {
    usize::MAX
}
pub extern "win64" fn thread() -> usize {
    usize::MAX - 1
}
pub extern "win64" fn set_error(code: u32) {
    crate::api::system::error::set(code);
}
pub unsafe extern "win64" fn startup_info(p: *mut u8) {
    if p.is_null() {
        return;
    }
    unsafe {
        std::ptr::write_bytes(p, 0, 104);
        p.cast::<u32>().write(104);
        p.add(60).cast::<u32>().write(0x100);
    }
    if let Some(process) = crate::host::process::PROCESS.get() {
        for i in 0..3 {
            unsafe {
                p.add(80 + i * 8)
                    .cast::<usize>()
                    .write(process.standard_handles[i].load(std::sync::atomic::Ordering::SeqCst));
            }
        }
    }
}
pub unsafe extern "win64" fn file_time(p: *mut u64) {
    if !p.is_null() {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let ticks =
            116444736000000000u64 + time.as_secs() * 10000000 + time.subsec_nanos() as u64 / 100;
        unsafe {
            p.write_unaligned(ticks);
        }
    }
}
pub unsafe extern "win64" fn performance_frequency(p: *mut i64) -> i32 {
    if p.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        *p = 1000000000;
    }
    1
}
pub unsafe extern "win64" fn performance_counter(p: *mut i64) -> i32 {
    if p.is_null() {
        crate::api::system::error::set(87);
        return 0;
    }
    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }
    let mut time = Timespec { sec: 0, nsec: 0 };
    if unsafe {
        clock_gettime(
            7, /* CLOCK_BOOTTIME: QPC includes suspend on Windows */
            &mut time,
        )
    } != 0
    {
        crate::api::system::error::set(87);
        return 0;
    }
    unsafe {
        *p = time.sec * 1000000000 + time.nsec;
    }
    1
}
pub extern "win64" fn sleep(ms: u32) {
    crate::api::threading::iocp::release_worker();
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}
pub extern "win64" fn is_debugger_present() -> i32 {
    0
}
pub extern "win64" fn get_command_line_w() -> *const u16 {
    static LINE: std::sync::OnceLock<Vec<u16>> = std::sync::OnceLock::new();
    LINE.get_or_init(|| {
        crate::host::process::PROCESS
            .get()
            .expect("configured process")
            .command_line
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect()
    })
    .as_ptr()
}
// c_void keeps Windows opaque pointer spelling available to this subsystem.
const _: usize = std::mem::size_of::<*const c_void>();
