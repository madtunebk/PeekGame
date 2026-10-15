pub(crate) mod backend;
use crate::pe;
use game_imports::images;
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
    let mapped = images::map::Mapping::new(&bytes, &image)?;
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
    if let Some(backend) = backend {
        bind_packaged(&image, &mapped, backend)?;
    } else {
        bind(path, &bytes, &mapped)?;
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
    if backend.is_none() {
        // DLLs attach before the EXE's own TLS callbacks and entry point.
        images::start_process()?;
    }
    for &address in &tls.callbacks {
        eprintln!("TLS callback {address:#x}: DLL_PROCESS_ATTACH");
    }
    game_imports::mapped::notify(mapped.base(), &tls.callbacks, 1);
    unsafe {
        mapped.enter(&image)?;
    }
    Ok(())
}

/// Binds the EXE's imports: PeekGame's implementations, exports of the PE DLLs the
/// game ships (loaded with their dependencies), or stops named after the import.
fn bind(path: &Path, bytes: &[u8], mapped: &images::map::Mapping) -> Result<(), String> {
    images::register_exe(mapped.base(), path, pe::parse(bytes)?);
    let exe = images::exe().ok_or("EXE not registered")?;
    images::with_loader_lock(|| {
        let mut providers: Vec<(String, images::Provider)> = Vec::new();
        let mut unresolved = 0;
        for import in exe.image.imports.iter().filter(|i| !i.delayed) {
            let index = match providers
                .iter()
                .position(|(d, _)| d.eq_ignore_ascii_case(&import.dll))
            {
                Some(i) => i,
                None => {
                    providers.push((import.dll.clone(), images::provider(&import.dll)));
                    providers.len() - 1
                }
            };
            let (name, ordinal) = (import.name.as_deref(), import.ordinal);
            let address = match images::lookup(&providers[index].1, &import.dll, name, ordinal) {
                Some(address) => address,
                None => {
                    unresolved += 1;
                    images::trap(&import.dll, name, ordinal)?
                }
            };
            mapped.bind(import.iat_rva, address)?;
        }
        if unresolved > 0 {
            eprintln!("PeekGame: {unresolved} static import(s) unresolved (each stops if called)");
        }
        Ok(())
    })
}

/// Package mode: imports resolve through the packaged runtime library only.
fn bind_packaged(
    image: &pe::Image,
    mapped: &images::map::Mapping,
    backend: &backend::Backend,
) -> Result<(), String> {
    for import in &image.imports {
        if import.delayed {
            return Err("package runner does not support delay imports".into());
        }
        let Some(name) = &import.name else {
            return Err("ordinal static imports are not supported yet".into());
        };
        let dll = CString::new(import.dll.as_str()).map_err(|e| e.to_string())?;
        let symbol = CString::new(name.as_str()).map_err(|e| e.to_string())?;
        let address = backend.resolve(&dll, &symbol) as usize;
        let address = if address == 0 {
            images::trap(&import.dll, Some(name), None)?
        } else {
            address
        };
        mapped.bind(import.iat_rva, address)?;
    }
    Ok(())
}
