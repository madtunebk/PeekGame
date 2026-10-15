//! Startup for a relinked image. `peekgame --relink` turns the PE sections into ELF
//! PT_LOAD segments at their preferred base and lets the static linker fill the IAT
//! with `peekgame.<dll>.<name>` symbols, so no PE mapping or import binding is left
//! for run time. What remains is the Windows process and thread environment.
use crate::{mapped, pe};
use std::ffi::{c_char, CStr, CString};

/// Entry called by the generated `main` of a relinked executable.
/// # Safety
/// `argv` must hold `argc` readable C strings. `image_base` must be the address the
/// image was linked at and `entry_rva` its PE entry point.
#[no_mangle]
pub unsafe extern "C" fn peekgame_relinked_main(
    argc: i32,
    argv: *const *const c_char,
    image_base: usize,
    entry_rva: u32,
) -> i32 {
    let args = (0..argc.max(0) as usize)
        .map(|i| {
            unsafe { CStr::from_ptr(*argv.add(i)) }
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    match unsafe { start(&args, image_base, entry_rva) } {
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

unsafe fn start(args: &[String], base: usize, entry_rva: u32) -> Result<(), String> {
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
    // The static linker merges neighbouring sections into one segment (read-only
    // data ends up writable), so restore the per-section rights the PE declares.
    unsafe { mapped::protect(&image, base) }?;
    crate::configure_windows_threads(base, tls.data.clone(), tls.callbacks.clone())?;
    crate::initialize_windows_thread(base, tls.data)?;
    mapped::notify(base, &tls.callbacks, 1);
    let entry: extern "win64" fn() = unsafe { std::mem::transmute(base + entry_rva as usize) };
    entry();
    Ok(())
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
