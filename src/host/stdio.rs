use std::{
    ffi::c_void,
    io::{Error, ErrorKind},
};
unsafe extern "C" {
    #[link_name = "write"]
    fn host_write(fd: i32, data: *const c_void, length: usize) -> isize;
    fn fcntl(fd: i32, command: i32, ...) -> i32;
}
// Private Windows-side handle tokens, never raw Linux descriptors or object pointers.
const STREAM_BASE: usize = 0x100;
pub(crate) fn inherited_handles() -> [usize; 3] {
    std::array::from_fn(|fd| {
        if unsafe {
            fcntl(fd as i32, 1 /* Linux F_GETFD */)
        } < 0
        {
            0
        } else {
            STREAM_BASE + fd
        }
    })
}
pub(crate) fn fd_for(handle: usize) -> Option<i32> {
    handle
        .checked_sub(STREAM_BASE)
        .filter(|fd| *fd < 3)
        .map(|fd| fd as i32)
}
pub(crate) unsafe fn write(fd: i32, data: *const c_void, length: usize) -> Result<usize, u32> {
    loop {
        let n = unsafe { host_write(fd, data, length) };
        if n >= 0 {
            return Ok(n as usize);
        }
        let error = Error::last_os_error();
        if error.kind() == ErrorKind::Interrupted {
            continue;
        }
        return Err(match error.raw_os_error() {
            Some(9) => 6,    // EBADF -> ERROR_INVALID_HANDLE
            Some(28) => 112, // ENOSPC -> ERROR_DISK_FULL
            _ => match error.kind() {
                ErrorKind::PermissionDenied => 5,
                ErrorKind::BrokenPipe => 109,
                ErrorKind::InvalidInput => 87,
                ErrorKind::WouldBlock => 232,
                _ => 29, // ERROR_WRITE_FAULT: a real host write failed.
            },
        });
    }
}
