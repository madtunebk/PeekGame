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
    fn realloc(p: *mut c_void, size: usize) -> *mut c_void;
}
type Callback = extern "win64" fn();
static CALLBACKS: Mutex<Vec<Callback>> = Mutex::new(Vec::new());
static REGISTERED: OnceLock<bool> = OnceLock::new();
#[export_name = "peekgame.msvcrt.dll._fmode"]
pub static FMODE: AtomicI32 = AtomicI32::new(0x4000);
#[export_name = "peekgame.msvcrt.dll._commode"]
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
pub unsafe extern "win64" fn dll_on_exit(
    func: usize,
    pbegin: *mut *mut usize,
    pend: *mut *mut usize,
) -> usize {
    if func == 0 || pbegin.is_null() || pend.is_null() {
        return 0;
    }
    let cur_begin = unsafe { *pbegin };
    let cur_end = unsafe { *pend };
    let count = if cur_begin.is_null() || cur_end.is_null() || cur_end < cur_begin {
        0
    } else {
        unsafe { cur_end.offset_from(cur_begin) as usize }
    };

    if count % 32 == 0 {
        let new_cap = count + 32;
        let new_buf = unsafe {
            realloc(
                if cur_begin.is_null() {
                    std::ptr::null_mut()
                } else {
                    cur_begin.cast()
                },
                new_cap * std::mem::size_of::<usize>(),
            )
        } as *mut usize;
        if new_buf.is_null() {
            return 0;
        }
        unsafe {
            *pbegin = new_buf;
            *pend = new_buf.add(count);
        }
    }

    unsafe {
        let end = *pend;
        *end = func;
        *pend = end.add(1);
    }
    func
}
pub unsafe extern "win64" fn init_term(
    mut begin: *const Option<Callback>,
    end: *const Option<Callback>,
) {
    // Unreal Engine games (such as Palworld) may include plugins or telemetry
    // whose C++ global constructors call FCommandLine::Get() before WinMain runs
    // FCommandLine::Set(). If FCommandLine::bIsInitialized is false (0), initialize it
    // with the process command line so early queries succeed instead of raising LowLevelFatalError.
    unsafe {
        if let Some(process) = crate::host::process::PROCESS.get() {
            if process.image_base == 0x140000000
                && crate::images::by_address(0x1434826b4).is_some()
                && crate::images::by_address(0x149427570).is_some()
            {
                let check_insn = 0x1434826b4 as *const [u8; 7];
                if *check_insn == [0x80, 0x3d, 0xad, 0x4e, 0xfa, 0x05, 0x00] {
                    let b_init = 0x149427568 as *mut u8;
                    let cmd_buf = 0x149427570 as *mut u16;
                    if *b_init == 0 {
                        eprintln!("Pre-initializing Unreal Engine FCommandLine...");
                        let wcmd = crate::api::system::startup::get_command_line_w();
                        let mut len = 0;
                        while *wcmd.add(len) != 0 && len < 16380 {
                            *cmd_buf.add(len) = *wcmd.add(len);
                            len += 1;
                        }
                        *cmd_buf.add(len) = 0;
                        *b_init = 1;
                        eprintln!("Pre-initialized Unreal Engine FCommandLine ({} chars)", len);
                    }
                }
            }
        }
    }
    while begin < end {
        if let Some(callback) = unsafe { *begin } {
            callback();
        }
        begin = unsafe { begin.add(1) };
    }
}
pub extern "win64" fn cexit() {
    run_thread_local_callbacks(0);
    run_callbacks();
    unsafe {
        fflush(std::ptr::null_mut());
    }
}
pub extern "win64" fn c_exit() {
    // Quick exit cleanup without running atexit / termination callbacks.
}
pub extern "win64" fn exit(code: i32) -> ! {
    eprintln!("CRT exit called with code {code}");
    let rsp_val: usize;
    unsafe { std::arch::asm!("mov {}, rsp", out(reg) rsp_val); }
    let ptr = rsp_val as *const usize;
    for i in 0..32 {
        let val = unsafe { ptr.add(i).read() };
        if let Some(m) = crate::images::try_by_address(val) {
            eprintln!("  exit_stack[{i}] = {val:#x} ({}+{:#x})", m.name, val - m.base);
        } else if val >= 0x140000000 && val < 0x150000000 {
            eprintln!("  exit_stack[{i}] = {val:#x} (Palworld+{:#x})", val - 0x140000000);
        }
    }
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
pub extern "win64" fn query_app_type() -> i32 {
    APP_TYPE.load(Ordering::Relaxed)
}
pub extern "win64" fn matherr(callback: usize) {
    *MATHERR.lock().unwrap() = callback;
}
pub extern "win64" fn seh_filter_exe(_code: u32, _info: usize) -> i32 {
    0 // EXCEPTION_CONTINUE_SEARCH
}
pub type TlsCallback = extern "win64" fn(*mut c_void, u32, *mut c_void);
static TLS_CALLBACKS: Mutex<Vec<TlsCallback>> = Mutex::new(Vec::new());

pub extern "win64" fn register_thread_local_exe_atexit_callback(
    callback: Option<TlsCallback>,
) -> i32 {
    let Some(callback) = callback else {
        return -1;
    };
    TLS_CALLBACKS.lock().unwrap().push(callback);
    0
}

pub fn run_thread_local_callbacks(reason: u32) {
    let callbacks: Vec<TlsCallback> = TLS_CALLBACKS.lock().unwrap().clone();
    let base = crate::host::process::PROCESS
        .get()
        .map(|p| p.image_base as *mut c_void)
        .unwrap_or(std::ptr::null_mut());
    for callback in callbacks.iter().rev() {
        callback(base, reason, std::ptr::null_mut());
    }
}
struct Args {
    strings: Vec<CString>,
    pointers: Vec<usize>,
    environment: Vec<CString>,
    envp: Vec<usize>,
}
static ARGS: OnceLock<Args> = OnceLock::new();
#[export_name = "peekgame.msvcrt.dll.__initenv"]
pub(crate) static CRT_INITENV: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
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

/// Stable, NUL-terminated snapshot created with the CRT argument/environment state.
pub extern "win64" fn initial_narrow_environment() -> *const *const c_char {
    args().envp.as_ptr().cast()
}

pub extern "win64" fn get_narrow_winmain_command_line() -> *const c_char {
    static WINMAIN_CMD: OnceLock<CString> = OnceLock::new();
    WINMAIN_CMD
        .get_or_init(|| {
            let line = crate::host::process::PROCESS
                .get()
                .expect("process configured")
                .command_line
                .to_bytes();
            let mut idx = 0;
            while idx < line.len() && (line[idx] == b' ' || line[idx] == b'\t') {
                idx += 1;
            }
            if idx < line.len() && line[idx] == b'"' {
                idx += 1;
                while idx < line.len() && line[idx] != b'"' {
                    idx += 1;
                }
                if idx < line.len() && line[idx] == b'"' {
                    idx += 1;
                }
            } else {
                while idx < line.len() && line[idx] != b' ' && line[idx] != b'\t' {
                    idx += 1;
                }
            }
            while idx < line.len() && (line[idx] == b' ' || line[idx] == b'\t') {
                idx += 1;
            }
            CString::new(&line[idx..]).unwrap_or_default()
        })
        .as_ptr()
}

pub extern "win64" fn get_wide_winmain_command_line() -> *const u16 {
    static WINMAIN_WCMD: OnceLock<Vec<u16>> = OnceLock::new();
    WINMAIN_WCMD
        .get_or_init(|| {
            let wline: Vec<u16> = crate::host::process::PROCESS
                .get()
                .expect("process configured")
                .command_line
                .to_string_lossy()
                .encode_utf16()
                .collect();
            let mut idx = 0;
            while idx < wline.len() && (wline[idx] == b' ' as u16 || wline[idx] == b'\t' as u16) {
                idx += 1;
            }
            if idx < wline.len() && wline[idx] == b'"' as u16 {
                idx += 1;
                while idx < wline.len() && wline[idx] != b'"' as u16 {
                    idx += 1;
                }
                if idx < wline.len() && wline[idx] == b'"' as u16 {
                    idx += 1;
                }
            } else {
                while idx < wline.len() && wline[idx] != b' ' as u16 && wline[idx] != b'\t' as u16 {
                    idx += 1;
                }
            }
            while idx < wline.len() && (wline[idx] == b' ' as u16 || wline[idx] == b'\t' as u16) {
                idx += 1;
            }
            let mut rem = wline[idx..].to_vec();
            rem.push(0);
            rem
        })
        .as_ptr()
}


#[cfg(test)]
mod tests {
    use super::*;

    static LOG: Mutex<Vec<(usize, u32)>> = Mutex::new(Vec::new());

    extern "win64" fn cb1(_base: *mut c_void, reason: u32, _res: *mut c_void) {
        LOG.lock().unwrap().push((1, reason));
    }
    extern "win64" fn cb2(_base: *mut c_void, reason: u32, _res: *mut c_void) {
        LOG.lock().unwrap().push((2, reason));
    }

    #[test]
    fn test_tls_callback_order_and_c_exit() {
        assert_eq!(register_thread_local_exe_atexit_callback(None), -1);
        assert_eq!(register_thread_local_exe_atexit_callback(Some(cb1)), 0);
        assert_eq!(register_thread_local_exe_atexit_callback(Some(cb2)), 0);

        // c_exit must not invoke callbacks
        c_exit();
        assert!(LOG.lock().unwrap().is_empty());

        // DLL_THREAD_DETACH = 3 executed in LIFO order (cb2 before cb1)
        run_thread_local_callbacks(3);
        let log = LOG.lock().unwrap().clone();
        let entries: Vec<(usize, u32)> = log.into_iter().filter(|(id, _)| *id == 1 || *id == 2).collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], (2, 3));
        assert_eq!(entries[1], (1, 3));
    }
}
