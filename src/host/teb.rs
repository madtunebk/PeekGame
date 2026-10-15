use std::{cell::RefCell, ffi::c_void};
unsafe extern "C" {
    fn syscall(number: i64, ...) -> i64;
    fn pthread_self() -> usize;
    fn pthread_getattr_np(thread: usize, attr: *mut u8) -> i32;
    fn pthread_attr_getstack(attr: *const u8, stack: *mut *mut c_void, size: *mut usize) -> i32;
    fn pthread_attr_destroy(attr: *mut u8) -> i32;
    fn getpid() -> i32;
}
struct ThreadEnvironment {
    teb: Box<[u64; 1024]>,
    _peb: Box<[u64; 128]>,
    previous: u64,
}
thread_local! {static ENVIRONMENT:RefCell<Option<ThreadEnvironment>>=const{RefCell::new(None)};}
/// Gives the calling thread a TEB/PEB in gs. `data` is the EXE's TLS template
/// (slot 0); the TLS vector also holds a block for every DLL with static TLS.
pub fn initialize(image: usize, data: Vec<u8>) -> Result<(), String> {
    ENVIRONMENT.with(|cell| {
        if cell.borrow().is_some() {
            return Err("thread environment already initialized".into());
        }
        let mut environment = ThreadEnvironment {
            teb: Box::new([0; 1024]),
            _peb: Box::new([0; 128]),
            previous: 0,
        };
        let mut attr = [0u64; 8];
        let mut stack = std::ptr::null_mut();
        let mut size = 0usize;
        unsafe {
            if pthread_getattr_np(pthread_self(), attr.as_mut_ptr().cast()) != 0 {
                return Err("cannot query Windows thread stack".into());
            }
            let code = pthread_attr_getstack(attr.as_ptr().cast(), &mut stack, &mut size);
            pthread_attr_destroy(attr.as_mut_ptr().cast());
            if code != 0 {
                return Err("cannot query stack bounds".into());
            }
            environment.teb[1] = stack as u64 + size as u64;
            environment.teb[2] = stack as u64;
            environment.teb[6] = environment.teb.as_ptr() as u64;
            environment.teb[8] = getpid() as u64;
            environment.teb[9] = syscall(186) as u64;
            environment._peb[2] = image as u64;
            environment.teb[12] = environment._peb.as_ptr() as u64;
            crate::images::tls::set_exe_template(data);
            crate::images::tls::thread_start(&mut environment.teb[11]);
            if syscall(158, 0x1004i32, &mut environment.previous) != 0 {
                return Err("ARCH_GET_GS failed".into());
            }
            if syscall(158, 0x1001i32, environment.teb.as_ptr()) != 0 {
                return Err("ARCH_SET_GS failed".into());
            }
        }
        *cell.borrow_mut() = Some(environment);
        Ok(())
    })
}
pub fn set_error(value: u32) {
    let _ = ENVIRONMENT.try_with(|env| {
        if let Some(env) = env.borrow_mut().as_mut() {
            unsafe {
                env.teb
                    .as_mut_ptr()
                    .cast::<u8>()
                    .add(0x68)
                    .cast::<u32>()
                    .write(value);
            }
        }
    });
}
pub fn error() -> Option<u32> {
    ENVIRONMENT
        .try_with(|env| {
            env.borrow()
                .as_ref()
                .map(|env| unsafe { env.teb.as_ptr().cast::<u8>().add(0x68).cast::<u32>().read() })
        })
        .ok()
        .flatten()
}
impl Drop for ThreadEnvironment {
    fn drop(&mut self) {
        crate::images::tls::thread_end(&mut self.teb[11]);
        unsafe {
            syscall(158, 0x1001i32, self.previous);
        }
    }
}
