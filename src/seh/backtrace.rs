//! Capture Windows frames using PE unwind metadata, stopping at the host boundary.
use super::{tables, unwind, Context};
std::arch::global_asm!(
    ".text",
    ".p2align 4",
    ".globl peekgame_capture_stack_backtrace",
    "peekgame_capture_stack_backtrace:",
    // Shadow space (32), fifth argument (8), padding (8), CONTEXT (1232), alignment.
    "sub rsp, 0x508",
    "lea r11, [rsp+0x30]",
    "mov r10d, 154",
    "xor eax, eax",
    "2:",
    "mov [r11], rax",
    "add r11, 8",
    "dec r10d",
    "jnz 2b",
    "mov [rsp+0x30], rcx",
    "mov [rsp+0x38], rdx",
    "mov [rsp+0x40], r8",
    "mov [rsp+0x48], r9",
    "lea rcx, [rsp+0x30]",
    "call {capture}",
    // Replace the capture helper's return site with the actual Windows caller.
    "mov rax, [rsp+0x508]",
    "mov [rsp+0x128], rax",
    "lea rax, [rsp+0x510]",
    "mov [rsp+0xc8], rax",
    "lea rcx, [rsp+0x30]",
    "mov edx, [rsp+0x30]",
    "mov r8d, [rsp+0x38]",
    "mov r9, [rsp+0x40]",
    "mov rax, [rsp+0x48]",
    "mov [rsp+0x20], rax",
    "call {walk}",
    "add rsp, 0x508",
    "ret",
    capture = sym super::capture_context_entry,
    walk = sym walk,
);
unsafe extern "win64" {
    #[link_name = "peekgame_capture_stack_backtrace"]
    pub fn capture_entry(skip: u32, count: u32, frames: *mut usize, hash: *mut u32) -> u16;
}
// Give the extern its explicit assembly entry name through a Rust declaration below.

const MAX_DEPTH: u32 = 4096;
unsafe extern "win64" fn walk(
    context: *mut Context,
    skip: u32,
    count: u32,
    frames: *mut usize,
    hash: *mut u32,
) -> u16 {
    if !hash.is_null() {
        unsafe { hash.write_unaligned(0) };
    }
    if frames.is_null() || count == 0 || skip >= MAX_DEPTH {
        return 0;
    }
    let Some((low, high)) = crate::host::teb::stack_bounds() else {
        return 0;
    };
    let context = unsafe { &mut *context };
    let mut captured = 0u16;
    let mut sum = 0u32;
    let limit = count.min(u16::MAX as u32);
    for depth in 0..MAX_DEPTH {
        if context.rip == 0 || context.rsp < low || context.rsp > high.saturating_sub(8) {
            break;
        }
        let mut base = 0;
        let function = unsafe { tables::lookup(context.rip, &mut base, 0) };
        if function.is_null() && crate::images::by_address(context.rip as usize).is_none() {
            break;
        }
        if depth >= skip {
            let address = context.rip as usize;
            unsafe { frames.add(captured as usize).write_unaligned(address) };
            sum = sum.wrapping_add(address as u32);
            captured += 1;
            if u32::from(captured) >= limit {
                break;
            }
        }
        let previous = context.rsp;
        if function.is_null() {
            context.rip = unsafe { (context.rsp as *const u64).read_unaligned() };
            context.rsp += 8;
        } else {
            let mut data = 0;
            let mut frame = 0;
            unsafe {
                unwind::virtual_unwind(
                    0,
                    base,
                    context.rip,
                    function,
                    context,
                    &mut data,
                    &mut frame,
                    0,
                )
            };
        }
        if context.rsp <= previous {
            break;
        }
    }
    if !hash.is_null() {
        unsafe { hash.write_unaligned(sum) };
    }
    captured
}
