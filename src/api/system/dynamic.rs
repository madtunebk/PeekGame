use crate::{api::system::error, images, registry};
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
/// First handle of PeekGame's own DLLs; each gets a distinct 16-byte step. They are
/// not PE images, so they sit apart from every mapped module.
const BUILTIN_HANDLES: usize = 0x50470000;
/// Handle of a DLL PeekGame implements; it counts as always loaded.
pub(crate) fn builtin_handle(name: &str) -> usize {
    let name = images::normalize(name)
        .unwrap_or_else(|| name.into())
        .to_ascii_lowercase();
    let mut list = modules().lock().unwrap();
    let index = match list.iter().position(|s| s == &name) {
        Some(index) => index,
        None => {
            list.push(name.clone());
            list.len() - 1
        }
    };
    BUILTIN_HANDLES + index * 16
}
pub(crate) fn builtin_name(handle: usize) -> Option<String> {
    let list = modules().lock().unwrap();
    handle
        .checked_sub(BUILTIN_HANDLES)
        .filter(|v| v % 16 == 0)
        .and_then(|v| list.get(v / 16))
        .cloned()
}

// LoadLibraryEx flags.
const DONT_RESOLVE_DLL_REFERENCES: u32 = 0x1;
const AS_DATAFILE: u32 = 0x2;
const WITH_ALTERED_SEARCH_PATH: u32 = 0x8;
const AS_IMAGE_RESOURCE: u32 = 0x20;
const AS_DATAFILE_EXCLUSIVE: u32 = 0x40;
const SEARCH_SYSTEM32: u32 = 0x800;

/// LoadLibraryEx: PeekGame's DLLs first, then a program DLL from disk.
fn load(name: &str, flags: u32) -> usize {
    let shown = images::normalize(name)
        .unwrap_or_else(|| name.into())
        .to_ascii_lowercase();
    let record = |what: &str, status: &str| record(&shown, what, status);
    if flags
        & (DONT_RESOLVE_DLL_REFERENCES | AS_DATAFILE | AS_IMAGE_RESOURCE | AS_DATAFILE_EXCLUSIVE)
        != 0
    {
        record("<LoadLibraryEx data/unresolved>", "unsupported_flags");
        error::set(50);
        return 0;
    }
    if crate::api::steam::recognizes(name) {
        if crate::api::steam::open() {
            record("<LoadLibrary>", "steam_bridge");
            return crate::api::steam::base_address();
        }
        record("<LoadLibrary>", "module_not_found");
        error::set(images::ERROR_MOD_NOT_FOUND);
        return 0;
    }
    if images::builtin(name) {
        record("<LoadLibrary>", "runtime_module");
        return builtin_handle(name);
    }
    // System32 holds only the DLLs PeekGame implements and their replacements.
    if flags & SEARCH_SYSTEM32 != 0
        && flags & !(SEARCH_SYSTEM32 | WITH_ALTERED_SEARCH_PATH) == 0
        && !images::is_replacement(name)
    {
        record("<LoadLibrary>", "module_not_found");
        error::set(images::ERROR_MOD_NOT_FOUND);
        return 0;
    }
    match images::load_library(name, flags & WITH_ALTERED_SEARCH_PATH != 0) {
        Ok(base) => {
            record("<LoadLibrary>", "pe_module");
            base
        }
        Err(code) => {
            record("<LoadLibrary>", "module_not_found");
            error::set(code);
            0
        }
    }
}
pub(crate) unsafe fn narrow(name: *const c_char) -> Option<String> {
    if name.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(name) }
        .to_str()
        .ok()
        .map(str::to_owned)
}
pub(crate) unsafe fn wide(name: *const u16) -> Option<String> {
    if name.is_null() {
        return None;
    }
    let length = (0..).take_while(|&i| unsafe { *name.add(i) } != 0).count();
    String::from_utf16(unsafe { std::slice::from_raw_parts(name, length) }).ok()
}
pub unsafe extern "win64" fn load_library_a(name: *const c_char) -> usize {
    unsafe { load_library_ex_a(name, 0, 0) }
}
pub unsafe extern "win64" fn load_library_w(name: *const u16) -> usize {
    unsafe { load_library_ex_w(name, 0, 0) }
}
pub unsafe extern "win64" fn load_library_ex_a(
    name: *const c_char,
    file: usize,
    flags: u32,
) -> usize {
    match unsafe { narrow(name) } {
        Some(name) if file == 0 => load(&name, flags),
        _ => {
            error::set(87);
            0
        }
    }
}
pub unsafe extern "win64" fn load_library_ex_w(name: *const u16, file: usize, flags: u32) -> usize {
    match unsafe { wide(name) } {
        Some(name) if file == 0 => load(&name, flags),
        _ => {
            error::set(87);
            0
        }
    }
}
pub unsafe extern "win64" fn get_proc_address(module: usize, name: *const c_char) -> *const c_void {
    let ordinal = (name as usize <= 0xffff).then_some(name as u32);
    let text = match ordinal {
        Some(n) => format!("#{n}"),
        None => match unsafe { narrow(name) } {
            Some(text) => text,
            None => {
                error::set(87);
                return std::ptr::null();
            }
        },
    };
    let (dll, address) = if crate::api::steam::loaded() && module == crate::api::steam::base_address() {
        let address = ordinal.is_none().then(|| crate::api::steam::export(&text)).flatten();
        ("steamclient64.dll".into(), address)
    } else if let Some(dll) = builtin_name(module) {
        let address = if crate::api::steam::recognizes(&dll) {
            ordinal.is_none().then(|| crate::api::steam::export(&text)).flatten()
        } else { registry::find_import(
            &dll,
            ordinal.is_none().then_some(text.as_str()),
            ordinal.map(|o| o as u16),
        )
        .and_then(|e| e.address)
        .map(|get| get() as usize) };
        (dll, address)
    } else if let Some(pe) = images::by_handle(module) {
        let address =
            images::get_proc_address(&pe, ordinal.is_none().then_some(text.as_str()), ordinal);
        (pe.name.clone(), address)
    } else {
        error::set(6);
        return std::ptr::null();
    };
    record(
        &dll,
        &text,
        if address.is_some() {
            "implemented"
        } else {
            "not_found"
        },
    );
    if address.is_none() {
        static VERBOSE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *VERBOSE.get_or_init(|| std::env::var_os("PEEKGAME_VERBOSE").is_some()) {
            eprintln!("GetProcAddress({dll}, {text}) -> None");
        }
    }
    match address {
        Some(address) => address as *const c_void,
        None => {
            error::set(127);
            std::ptr::null()
        }
    }
}
pub extern "win64" fn free_library(module: usize) -> i32 {
    // Built-in DLLs and the Steam bridge stay loaded for the whole process.
    if builtin_name(module).is_some() || crate::api::steam::loaded() && module == crate::api::steam::base_address() {
        return 1;
    }
    match images::by_handle(module) {
        Some(pe) => {
            images::free_library(&pe);
            1
        }
        None => {
            error::set(6);
            0
        }
    }
}
pub extern "win64" fn disable_thread_library_calls(module: usize) -> i32 {
    if builtin_name(module).is_some() {
        return 1;
    }
    match images::by_handle(module) {
        Some(pe) => {
            images::disable_thread_calls(&pe);
            1
        }
        None => {
            error::set(6);
            0
        }
    }
}
pub unsafe extern "win64" fn set_dll_directory_w(path: *const u16) -> i32 {
    set_dll_directory(unsafe { wide(path) }.as_deref())
}
pub unsafe extern "win64" fn set_dll_directory_a(path: *const c_char) -> i32 {
    set_dll_directory(unsafe { narrow(path) }.as_deref())
}
pub unsafe extern "win64" fn get_dll_directory_w(n_buffer_length: u32, lp_buffer: *mut u16) -> u32 {
    let dir = images::get_dll_directory();
    let s = match dir {
        Some(s) => s,
        None => {
            error::set(0);
            if n_buffer_length > 0 && !lp_buffer.is_null() {
                unsafe { *lp_buffer = 0 };
            }
            return 0;
        }
    };
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let needed = utf16.len() as u32 + 1;
    if n_buffer_length < needed {
        return needed;
    }
    if !lp_buffer.is_null() {
        for (i, &ch) in utf16.iter().enumerate() {
            unsafe { *lp_buffer.add(i) = ch };
        }
        unsafe { *lp_buffer.add(utf16.len()) = 0 };
    }
    utf16.len() as u32
}
pub unsafe extern "win64" fn get_dll_directory_a(n_buffer_length: u32, lp_buffer: *mut u8) -> u32 {
    let dir = images::get_dll_directory();
    let s = match dir {
        Some(s) => s,
        None => {
            error::set(0);
            if n_buffer_length > 0 && !lp_buffer.is_null() {
                unsafe { *lp_buffer = 0 };
            }
            return 0;
        }
    };
    let bytes = s.as_bytes();
    let needed = bytes.len() as u32 + 1;
    if n_buffer_length < needed {
        return needed;
    }
    if !lp_buffer.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), lp_buffer, bytes.len());
            *lp_buffer.add(bytes.len()) = 0;
        }
    }
    bytes.len() as u32
}
/// NULL restores the default search; "" removes the current directory from it.
fn set_dll_directory(path: Option<&str>) -> i32 {
    if images::set_dll_directory(path) {
        1
    } else {
        error::set(3);
        0
    }
}
/// AddDllDirectory: a directory searched for program DLLs; returns its cookie.
pub unsafe extern "win64" fn add_dll_directory(path: *const u16) -> usize {
    let Some(path) = (unsafe { wide(path) }) else {
        error::set(87);
        return 0;
    };
    images::add_dll_directory(&path).unwrap_or_else(|| {
        error::set(3);
        0
    })
}
pub extern "win64" fn remove_dll_directory(cookie: usize) -> i32 {
    if images::remove_dll_directory(cookie) {
        1
    } else {
        error::set(87);
        0
    }
}
/// SetDefaultDllDirectories: accepted. PeekGame's DLLs never come from disk and
/// program DLLs are searched as described in images::search for every flag set.
pub extern "win64" fn set_default_dll_directories(flags: u32) -> i32 {
    if flags == 0 || flags & !0x1f00 != 0 {
        error::set(87);
        return 0;
    }
    1
}

/// Builtin providers already loaded through static imports or LoadLibrary.
/// Their handles belong in EnumProcessModules, just like mapped PE modules.
pub(crate) fn loaded_builtin_handles() -> Vec<usize> {
    (0..modules().lock().unwrap().len())
        .map(|index| BUILTIN_HANDLES + index * 16)
        .collect()
}
