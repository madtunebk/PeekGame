//! Stops for imports nothing provides. Each trap is a small Windows-ABI stub that
//! reports `DLL!Name` and its caller, then exits; it only runs if the program
//! actually calls the import. Stubs live in executable pages that grow on demand.
use std::{
    ffi::{c_void, CString},
    sync::Mutex,
};
unsafe extern "C" {
    fn _exit(code: i32) -> !;
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn mprotect(a: *mut c_void, n: usize, p: i32) -> i32;
}
const STUB: usize = 32;
const PAGE: usize = 4096;
struct Arena {
    page: usize,
    used: usize,
}
static ARENA: Mutex<Arena> = Mutex::new(Arena {
    page: 0,
    used: PAGE,
});

/// Address of a new stub that stops with `name` (kept for the life of the process).
pub fn make(name: &str) -> Result<usize, String> {
    let name = CString::new(name).map_err(|e| e.to_string())?;
    let mut arena = ARENA.lock().unwrap();
    if arena.used + STUB > PAGE {
        let page = unsafe { mmap(std::ptr::null_mut(), PAGE, 3, 0x22, -1, 0) };
        if page as isize == -1 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        *arena = Arena {
            page: page as usize,
            used: 0,
        };
    }
    let at = arena.page + arena.used;
    arena.used += STUB;
    // mov rcx, name; mov rdx, [rsp] (caller); mov rax, stop; jmp rax
    let mut code = vec![0x48, 0xb9];
    code.extend_from_slice(&(name.into_raw() as usize).to_le_bytes());
    code.extend_from_slice(&[0x48, 0x8b, 0x14, 0x24, 0x48, 0xb8]);
    code.extend_from_slice(&(stop as *const () as usize).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xe0]);
    unsafe {
        let page = arena.page as *mut c_void;
        if mprotect(page, PAGE, 3) != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        std::ptr::copy_nonoverlapping(code.as_ptr(), at as *mut u8, code.len());
        if mprotect(page, PAGE, 5) != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(at)
}

extern "win64" fn stop(name: *const std::ffi::c_char, caller: usize) -> ! {
    let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy();
    eprintln!("STOP: missing Windows API {name}, called from {caller:#x}");
    unsafe { _exit(78) }
}
