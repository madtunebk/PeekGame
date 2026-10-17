//! Legacy printf entry points using the existing Windows va_list formatter.
use super::{stdio, ucrt};
use std::ffi::{c_char, c_void};
pub unsafe extern "win64" fn vfprintf(
    file: *mut c_void,
    fmt: *const c_char,
    args: *const u64,
) -> i32 {
    if fmt.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    let text = unsafe { ucrt::render_narrow(fmt, args) };
    let n = unsafe { stdio::write(text.as_ptr().cast(), 1, text.len(), file) };
    if n != text.len() {
        -1
    } else {
        n as i32
    }
}
pub unsafe extern "win64" fn vfwprintf(
    file: *mut c_void,
    fmt: *const u16,
    args: *const u64,
) -> i32 {
    if fmt.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    let text = unsafe { ucrt::render_wide(fmt, args) };
    let n = unsafe { stdio::write(text.as_ptr().cast(), 1, text.len(), file) };
    if n != text.len() {
        -1
    } else {
        text.encode_utf16().count() as i32
    }
}
pub unsafe extern "win64" fn vsnprintf(
    out: *mut u8,
    n: usize,
    fmt: *const c_char,
    args: *const u64,
) -> i32 {
    if fmt.is_null() || (out.is_null() && n != 0) {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    let text = unsafe { ucrt::render_narrow(fmt, args) };
    if !out.is_null() {
        let copied = text.len().min(n);
        unsafe {
            std::ptr::copy_nonoverlapping(text.as_ptr(), out, copied);
            if copied < n {
                *out.add(copied) = 0;
            }
        }
    }
    if text.len() > n {
        -1
    } else {
        text.len() as i32
    }
}
pub unsafe extern "win64" fn vscprintf(fmt: *const c_char, args: *const u64) -> i32 {
    if fmt.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    unsafe { ucrt::render_narrow(fmt, args) }.len() as i32
}
pub unsafe extern "win64" fn vsnprintf_s(
    out: *mut u8,
    size: usize,
    count: usize,
    fmt: *const c_char,
    args: *const u64,
) -> i32 {
    if out.is_null() || size == 0 || fmt.is_null() {
        unsafe {
            *stdio::errno() = 22;
        }
        return -1;
    }
    let text = unsafe { ucrt::render_narrow(fmt, args) };
    if count != usize::MAX && count >= size && text.len() >= size {
        unsafe {
            *out = 0;
            *stdio::errno() = 34;
        }
        return -1;
    }
    let n = text.len().min(count).min(size - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    if n < text.len() {
        -1
    } else {
        n as i32
    }
}
// Windows variadic args 3 and 4 arrive in R8/R9. Spill them into their home
// slots; subsequent arguments are already contiguous on the Windows stack.
std::arch::global_asm!(
 ".text", ".globl peekgame_crt_fprintf", "peekgame_crt_fprintf:",
 "mov [rsp+24],r8", "mov [rsp+32],r9", "lea r8,[rsp+24]", "jmp {narrow}",
 ".globl peekgame_crt_fwprintf", "peekgame_crt_fwprintf:",
 "mov [rsp+24],r8", "mov [rsp+32],r9", "lea r8,[rsp+24]", "jmp {wide}",
 narrow=sym vfprintf,wide=sym vfwprintf,
);
unsafe extern "win64" {
    #[link_name = "peekgame_crt_fprintf"]
    pub fn fprintf(file: *mut c_void, fmt: *const c_char) -> i32;
    #[link_name = "peekgame_crt_fwprintf"]
    pub fn fwprintf(file: *mut c_void, fmt: *const u16) -> i32;
}
