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
    /// TlsExpansionSlots (TEB+0x1780): TLS indices 64 and up.
    expansion: Box<[usize; EXPANSION]>,
    previous: u64,
}
/// TlsSlots (TEB+0x1480) hold indices 0..63, TlsExpansionSlots 1024 more, as on
/// Windows: programs and JIT compilers read them straight from gs.
pub const TLS_SLOTS: usize = 64;
const EXPANSION: usize = 1024;
pub const TLS_INDICES: usize = TLS_SLOTS + EXPANSION;
/// Every live TEB, so TlsAlloc/TlsFree can clear an index in all threads.
static TEBS: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());
unsafe fn slot_in(teb: usize, index: usize) -> *mut usize {
    unsafe {
        if index < TLS_SLOTS {
            (teb as *mut usize).add(0x1480 / 8 + index)
        } else {
            (*(teb as *const *mut usize).add(0x1780 / 8)).add(index - TLS_SLOTS)
        }
    }
}
/// The calling thread's value cell for TLS `index`, if it has a TEB.
pub fn tls_slot(index: usize) -> Option<*mut usize> {
    if index >= TLS_INDICES {
        return None;
    }
    ENVIRONMENT
        .try_with(|env| {
            env.borrow()
                .as_ref()
                .map(|env| unsafe { slot_in(env.teb.as_ptr() as usize, index) })
        })
        .ok()
        .flatten()
}
/// Zeroes TLS `index` in every thread (TlsAlloc and TlsFree do this on Windows).
pub fn clear_tls_slot(index: usize) {
    if index >= TLS_INDICES {
        return;
    }
    for &teb in TEBS.lock().unwrap().iter() {
        unsafe { slot_in(teb, index).write(0) };
    }
}
pub(crate) fn peb_address() -> usize {
    peb(0)
}
/// The process environment block, shared by every thread (Windows code reads it
/// through gs:0x60): image base, process heap, the process parameters (command
/// line, image path, standard handles), processor count and OS version.
fn peb(image: usize) -> usize {
    static PEB: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *PEB.get_or_init(|| {
        let peb: &'static mut [u8; 0x800] = Box::leak(Box::new([0u8; 0x800]));
        let parameters: &'static mut [u8; 0x400] = Box::leak(Box::new([0u8; 0x400]));
        let put = |b: &mut [u8], at: usize, v: u64, n: usize| {
            b[at..at + n].copy_from_slice(&v.to_le_bytes()[..n])
        };
        // UNICODE_STRING: Length, MaximumLength (bytes), Buffer @8.
        let string = |b: &mut [u8], at: usize, text: &str| {
            let units: &'static mut [u16] = Box::leak(
                text.encode_utf16()
                    .chain([0])
                    .collect::<Vec<u16>>()
                    .into_boxed_slice(),
            );
            let length = (units.len() - 1) * 2;
            put(b, at, length as u64, 2);
            put(b, at + 2, length as u64 + 2, 2);
            put(b, at + 8, units.as_ptr() as u64, 8);
        };
        let process = crate::host::process::PROCESS.get();
        let command_line = process
            .map(|p| p.command_line.to_string_lossy().into_owned())
            .unwrap_or_default();
        let image_path = crate::images::exe()
            .map(|m| crate::host::paths::windows(&m.path))
            .unwrap_or_default();
        let directory = std::env::current_dir()
            .map(|d| crate::host::paths::windows(&d) + "\\")
            .unwrap_or_default();
        put(parameters, 0, 0x400, 4); // MaximumLength
        put(parameters, 4, 0x400, 4); // Length
        put(parameters, 8, 1, 4); // Flags: RTL_USER_PROC_PARAMS_NORMALIZED
        if let Some(p) = process {
            for (i, h) in p.standard_handles.iter().enumerate() {
                put(
                    parameters,
                    0x20 + 8 * i,
                    h.load(std::sync::atomic::Ordering::SeqCst) as u64,
                    8,
                );
            }
        }
        string(parameters, 0x38, &directory);
        string(parameters, 0x60, &image_path);
        string(parameters, 0x70, &command_line);
        put(peb, 0x10, image as u64, 8); // ImageBaseAddress
        put(peb, 0x20, parameters.as_ptr() as u64, 8); // ProcessParameters
        put(
            peb,
            0x30,
            crate::api::memory::heap::get_process_heap() as u64,
            8,
        ); // ProcessHeap
        let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
        put(peb, 0xb8, cpus as u64, 4); // NumberOfProcessors
        put(peb, 0x118, 10, 4); // OSMajorVersion
        put(peb, 0x11c, 0, 4); // OSMinorVersion
        put(peb, 0x120, 19045, 2); // OSBuildNumber
        put(peb, 0x124, 2, 4); // OSPlatformId: VER_PLATFORM_WIN32_NT
        peb.as_ptr() as usize
    })
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
            expansion: Box::new([0; EXPANSION]),
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
            environment.teb[2] = 0; // StackLimit: 0 disables MSVC __chkstk page probing past Linux pthread guard
            environment.teb[6] = environment.teb.as_ptr() as u64;
            environment.teb[8] = getpid() as u64;
            environment.teb[9] = syscall(186) as u64;
            environment.teb[12] = peb(image) as u64;
            environment.teb[0x108 / 8] = crate::api::system::locale::thread::get() as u64;
            environment.teb[0x1478 / 8] = stack as u64;
            environment.teb[0x1780 / 8] = environment.expansion.as_ptr() as u64;
            crate::images::tls::set_exe_template(data);
            crate::images::tls::thread_start(&mut environment.teb[11]);
            if syscall(158, 0x1004i32, &mut environment.previous) != 0 {
                return Err("ARCH_GET_GS failed".into());
            }
            if syscall(158, 0x1001i32, environment.teb.as_ptr()) != 0 {
                return Err("ARCH_SET_GS failed".into());
            }
        }
        TEBS.lock().unwrap().push(environment.teb.as_ptr() as usize);
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
/// Windows TEB.CurrentLocale, a 32-bit LCID at x64 offset 0x108.
pub(crate) fn set_locale(value: u32) {
    let _ = ENVIRONMENT.try_with(|env| {
        if let Some(env) = env.borrow_mut().as_mut() {
            unsafe {
                env.teb
                    .as_mut_ptr()
                    .cast::<u8>()
                    .add(0x108)
                    .cast::<u32>()
                    .write(value)
            };
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
        let teb = self.teb.as_ptr() as usize;
        TEBS.lock().unwrap().retain(|&t| t != teb);
        crate::images::tls::thread_end(&mut self.teb[11]);
        unsafe {
            syscall(158, 0x1001i32, self.previous);
        }
    }
}

/// Current Windows thread's native stack bounds, for bounded stack walking.
pub(crate) fn stack_bounds() -> Option<(u64, u64)> {
    ENVIRONMENT
        .try_with(|env| env.borrow().as_ref().map(|env| (env.teb[0x1478 / 8], env.teb[1])))
        .ok()
        .flatten()
}

pub(crate) fn set_fiber_data(ptr: usize) {
    let _ = ENVIRONMENT.try_with(|env| {
        if let Some(env) = env.borrow_mut().as_mut() {
            env.teb[4] = ptr as u64;
        }
    });
}

pub(crate) fn get_fiber_data() -> usize {
    ENVIRONMENT
        .try_with(|env| {
            env.borrow()
                .as_ref()
                .map(|env| env.teb[4] as usize)
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub(crate) fn current_stack_limits(low: *mut usize, high: *mut usize) {
    let (dealloc, base) = ENVIRONMENT
        .try_with(|env| {
            env.borrow()
                .as_ref()
                .map(|env| (env.teb[0x1478 / 8] as usize, env.teb[1] as usize))
        })
        .ok()
        .flatten()
        .unwrap_or((0, 0));
    unsafe {
        if !low.is_null() {
            *low = dealloc;
        }
        if !high.is_null() {
            *high = base;
        }
    }
}


