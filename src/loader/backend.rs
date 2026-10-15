use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::Path,
};
type Configure = unsafe extern "C" fn(usize, *const c_char) -> u32;
type Resolve = unsafe extern "C" fn(*const c_char, *const c_char) -> *const c_void;
type Trace = unsafe extern "C" fn(*const c_char) -> u32;
unsafe extern "C" {
    fn dlopen(name: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
    fn dlclose(handle: *mut c_void) -> i32;
}
pub struct Backend {
    handle: *mut c_void,
    configure: Configure,
    resolve: Resolve,
    trace: Trace,
}
impl Backend {
    pub fn open(path: &Path) -> Result<Self, String> {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
        let handle = unsafe { dlopen(path.as_ptr(), 2) };
        if handle.is_null() {
            return Err(last_error());
        }
        let result = (|| {
            let symbol = |name: &CStr| {
                let p = unsafe { dlsym(handle, name.as_ptr()) };
                if p.is_null() {
                    Err(last_error())
                } else {
                    Ok(p)
                }
            };
            let version: extern "C" fn() -> u32 =
                unsafe { std::mem::transmute(symbol(c"peekgame_abi_version")?) };
            if version() != 1 {
                return Err("unsupported game_imports.so ABI version".into());
            }
            Ok(Self {
                handle,
                configure: unsafe {
                    std::mem::transmute::<*mut c_void, Configure>(symbol(
                        c"peekgame_configure_process",
                    )?)
                },
                resolve: unsafe {
                    std::mem::transmute::<*mut c_void, Resolve>(symbol(c"peekgame_resolve")?)
                },
                trace: unsafe {
                    std::mem::transmute::<*mut c_void, Trace>(symbol(
                        c"peekgame_start_import_trace",
                    )?)
                },
            })
        })();
        if result.is_err() {
            unsafe {
                dlclose(handle);
            }
        }
        result
    }
    pub fn configure(&self, base: usize, line: &CStr) -> u32 {
        unsafe { (self.configure)(base, line.as_ptr()) }
    }
    pub fn resolve(&self, dll: &CStr, name: &CStr) -> *const c_void {
        unsafe { (self.resolve)(dll.as_ptr(), name.as_ptr()) }
    }
    pub fn trace(&self, path: &Path) -> Result<(), String> {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
        let code = unsafe { (self.trace)(path.as_ptr()) };
        if code == 0 {
            Ok(())
        } else {
            Err(format!("trace initialization failed: {code}"))
        }
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
            dlclose(self.handle);
        }
    }
}
fn last_error() -> String {
    let p = unsafe { dlerror() };
    if p.is_null() {
        "runtime library load failed".into()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}
