//! Program PE images: the EXE and the DLLs a game ships (UnityPlayer.dll, Mono, ...).
//! Ported from peek's `pe_modules.inc` and following its rules:
//! - DLLs PeekGame implements (the registry) always win and are never read from disk;
//!   any other name is searched in the EXE's directory, the SetDllDirectory directory
//!   (if set), then the current directory, case-insensitively.
//! - A module is mapped at its preferred base or relocated, its imports are bound
//!   (loading PE dependencies recursively), its static TLS gets a slot in every
//!   thread, and its TLS callbacks and DllMain run in dependency order once the
//!   process has a TEB.
//! - The loader lock (recursive) serializes loading, unloading and every DllMain
//!   call, like the Windows loader lock; the module list has its own leaf lock so
//!   lookups never wait for a DllMain.
mod lock;
pub mod map;
mod search;
pub mod tls;
pub mod traps;
use crate::{pe, registry};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
};

pub struct Module {
    pub base: usize,
    pub size: usize,
    /// File name as found on disk.
    pub name: String,
    pub path: PathBuf,
    pub image: pe::Image,
    /// Keeps the image mapped; `None` for an image mapped by someone else (the EXE).
    _mapping: Option<map::Mapping>,
    refs: AtomicU32,
    pinned: AtomicBool,
    attached: AtomicBool,
    thread_calls: AtomicBool,
    tls_slot: Option<usize>,
    tls_callbacks: Vec<usize>,
    deps: Mutex<Vec<Arc<Module>>>,
}
impl Module {
    fn is_dll(&self) -> bool {
        self.image.is_dll()
    }
    pub fn contains(&self, address: usize) -> bool {
        address >= self.base && address - self.base < self.size
    }
}

/// Load order, the EXE first.
static MODULES: Mutex<Vec<Arc<Module>>> = Mutex::new(Vec::new());
static LOADER: lock::Recursive = lock::Recursive::new();
static STARTED: AtomicBool = AtomicBool::new(false);
/// Mapped but not yet attached, dependencies first.
static PENDING: Mutex<Vec<Arc<Module>>> = Mutex::new(Vec::new());

pub const ERROR_MOD_NOT_FOUND: u32 = 126;
pub const ERROR_BAD_EXE_FORMAT: u32 = 193;
pub const ERROR_DLL_INIT_FAILED: u32 = 1114;

/// Is `dll` one PeekGame implements (never loaded from disk)?
/// Is `dll` in the replacement directory?
pub fn is_replacement(dll: &str) -> bool {
    search::replacement(dll).is_some()
}
/// A DLL found in the replacement directory (Windows builds of libraries such as
/// DXVK's d3d11.dll and dxgi.dll) is loaded from there instead.
pub fn builtin(dll: &str) -> bool {
    registry::provides(dll) && search::replacement(dll).is_none()
}

/// The file name LoadLibrary means by `name` (default `.dll` extension).
pub fn normalize(name: &str) -> Option<String> {
    search::normalize(name)
}
pub fn by_handle(handle: usize) -> Option<Arc<Module>> {
    MODULES
        .lock()
        .unwrap()
        .iter()
        .find(|m| m.base == handle)
        .cloned()
}
pub fn by_address(address: usize) -> Option<Arc<Module>> {
    MODULES
        .lock()
        .unwrap()
        .iter()
        .find(|m| m.contains(address))
        .cloned()
}
/// by_address without waiting: None if the module list is busy (signal handlers).
pub fn try_by_address(address: usize) -> Option<Arc<Module>> {
    MODULES
        .try_lock()
        .ok()?
        .iter()
        .find(|m| m.contains(address))
        .cloned()
}
pub fn by_name(name: &str) -> Option<Arc<Module>> {
    let wanted = search::normalize(name)?;
    MODULES
        .lock()
        .unwrap()
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(&wanted))
        .cloned()
}
/// Every loaded image, in load order (the EXE first).
pub fn all() -> Vec<Arc<Module>> {
    MODULES.lock().unwrap().clone()
}
pub fn exe() -> Option<Arc<Module>> {
    MODULES.lock().unwrap().first().cloned()
}

/// Registers the EXE, mapped by the runner at `base`. Its exports serve
/// GetProcAddress on the EXE and DLLs that import from it.
pub fn register_exe(base: usize, path: &Path, image: pe::Image) {
    let module = Module {
        base,
        size: (image.image_size as usize).next_multiple_of(4096),
        name: file_name(path),
        path: path.to_owned(),
        image,
        _mapping: None,
        refs: AtomicU32::new(1),
        pinned: AtomicBool::new(true),
        attached: AtomicBool::new(true),
        thread_calls: AtomicBool::new(false),
        tls_slot: None,
        tls_callbacks: Vec::new(),
        deps: Mutex::new(Vec::new()),
    };
    search::set_exe_directory(path.parent());
    MODULES.lock().unwrap().insert(0, Arc::new(module));
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Where an import resolves: a PeekGame implementation, an export of a loaded PE
/// module, or a trap that stops with the name if called.
pub enum Provider {
    Builtin,
    Module(Arc<Module>),
    Missing,
}
/// Finds the provider for imports from `dll` (loading it if it is a PE DLL).
/// Must be called with the loader lock held (see [`with_loader_lock`]).
pub fn provider(dll: &str) -> Provider {
    if builtin(dll) {
        return Provider::Builtin;
    }
    match load_locked(dll, None) {
        Ok(module) => Provider::Module(module),
        Err(error) => {
            eprintln!(
                "PeekGame: {dll} is not available (error {error}); its imports stop if called"
            );
            Provider::Missing
        }
    }
}
/// The address `provider` gives for an import, if it has one.
pub fn lookup(
    provider: &Provider,
    dll: &str,
    name: Option<&str>,
    ordinal: Option<u16>,
) -> Option<usize> {
    match provider {
        Provider::Builtin => registry::find_import(dll, name, ordinal)
            .and_then(|e| e.address)
            .map(|get| get() as usize),
        Provider::Module(module) => export(module, name, ordinal.map(u32::from), 0),
        Provider::Missing => None,
    }
}
/// A stop for an import nothing provides.
pub fn trap(dll: &str, name: Option<&str>, ordinal: Option<u16>) -> Result<usize, String> {
    traps::make(&match name {
        Some(n) => format!("{dll}!{n}"),
        None => format!("{dll}!#{}", ordinal.unwrap_or(0)),
    })
}

/// Runs `f` holding the loader lock.
pub fn with_loader_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = LOADER.lock();
    f()
}

/// An export by name or ordinal; a forwarder loads (or resolves into) its target.
pub fn export(
    module: &Arc<Module>,
    name: Option<&str>,
    ordinal: Option<u32>,
    depth: u32,
) -> Option<usize> {
    let exports = module.image.exports.as_ref()?;
    let entry = match name {
        Some(n) => exports
            .entries
            .iter()
            .find(|e| e.name.as_deref() == Some(n)),
        None => exports.entries.iter().find(|e| Some(e.ordinal) == ordinal),
    }?;
    match &entry.target {
        pe::ExportTarget::Rva(rva) => Some(module.base + *rva as usize),
        pe::ExportTarget::Forward(text) => forward(module, text, depth),
    }
}
/// `DLL.Name` or `DLL.#Ordinal`, into PeekGame's own DLLs or a PE module that
/// `from` then keeps loaded.
fn forward(from: &Arc<Module>, text: &str, depth: u32) -> Option<usize> {
    let (dll, function) = text.rsplit_once('.')?;
    if dll.is_empty() || depth > 8 {
        return None;
    }
    let dll = format!("{dll}.dll");
    let ordinal = function.strip_prefix('#').and_then(|n| n.parse().ok());
    let name = if ordinal.is_some() {
        None
    } else {
        Some(function)
    };
    if builtin(&dll) {
        return registry::find(&dll, name?)
            .and_then(|e| e.address)
            .map(|get| get() as usize);
    }
    let target = load_locked(&dll, None).ok()?;
    from.deps.lock().unwrap().push(target.clone());
    export(&target, name, ordinal, depth + 1)
}

/// Finds or loads `name`; a loaded module gains a reference. Loader lock held.
fn load_locked(name: &str, first_dir: Option<&Path>) -> Result<Arc<Module>, u32> {
    if let Some(module) = by_name(name) {
        module.refs.fetch_add(1, Ordering::Relaxed);
        return Ok(module);
    }
    let path = search::find(name, first_dir).ok_or(ERROR_MOD_NOT_FOUND)?;
    let bytes = std::fs::read(&path).map_err(|_| ERROR_MOD_NOT_FOUND)?;
    let bad = |why: String| {
        eprintln!(
            "LoadLibrary: {}: {why} (ERROR_BAD_EXE_FORMAT)",
            path.display()
        );
        ERROR_BAD_EXE_FORMAT
    };
    let image = pe::parse(&bytes).map_err(bad)?;
    let mapping = map::Mapping::new(&bytes, &image).map_err(bad)?;
    let (base, size) = (mapping.base(), mapping.size());
    // TLS before protection: the index variable may sit in a read-only section.
    let tls = unsafe { crate::mapped::tls(&image, base) }.map_err(bad)?;
    let tls_slot = image
        .tls
        .is_some()
        .then(|| crate::images::tls::add_module(tls.data));
    if let Some(slot) = tls_slot {
        let index = (image.tls.as_ref().unwrap().index - image.image_base) as usize;
        mapping.bind_u32(index as u32, slot as u32).map_err(bad)?;
    }
    let module = Arc::new(Module {
        base,
        size,
        name: file_name(&path),
        path: std::fs::canonicalize(&path).unwrap_or(path.clone()),
        image,
        _mapping: Some(mapping),
        refs: AtomicU32::new(1),
        pinned: AtomicBool::new(false),
        attached: AtomicBool::new(false),
        thread_calls: AtomicBool::new(true),
        tls_slot,
        tls_callbacks: tls.callbacks,
        deps: Mutex::new(Vec::new()),
    });
    eprintln!(
        "LoadLibrary: mapped {} at {base:#x} ({size:#x} bytes)",
        module.path.display()
    );
    // Listed before its imports are bound, so circular imports find it.
    MODULES.lock().unwrap().push(module.clone());
    if let Err(why) = bind_imports(&module) {
        let error = bad(why);
        unload(&module);
        return Err(error);
    }
    let mapping = module._mapping.as_ref().unwrap();
    if let Err(why) = mapping.protect(&module.image) {
        let error = bad(why);
        unload(&module);
        return Err(error);
    }
    PENDING.lock().unwrap().push(module.clone());
    Ok(module)
}

fn bind_imports(module: &Arc<Module>) -> Result<(), String> {
    let mapping = module._mapping.as_ref().unwrap();
    let mut providers: Vec<(String, Provider)> = Vec::new();
    let mut unresolved = Vec::new();
    for import in &module.image.imports {
        if import.delayed {
            continue; // bound by the image's own delay-load helper through LoadLibrary
        }
        let index = match providers
            .iter()
            .position(|(d, _)| d.eq_ignore_ascii_case(&import.dll))
        {
            Some(i) => i,
            None => {
                let provider = provider(&import.dll);
                if let Provider::Module(m) = &provider {
                    module.deps.lock().unwrap().push(m.clone());
                }
                providers.push((import.dll.clone(), provider));
                providers.len() - 1
            }
        };
        let provider = &providers[index].1;
        let address = match lookup(
            provider,
            &import.dll,
            import.name.as_deref(),
            import.ordinal,
        ) {
            Some(address) => address,
            None => {
                unresolved.push(format!(
                    "{}!{}",
                    import.dll,
                    import
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("#{}", import.ordinal.unwrap_or(0)))
                ));
                trap(&import.dll, import.name.as_deref(), import.ordinal)?
            }
        };
        mapping.bind(import.iat_rva, address)?;
    }
    if !unresolved.is_empty() {
        eprintln!(
            "{}: {} imports, {} unresolved (each stops if called): {}",
            module.name,
            module.image.imports.len(),
            unresolved.len(),
            unresolved.join(" ")
        );
    }
    Ok(())
}

fn call_entry(module: &Module, reason: u32) -> bool {
    if !module.is_dll() || module.image.entry_rva == 0 {
        return true;
    }
    let entry: extern "win64" fn(usize, u32, usize) -> i32 =
        unsafe { std::mem::transmute(module.base + module.image.entry_rva as usize) };
    entry(module.base, reason, 0) != 0
}

/// DLL_PROCESS_ATTACH for every module mapped since the last call, dependencies
/// first. Returns the module whose DllMain failed. Loader lock held.
fn initialize_pending() -> Result<(), Arc<Module>> {
    if !STARTED.load(Ordering::Acquire) {
        return Ok(());
    }
    let pending = std::mem::take(&mut *PENDING.lock().unwrap());
    for module in pending {
        if module.attached.swap(true, Ordering::AcqRel) {
            continue;
        }
        eprintln!("{}: DLL_PROCESS_ATTACH", module.name);
        crate::mapped::notify(module.base, &module.tls_callbacks, 1);
        if !call_entry(&module, 1) {
            eprintln!(
                "{}: DllMain(DLL_PROCESS_ATTACH) returned FALSE (ERROR_DLL_INIT_FAILED)",
                module.name
            );
            module.attached.store(false, Ordering::Release);
            return Err(module);
        }
    }
    Ok(())
}

/// Marks the process started (the main thread has its TEB) and attaches every
/// DLL loaded so far. Called once by the runner before the EXE entry point.
pub fn start_process() -> Result<(), String> {
    let _guard = LOADER.lock();
    STARTED.store(true, Ordering::Release);
    initialize_pending().map_err(|m| format!("{}: DllMain failed", m.name))
}

/// DLL_THREAD_ATTACH (2) / DLL_THREAD_DETACH (3) for the calling thread.
pub fn thread_notify(reason: u32) {
    let modules: Vec<_> = MODULES.lock().unwrap().iter().skip(1).cloned().collect();
    if modules.is_empty() {
        return;
    }
    let _guard = LOADER.lock();
    for module in modules {
        if !module.attached.load(Ordering::Acquire) || !module.is_dll() {
            continue;
        }
        crate::mapped::notify(module.base, &module.tls_callbacks, reason);
        if module.thread_calls.load(Ordering::Relaxed) {
            call_entry(&module, reason);
        }
    }
}

fn unload(module: &Arc<Module>) {
    if module.attached.swap(false, Ordering::AcqRel) {
        crate::mapped::notify(module.base, &module.tls_callbacks, 0);
        call_entry(module, 0);
    }
    PENDING.lock().unwrap().retain(|m| !Arc::ptr_eq(m, module));
    MODULES.lock().unwrap().retain(|m| !Arc::ptr_eq(m, module));
    if let Some(slot) = module.tls_slot {
        tls::remove_module(slot);
    }
    eprintln!("FreeLibrary: unloaded {}", module.name);
    for dep in std::mem::take(&mut *module.deps.lock().unwrap()) {
        release(&dep);
    }
}
fn release(module: &Arc<Module>) {
    if module.pinned.load(Ordering::Relaxed) || !module.is_dll() {
        return;
    }
    if module.refs.fetch_sub(1, Ordering::AcqRel) == 1 {
        unload(module);
    }
}

/// LoadLibrary for a DLL PeekGame does not implement. `altered` is
/// LOAD_WITH_ALTERED_SEARCH_PATH: a path's directory is searched first.
pub fn load_library(name: &str, altered: bool) -> Result<usize, u32> {
    let first_dir = altered
        .then(|| search::host_path(name))
        .flatten()
        .and_then(|p| p.parent().map(Path::to_owned));
    let _guard = LOADER.lock();
    let module = load_locked(name, first_dir.as_deref())?;
    if initialize_pending().is_err() {
        release(&module);
        return Err(ERROR_DLL_INIT_FAILED);
    }
    Ok(module.base)
}
/// GetProcAddress on a PE module (`ordinal` when the name is an ordinal).
pub fn get_proc_address(
    module: &Arc<Module>,
    name: Option<&str>,
    ordinal: Option<u32>,
) -> Option<usize> {
    let _guard = LOADER.lock();
    let address = export(module, name, ordinal, 0);
    if address.is_some() {
        let _ = initialize_pending(); // modules a forwarder loaded
    }
    address
}
pub fn free_library(module: &Arc<Module>) {
    let _guard = LOADER.lock();
    release(module);
}
/// GetModuleHandleEx reference rules: PIN (1) keeps it for good; without
/// UNCHANGED_REFCOUNT (2) the caller gains a reference.
pub fn handle_reference(module: &Module, flags: u32) {
    if flags & 1 != 0 {
        module.pinned.store(true, Ordering::Relaxed);
    } else if flags & 2 == 0 {
        module.refs.fetch_add(1, Ordering::Relaxed);
    }
}
pub fn disable_thread_calls(module: &Module) {
    module.thread_calls.store(false, Ordering::Relaxed);
}
pub fn add_dll_directory(path: &str) -> Option<usize> {
    let _guard = LOADER.lock();
    search::add_directory(path)
}
pub fn remove_dll_directory(cookie: usize) -> bool {
    let _guard = LOADER.lock();
    search::remove_directory(cookie)
}
pub fn set_dll_directory(path: Option<&str>) -> bool {
    let _guard = LOADER.lock();
    search::set_dll_directory(path)
}
