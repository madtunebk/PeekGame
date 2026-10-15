//! Startup for a relinked image. `peekgame --relink` turns the PE sections into ELF
//! PT_LOAD segments at their preferred base and lets the static linker fill the IAT
//! with `peekgame.<dll>.<name>` symbols, so no PE mapping or import binding is left
//! for run time. What remains is the Windows process and thread environment.
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
    let (data, callbacks) = unsafe { tls_template(base) }?;
    unsafe { protect(base) }?;
    crate::configure_windows_threads(base, data.clone(), callbacks.clone())?;
    crate::initialize_windows_thread(base, data)?;
    for &address in &callbacks {
        let callback: extern "win64" fn(usize, u32, usize) =
            unsafe { std::mem::transmute(address) };
        callback(base, 1, 0);
    }
    let entry: extern "win64" fn() = unsafe { std::mem::transmute(base + entry_rva as usize) };
    entry();
    Ok(())
}

/// Applies each PE section's own protection. The static linker merges neighbouring
/// sections into one segment (read-only data ends up writable), so restore the
/// per-section rights the PE runner gives them.
unsafe fn protect(base: usize) -> Result<(), String> {
    unsafe extern "C" {
        fn mprotect(address: *mut std::ffi::c_void, size: usize, prot: i32) -> i32;
    }
    let u16_at = |va: usize| unsafe { (va as *const u16).read_unaligned() } as usize;
    let u32_at = |va: usize| unsafe { (va as *const u32).read_unaligned() };
    let pe = base + u32_at(base + 60) as usize;
    let table = pe + 24 + u16_at(pe + 20);
    for index in 0..u16_at(pe + 6) {
        let at = table + index * 40;
        let extent = u32_at(at + 8).max(u32_at(at + 16)) as usize;
        let flags = u32_at(at + 36);
        let prot = ((flags & 0x4000_0000 != 0) as i32)
            | (((flags & 0x8000_0000 != 0) as i32) << 1)
            | (((flags & 0x2000_0000 != 0) as i32) << 2);
        let start = base + u32_at(at + 12) as usize;
        if extent > 0
            && unsafe { mprotect(start as *mut _, extent.div_ceil(4096) * 4096, prot) } != 0
        {
            return Err(format!(
                "mprotect section {index}: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    Ok(())
}

/// Reads the TLS directory of the image mapped at `base`: the initial data block
/// (template plus zero fill) and the callback list. Clears the slot index to 0,
/// the main image's index under Windows.
unsafe fn tls_template(base: usize) -> Result<(Vec<u8>, Vec<usize>), String> {
    let u32_at = |va: usize| unsafe { (va as *const u32).read_unaligned() } as usize;
    let u64_at = |va: usize| unsafe { (va as *const u64).read_unaligned() } as usize;
    let opt = base + u32_at(base + 60) + 24;
    let size = u32_at(opt + 56);
    let inside =
        |va: usize, n: usize| va >= base && va.checked_add(n).is_some_and(|e| e <= base + size);
    if u32_at(opt + 108) <= 9 || u32_at(opt + 184) == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let tls = base + u32_at(opt + 184);
    if !inside(tls, 40) {
        return Err("TLS directory outside image".into());
    }
    let (start, end, index, callbacks) = (
        u64_at(tls),
        u64_at(tls + 8),
        u64_at(tls + 16),
        u64_at(tls + 24),
    );
    let raw = end.checked_sub(start).ok_or("invalid TLS template range")?;
    let total = raw
        .checked_add(u32_at(tls + 32))
        .filter(|n| *n <= 16 * 1024 * 1024)
        .ok_or("TLS template too large")?;
    if (raw > 0 && !inside(start, raw)) || !inside(index, 4) {
        return Err("TLS template outside image".into());
    }
    let mut data = vec![0; total.max(1)];
    if raw > 0 {
        data[..raw].copy_from_slice(unsafe { std::slice::from_raw_parts(start as *const u8, raw) });
    }
    unsafe { (index as *mut u32).write_unaligned(0) };
    let mut list = Vec::new();
    if callbacks != 0 {
        for i in 0..64 {
            if !inside(callbacks + i * 8, 8) {
                return Err("TLS callback array outside image".into());
            }
            match u64_at(callbacks + i * 8) {
                0 => return Ok((data, list)),
                address if inside(address, 1) => list.push(address),
                _ => return Err("TLS callback outside image".into()),
            }
        }
        return Err("unterminated TLS callback array".into());
    }
    Ok((data, list))
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
