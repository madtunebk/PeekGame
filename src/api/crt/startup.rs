use std::{
    ffi::{c_char, c_void, CString},
    sync::{
        atomic::{AtomicI32, Ordering},
        Mutex, OnceLock,
    },
};
unsafe extern "C" {
    fn atexit(callback: extern "C" fn()) -> i32;
    fn fflush(file: *mut c_void) -> i32;
}
type Callback = extern "win64" fn();
static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static REGISTERED: OnceLock<bool> = OnceLock::new();
pub static FMODE: AtomicI32 = AtomicI32::new(0);
pub static COMMODE: AtomicI32 = AtomicI32::new(0);
static APP_TYPE: AtomicI32 = AtomicI32::new(0);
static MATHERR: Mutex<usize> = Mutex::new(0);
extern "C" fn run_callbacks() {
    loop {
        let callback = CALLBACKS.lock().unwrap().pop();
        match callback {
            Some(callback) => callback(),
            None => break,
        }
    }
}
pub extern "win64" fn register(callback: Option<Callback>) -> i32 {
    let Some(callback) = callback else {
        return -1;
    };
    if !*REGISTERED.get_or_init(|| unsafe { atexit(run_callbacks) == 0 }) {
        return -1;
    }
    CALLBACKS.lock().unwrap().push(callback);
    0
}
pub extern "win64" fn on_exit(callback: Option<Callback>) -> Option<Callback> {
    if register(callback) == 0 {
        callback
    } else {
        None
    }
}
pub unsafe extern "win64" fn init_term(
    mut begin: *const Option<Callback>,
    end: *const Option<Callback>,
) {
    while begin < end {
        if let Some(callback) = unsafe { *begin } {
            callback();
        }
        begin = unsafe { begin.add(1) };
    }
}
pub extern "win64" fn cexit() {
    run_callbacks();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}
pub extern "win64" fn exit(code: i32) -> ! {
    cexit();
    std::process::exit(code & 255)
}
pub extern "win64" fn amsg_exit(code: i32) -> ! {
    eprintln!("msvcrt runtime error R60{code:02}");
    std::process::exit(255)
}
pub extern "win64" fn app_type(kind: i32) {
    if !(0..=2).contains(&kind) {
        eprintln!("STOP: invalid CRT application type {kind}");
        std::process::exit(78);
    }
    APP_TYPE.store(kind, Ordering::Relaxed);
}
pub extern "win64" fn matherr(callback: usize) {
    *MATHERR.lock().unwrap() = callback;
}
struct Args {
    strings: Vec<CString>,
    pointers: Vec<usize>,
    environment: Vec<CString>,
    envp: Vec<usize>,
}
static ARGS: OnceLock<Args> = OnceLock::new();
#[no_mangle]
static CRT_INITENV: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
pub fn initenv() -> *const c_void {
    &CRT_INITENV as *const _ as *const c_void
}
fn args() -> &'static Args {
    ARGS.get_or_init(|| {
        let line = crate::host::process::PROCESS
            .get()
            .expect("process configured")
            .command_line
            .to_string_lossy();
        let mut strings = Vec::new();
        let mut token = String::new();
        let mut quoted = false;
        let mut slashes = 0;
        let mut started = false;
        for c in line.chars() {
            if c == '\\' {
                slashes += 1;
                started = true;
                continue;
            }
            if c == '"' {
                token.push_str(&"\\".repeat(slashes / 2));
                if slashes % 2 == 1 {
                    token.push('"');
                } else {
                    quoted = !quoted;
                }
                slashes = 0;
                started = true;
                continue;
            }
            token.push_str(&"\\".repeat(slashes));
            slashes = 0;
            if c.is_ascii_whitespace() && !quoted {
                if started {
                    strings.push(CString::new(std::mem::take(&mut token)).unwrap());
                    started = false;
                }
            } else {
                token.push(c);
                started = true;
            }
        }
        token.push_str(&"\\".repeat(slashes));
        if started {
            strings.push(CString::new(token).unwrap());
        }
        let mut pointers = strings
            .iter()
            .map(|s| s.as_ptr() as usize)
            .collect::<Vec<_>>();
        pointers.push(0);
        use std::os::unix::ffi::OsStrExt;
        let environment = std::env::vars_os()
            .filter_map(|(k, v)| {
                let mut b = k.as_bytes().to_vec();
                b.push(b'=');
                b.extend_from_slice(v.as_bytes());
                CString::new(b).ok()
            })
            .collect::<Vec<_>>();
        let mut envp = environment
            .iter()
            .map(|s| s.as_ptr() as usize)
            .collect::<Vec<_>>();
        envp.push(0);
        CRT_INITENV.store(envp.as_ptr() as usize, Ordering::Relaxed);
        Args {
            strings,
            pointers,
            environment,
            envp,
        }
    })
}
pub unsafe extern "win64" fn mainargs(
    argc: *mut i32,
    argv: *mut *const *const c_char,
    env: *mut *const *const c_char,
    wildcard: i32,
    _info: *mut c_void,
) -> i32 {
    if wildcard != 0 || argc.is_null() || argv.is_null() || env.is_null() {
        return 22;
    }
    let args = args();
    let _keep = &args.environment;
    unsafe {
        *argc = args.strings.len() as i32;
        *argv = args.pointers.as_ptr().cast();
        *env = args.envp.as_ptr().cast();
    }
    0
}
