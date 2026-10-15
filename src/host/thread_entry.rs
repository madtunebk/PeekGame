//! A return boundary for ExitThread. Jumping back here avoids pthread_exit's
//! forced unwind across Windows frames and lets the Rust owner clean up normally.
use std::cell::Cell;
thread_local! { static BOUNDARY: Cell<usize> = const { Cell::new(0) }; }
#[repr(C, align(16))]
struct Boundary([usize; 7]);
std::arch::global_asm!(
    ".globl peekgame_thread_call",
    "peekgame_thread_call:",
    "mov [rdx], rsp",
    "mov [rdx+8], rbx",
    "mov [rdx+16], rbp",
    "mov [rdx+24], r12",
    "mov [rdx+32], r13",
    "mov [rdx+40], r14",
    "mov [rdx+48], r15",
    "mov r12, rdx",
    "mov rcx, rsi",
    "sub rsp, 40",
    "call rdi",
    "mov rdi, r12",
    "mov esi, eax",
    ".globl peekgame_thread_return",
    "peekgame_thread_return:",
    "mov eax, esi",
    "mov rsp, [rdi]",
    "mov rbx, [rdi+8]",
    "mov rbp, [rdi+16]",
    "mov r12, [rdi+24]",
    "mov r13, [rdi+32]",
    "mov r14, [rdi+40]",
    "mov r15, [rdi+48]",
    "ret",
);
unsafe extern "C" {
    fn peekgame_thread_call(
        start: super::super::api::threading::threads::Start,
        parameter: usize,
        boundary: *mut Boundary,
    ) -> u32;
    fn peekgame_thread_return(boundary: usize, code: u32) -> !;
}
pub fn run(start: crate::api::threading::threads::Start, parameter: usize) -> u32 {
    let mut boundary = Boundary([0; 7]);
    BOUNDARY.with(|slot| slot.set(&mut boundary as *mut Boundary as usize));
    let result = unsafe { peekgame_thread_call(start, parameter, &mut boundary) };
    BOUNDARY.with(|slot| slot.set(0));
    result
}
pub extern "win64" fn exit(code: u32) -> ! {
    let boundary = BOUNDARY.with(Cell::get);
    if boundary == 0 {
        eprintln!("ExitThread: calling thread has no Windows thread entry boundary");
        std::process::exit(78);
    }
    unsafe { peekgame_thread_return(boundary, code) }
}
pub extern "win64" fn free_and_exit(module: usize, code: u32) -> ! {
    if module != 0 {
        crate::api::system::dynamic::free_library(module);
    }
    exit(code)
}
