//! Windows x64 structured exception handling (ported from peek's seh.inc,
//! exceptions.inc and function_tables.inc).
//!
//! Dispatch (RaiseException, or a hardware fault in program code): vectored handlers,
//! then each frame's language handler, found by unwinding with the function tables,
//! then the unhandled-exception filter. PeekGame's own frames (ELF code, no unwind
//! data) sit between a raise and its handlers: every dispatch records the context it
//! started from, and an unwind that reaches those frames continues from that context,
//! as if they were ntdll's dispatcher frames.
mod dispatch;
pub mod tables;
pub mod unwind;
pub use dispatch::{
    c_specific_handler, capture_context_entry, install_fault_handlers, raise_exception_entry,
    restore_context_entry, unwind_entry, unwind_ex_entry,
};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

/// Windows x64 CONTEXT (1232 bytes, 16-byte aligned).
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct Context {
    pub home: [u64; 6],
    pub flags: u32,
    pub mxcsr: u32,
    pub segments: [u16; 6],
    pub eflags: u32,
    pub debug: [u64; 6],
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rbx: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    /// FltSave (XMM_SAVE_AREA32, 512 bytes) and the vector registers that follow.
    pub extended: [u8; 1232 - 256],
}
const _: () = assert!(size_of::<Context>() == 1232 && std::mem::offset_of!(Context, rip) == 248);
impl Context {
    /// rax, rcx, rdx, rbx, rsp, rbp, rsi, rdi, r8..r15 by unwind register number.
    pub fn int_reg(&mut self, reg: usize) -> &mut u64 {
        let first = &mut self.rax as *mut u64;
        unsafe { &mut *first.add(reg & 15) }
    }
    pub fn xmm_reg(&mut self, reg: usize) -> &mut [u8; 16] {
        let at = 0xa0 + 16 * (reg & 15);
        (&mut self.extended[at..at + 16]).try_into().unwrap()
    }
}
#[repr(C)]
pub struct ExceptionRecord {
    pub code: u32,
    pub flags: u32,
    pub next: *mut ExceptionRecord,
    pub address: usize,
    pub count: u32,
    pub information: [usize; 15],
}
#[repr(C)]
pub struct ExceptionPointers {
    pub record: *mut ExceptionRecord,
    pub context: *mut Context,
}

// Vectored exception handlers: first and normal lists. Nodes are never freed, so a
// dispatch never follows a freed node while another thread removes a handler.
type Vectored = extern "win64" fn(*mut ExceptionPointers) -> i32;
struct Registration {
    callback: Vectored,
    active: AtomicBool,
}
static VECTORED: Mutex<(Vec<&'static Registration>, Vec<&'static Registration>)> =
    Mutex::new((Vec::new(), Vec::new()));
pub extern "win64" fn add_vectored_handler(first: u32, callback: Option<Vectored>) -> usize {
    let Some(callback) = callback else {
        crate::api::system::error::set(87);
        return 0;
    };
    let node: &'static Registration = Box::leak(Box::new(Registration {
        callback,
        active: AtomicBool::new(true),
    }));
    let mut lists = VECTORED.lock().unwrap();
    if first != 0 {
        lists.0.insert(0, node);
    } else {
        lists.1.push(node);
    }
    node as *const Registration as usize
}
pub extern "win64" fn remove_vectored_handler(handle: usize) -> u32 {
    let lists = VECTORED.lock().unwrap();
    match lists
        .0
        .iter()
        .chain(lists.1.iter())
        .find(|n| **n as *const Registration as usize == handle)
    {
        Some(node) if node.active.swap(false, Ordering::AcqRel) => 1,
        _ => {
            crate::api::system::error::set(6);
            0
        }
    }
}
fn vectored_handlers() -> Vec<Vectored> {
    let lists = VECTORED.lock().unwrap();
    lists
        .0
        .iter()
        .chain(lists.1.iter())
        .filter(|n| n.active.load(Ordering::Acquire))
        .map(|n| n.callback)
        .collect()
}

static FILTER: AtomicUsize = AtomicUsize::new(0);
/// SetUnhandledExceptionFilter: called for exceptions no handler takes.
pub extern "win64" fn set_unhandled_filter(filter: usize) -> usize {
    FILTER.swap(filter, Ordering::AcqRel)
}

/// UnhandledExceptionFilter: the process's top-level filter, or
/// EXCEPTION_EXECUTE_HANDLER (1) without one.
pub extern "win64" fn unhandled_exception_filter(pointers: *mut ExceptionPointers) -> i32 {
    match FILTER.load(Ordering::Acquire) {
        0 => 1,
        filter => {
            let filter: extern "win64" fn(*mut ExceptionPointers) -> i32 =
                unsafe { std::mem::transmute(filter) };
            filter(pointers)
        }
    }
}
