use crate::{api::system::error, registry};
use std::{
    collections::BTreeMap,
    ffi::{c_char, c_void, CStr},
    fs::File,
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
};
static MODULES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static TRACE: OnceLock<Mutex<Trace>> = OnceLock::new();
struct Trace {
    file: File,
    counts: BTreeMap<(String, String, String), usize>,
}
pub fn start(path: &Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "dll\texport\tstatus\trequests")?;
    TRACE
        .set(Mutex::new(Trace {
            file,
            counts: BTreeMap::new(),
        }))
        .map_err(|_| std::io::Error::other("trace already initialized"))
}
fn record(dll: &str, name: &str, status: &str) {
    if let Some(trace) = TRACE.get() {
        let mut t = trace.lock().unwrap();
        let key = (dll.to_owned(), name.to_owned(), status.to_owned());
        let count = t.counts.entry(key).or_default();
        *count += 1;
        let count = *count;
        if let Err(e) =
            writeln!(t.file, "{dll}\t{name}\t{status}\t{count}").and_then(|_| t.file.flush())
        {
            eprintln!("dynamic import trace write failed: {e}");
        }
    }
}
fn modules() -> &'static Mutex<Vec<String>> {
    MODULES.get_or_init(|| Mutex::new(Vec::new()))
}
fn load(name: &str) -> usize {
    let mut name = name.to_ascii_lowercase();
    if !name.ends_with(".dll") {
        name.push_str(".dll");
    }
    if !registry::entries().any(|e| e.dll == name) {
        record(&name, "<LoadLibrary>", "module_not_found");
        error::set(126);
        return 0;
    }
    let mut list = modules().lock().unwrap();
    let index = if let Some(index) = list.iter().position(|s| s == &name) {
        index
    } else {
        list.push(name.clone());
        list.len() - 1
    };
    record(&name, "<LoadLibrary>", "runtime_module");
    0x50470000 + index * 16
}
pub unsafe extern "win64" fn load_library_a(name: *const c_char) -> usize {
    if name.is_null() {
        error::set(87);
        return 0;
    }
    match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => load(s),
        Err(_) => {
            error::set(87);
            0
        }
    }
}
pub unsafe extern "win64" fn load_library_w(name: *const u16) -> usize {
    if name.is_null() {
        error::set(87);
        return 0;
    }
    let mut text = Vec::new();
    for i in 0..32768 {
        let ch = unsafe { *name.add(i) };
        if ch == 0 {
            return match String::from_utf16(&text) {
                Ok(s) => load(&s),
                Err(_) => {
                    error::set(87);
                    0
                }
            };
        }
        text.push(ch);
    }
    error::set(87);
    0
}
pub unsafe extern "win64" fn get_proc_address(module: usize, name: *const c_char) -> *const c_void {
    let dll = {
        let list = modules().lock().unwrap();
        module
            .checked_sub(0x50470000)
            .filter(|v| v % 16 == 0)
            .and_then(|v| list.get(v / 16))
            .cloned()
    };
    let Some(dll) = dll else {
        error::set(6);
        return std::ptr::null();
    };
    if name as usize <= 65535 {
        record(&dll, &format!("#{}", name as usize), "ordinal_unsupported");
        error::set(127);
        return std::ptr::null();
    }
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        error::set(87);
        return std::ptr::null();
    };
    let address = registry::find(&dll, name)
        .and_then(|e| e.address)
        .map(|get| get())
        .unwrap_or(std::ptr::null());
    record(
        &dll,
        name,
        if address.is_null() {
            "not_found"
        } else {
            "implemented"
        },
    );
    if address.is_null() {
        error::set(127);
    }
    address
}
