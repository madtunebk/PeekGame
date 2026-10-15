pub(crate) mod backend;
mod mapping;
mod traps;
use crate::pe;
use std::{ffi::CString, fs, path::Path};
pub fn run(path: &Path, args: &[String], output: &Path) -> Result<(), String> {
    run_with(path, args, output, None)
}
pub fn run_with(
    path: &Path,
    args: &[String],
    output: &Path,
    backend: Option<&backend::Backend>,
) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let image = pe::parse(&bytes)?;
    if image.imports.iter().any(|i| i.delayed) {
        return Err("delay imports are not supported by this runner yet".into());
    }
    let mapped = mapping::Mapping::new(&bytes, &image)?;
    let trace = output.join("dynamic-imports.tsv");
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    if let Some(backend) = backend {
        backend.trace(&trace)?;
    } else {
        game_imports::start_import_trace(&trace).map_err(|e| e.to_string())?;
    }
    let line = std::iter::once(path.to_string_lossy().into_owned())
        .chain(args.iter().cloned())
        .map(|a| game_imports::quote_windows_argument(&a))
        .collect::<Vec<_>>()
        .join(" ");
    let line = CString::new(line).map_err(|e| e.to_string())?;
    let code = if let Some(backend) = backend {
        backend.configure(mapped.base(), &line)
    } else {
        unsafe { game_imports::peekgame_configure_process(mapped.base(), line.as_ptr()) }
    };
    if code != 0 {
        return Err(format!("process configuration failed: {code}"));
    }
    let tls = unsafe { game_imports::mapped::tls(&image, mapped.base()) }?;
    if backend.is_some() && !tls.data.is_empty() {
        return Err("packaged TLS setup is not supported yet".into());
    }
    let mut unresolved = Vec::new();
    let mut slots = Vec::new();
    for import in &image.imports {
        let Some(name) = &import.name else {
            return Err("ordinal static imports are not supported yet".into());
        };
        let dll = CString::new(import.dll.as_str()).map_err(|e| e.to_string())?;
        let symbol = CString::new(name.as_str()).map_err(|e| e.to_string())?;
        let address = if let Some(backend) = backend {
            backend.resolve(&dll, &symbol)
        } else {
            unsafe { game_imports::peekgame_resolve(dll.as_ptr(), symbol.as_ptr()) }
        };
        if address.is_null() {
            unresolved.push(format!("{}!{name}", import.dll));
            slots.push(import.iat_rva);
        } else {
            mapped.bind(import.iat_rva, address as usize)?;
        }
    }
    let traps = traps::Traps::new(unresolved)?;
    for (index, rva) in slots.iter().enumerate() {
        mapped.bind(*rva, traps.address(index))?;
    }
    if backend.is_none() {
        game_imports::configure_windows_threads(
            mapped.base(),
            tls.data.clone(),
            tls.callbacks.clone(),
        )?;
        game_imports::initialize_windows_thread(mapped.base(), tls.data.clone())?;
    }
    mapped.protect(&image)?;
    eprintln!(
        "PeekGame: entering {} at {:#x}; dynamic requests -> {}",
        path.display(),
        mapped.base() + image.entry_rva as usize,
        trace.display()
    );
    for &address in &tls.callbacks {
        eprintln!("TLS callback {address:#x}: DLL_PROCESS_ATTACH");
    }
    game_imports::mapped::notify(mapped.base(), &tls.callbacks, 1);
    unsafe {
        mapped.enter(&image)?;
    }
    Ok(())
}
