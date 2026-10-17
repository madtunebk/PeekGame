//! Startup for a relinked image. `peekgame --relink` turns the PE sections into ELF
//! PT_LOAD segments at their preferred base and lets the static linker fill the IAT
//! with `peekgame.<dll>.<name>` symbols. Imports from program DLLs (the PE DLLs a
//! game ships) are listed in an [`Info`] table and bound here by the PE loader;
//! the rest of startup is the Windows process and thread environment.
use crate::{images, mapped, pe};
use std::ffi::{c_char, CStr, CString};

/// Written by the relinker (`relink.rs`): the EXE's original file name and the
/// imports that come from program DLLs.
#[repr(C)]
pub struct Info {
    exe_name: *const c_char,
    count: u64,
    entries: [Entry; 0],
}
#[repr(C)]
struct Entry {
    iat_rva: u32,
    /// `u32::MAX` for a named import.
    ordinal: u32,
    dll: *const c_char,
    /// NULL for an ordinal import.
    name: *const c_char,
}

/// Entry called by the generated `main` of a relinked executable.
/// # Safety
/// `argv` must hold `argc` readable C strings. `image_base` must be the address the
/// image was linked at, `entry_rva` its PE entry point and `info` its relink table.
#[no_mangle]
pub unsafe extern "C" fn peekgame_relinked_main(
    argc: i32,
    argv: *const *const c_char,
    image_base: usize,
    entry_rva: u32,
    info: *const Info,
) -> i32 {
    let args = (0..argc.max(0) as usize)
        .map(|i| {
            unsafe { CStr::from_ptr(*argv.add(i)) }
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    match unsafe { start(&args, image_base, entry_rva, &*info) } {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("peekgame: {error}");
            1
        }
    }
}

/// Target of the IAT slots whose import had no implementation at relink time.
/// Called with the Windows ABI; the original return address stays on the stack.
#[no_mangle]
pub extern "win64" fn peekgame_missing_import(name: *const c_char, caller: usize) -> ! {
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy();
    eprintln!("STOP: missing Windows API {name}, called from {caller:#x}");
    unsafe { libc_exit(78) }
}

unsafe extern "C" {
    #[link_name = "_exit"]
    fn libc_exit(code: i32) -> !;
}

unsafe fn start(args: &[String], base: usize, entry_rva: u32, info: &Info) -> Result<(), String> {
    // DXVK is loaded from `runtime/` beside the executable unless the caller chose.
    if std::env::var_os("PEEKGAME_DXVK_DIR").is_none() {
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("runtime")))
        {
            std::env::set_var("PEEKGAME_DXVK_DIR", dir);
        }
    }
    if let Some(path) = std::env::var_os("PEEKGAME_IMPORT_TRACE") {
        crate::start_import_trace(std::path::Path::new(&path)).map_err(|e| e.to_string())?;
    }
    let line = args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ");
    let line = CString::new(line).map_err(|e| e.to_string())?;
    let code = unsafe { crate::peekgame_configure_process(base, line.as_ptr()) };
    if code != 0 {
        return Err(format!("process configuration failed: {code}"));
    }
    let size = unsafe { pe::mapped_size(base as *const u8) };
    let image = pe::parse_mapped(unsafe { std::slice::from_raw_parts(base as *const u8, size) })?;
    let tls = unsafe { mapped::tls(&image, base) }?;
    // The EXE as a module, named as on Windows and placed beside this executable, so
    // program DLLs and data are found next to it.
    let exe_name = unsafe { CStr::from_ptr(info.exe_name) }
        .to_string_lossy()
        .into_owned();
    let directory = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = directory.parent().ok_or("executable has no directory")?;
    images::register_exe(base, &directory.join(exe_name), image.clone());
    unsafe { bind_program_dlls(base, info) }?;
    // The static linker merges neighbouring sections into one segment (read-only
    // data ends up writable), so restore the per-section rights the PE declares.
    unsafe { mapped::protect(&image, base) }?;
    crate::configure_windows_threads(base, tls.data.clone(), tls.callbacks.clone())?;
    crate::initialize_windows_thread(base, tls.data)?;
    // DLLs attach before the EXE's own TLS callbacks and entry point.
    images::start_process()?;
    mapped::notify(base, &tls.callbacks, 1);
    let entry: extern "win64" fn() = unsafe { std::mem::transmute(base + entry_rva as usize) };
    entry();
    Ok(())
}

/// Loads the program DLLs the EXE imports from and writes their addresses into its IAT.
unsafe fn bind_program_dlls(base: usize, info: &Info) -> Result<(), String> {
    let entries = unsafe { std::slice::from_raw_parts(info.entries.as_ptr(), info.count as usize) };
    // ELF maps .rdata read-only immediately. The loader must temporarily write
    // program-DLL IAT slots before mapped::protect restores the PE permissions.
    unsafe extern "C" {
        fn mprotect(address: *mut std::ffi::c_void, size: usize, prot: i32) -> i32;
    }
    let module = images::by_handle(base).ok_or("main image is not registered")?;
    for entry in entries {
        let rva = entry.iat_rva as usize;
        let section = module
            .image
            .sections
            .iter()
            .find(|section| {
                rva >= section.rva as usize
                    && rva
                        .checked_add(8)
                        .is_some_and(|end| end <= section.rva as usize + section.extent() as usize)
            })
            .ok_or("runtime IAT slot outside image sections")?;
        let start = (base + rva) & !4095;
        let end = (base + rva + 8).div_ceil(4096) * 4096;
        if unsafe { mprotect(start as *mut _, end - start, section.protection() | 2) } != 0 {
            return Err(format!(
                "cannot make runtime IAT writable: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    images::with_loader_lock(|| {
        let mut providers: Vec<(String, images::Provider)> = Vec::new();
        for entry in entries {
            let dll = unsafe { CStr::from_ptr(entry.dll) }
                .to_string_lossy()
                .into_owned();
            let name = (!entry.name.is_null()).then(|| {
                unsafe { CStr::from_ptr(entry.name) }
                    .to_string_lossy()
                    .into_owned()
            });
            let ordinal = (entry.ordinal != u32::MAX).then_some(entry.ordinal as u16);
            let index = match providers
                .iter()
                .position(|(d, _)| d.eq_ignore_ascii_case(&dll))
            {
                Some(i) => i,
                None => {
                    providers.push((dll.clone(), images::provider(&dll)));
                    providers.len() - 1
                }
            };
            let address = match images::lookup(&providers[index].1, &dll, name.as_deref(), ordinal)
            {
                Some(address) => address,
                None => images::trap(&dll, name.as_deref(), ordinal)?,
            };
            unsafe { ((base + entry.iat_rva as usize) as *mut usize).write_unaligned(address) };
        }
        Ok(())
    })
}

/// Quotes one argument the way `CommandLineToArgvW` splits it back.
pub fn quote(s: &str) -> String {
    if !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '"') {
        return s.to_owned();
    }
    let mut out = String::from("\"");
    let mut slashes = 0;
    for ch in s.chars() {
        if ch == '\\' {
            slashes += 1;
            continue;
        }
        if ch == '"' {
            out.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            out.push_str(&"\\".repeat(slashes));
        }
        slashes = 0;
        out.push(ch);
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}
