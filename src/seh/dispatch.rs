//! Exception dispatch, unwinding, __C_specific_handler and hardware exceptions.
//! The entry points that need the caller's exact register state are assembly stubs
//! that build a CONTEXT describing the caller and call the Rust side.
use super::{
    tables::{self, RuntimeFunction},
    unwind::{virtual_unwind, UNW_FLAG_EHANDLER, UNW_FLAG_UHANDLER},
    vectored_handlers, Context, ExceptionPointers, ExceptionRecord, FILTER,
};
use crate::images;
use std::{cell::Cell, sync::atomic::Ordering};

const EH_NONCONTINUABLE: u32 = 0x01;
const EH_UNWINDING: u32 = 0x02;
const EH_EXIT_UNWIND: u32 = 0x04;
const EH_STACK_INVALID: u32 = 0x08;
const EH_NESTED_CALL: u32 = 0x10;
const EH_TARGET_UNWIND: u32 = 0x20;
const EH_COLLIDED_UNWIND: u32 = 0x40;
const CONTINUE_EXECUTION: i32 = 0;
const CONTINUE_SEARCH: i32 = 1;
const NESTED: i32 = 2;
const COLLIDED: i32 = 3;

#[repr(C)]
pub struct DispatcherContext {
    control_pc: u64,
    image_base: u64,
    function: *mut RuntimeFunction,
    establisher_frame: u64,
    target_ip: u64,
    context: *mut Context,
    language_handler: usize,
    handler_data: usize,
    history: usize,
    scope_index: u32,
    fill: u32,
}
const _: () = assert!(size_of::<DispatcherContext>() == 80);
type LanguageHandler =
    extern "win64" fn(*mut ExceptionRecord, u64, *mut Context, *mut DispatcherContext) -> i32;

// Offsets in CONTEXT used by the stubs: rax 0x78 .. r15 0xf0, rsp 0x98, rip 0xf8,
// eflags 0x44, mxcsr 0x34, segments 0x38.., FltSave 0x100, ContextFlags 0x30.
std::arch::global_asm!(
    ".macro SEH_SAVE_INTEGER base",
    "mov %rax,0x78(\\base)",
    "mov %rcx,0x80(\\base)",
    "mov %rdx,0x88(\\base)",
    "mov %rbx,0x90(\\base)",
    "mov %rbp,0xa0(\\base)",
    "mov %rsi,0xa8(\\base)",
    "mov %rdi,0xb0(\\base)",
    "mov %r8,0xb8(\\base)",
    "mov %r9,0xc0(\\base)",
    "mov %r10,0xc8(\\base)",
    "mov %r11,0xd0(\\base)",
    "mov %r12,0xd8(\\base)",
    "mov %r13,0xe0(\\base)",
    "mov %r14,0xe8(\\base)",
    "mov %r15,0xf0(\\base)",
    ".endm",
    ".macro SEH_SAVE_REST base",
    "pushfq",
    "pop %rax",
    "mov %eax,0x44(\\base)",
    "fxsave 0x100(\\base)",
    "stmxcsr 0x34(\\base)",
    "mov %cs,0x38(\\base)",
    "mov %ds,0x3a(\\base)",
    "mov %es,0x3c(\\base)",
    "mov %fs,0x3e(\\base)",
    "mov %gs,0x40(\\base)",
    "mov %ss,0x42(\\base)",
    "movl $0x10000f,0x30(\\base)",
    ".endm",
    // A stub entered by a Windows call: CONTEXT on its own stack describing the caller
    // at the return address (Rip = return address, Rsp = the caller's after return).
    ".macro SEH_CALLER_STUB name, target",
    ".text",
    ".p2align 4",
    ".globl \\name",
    "\\name:",
    "sub $0x4d8,%rsp",
    "SEH_SAVE_INTEGER %rsp",
    "lea 0x4e0(%rsp),%rax",
    "mov %rax,0x98(%rsp)",
    "mov 0x4d8(%rsp),%rax",
    "mov %rax,0xf8(%rsp)",
    "SEH_SAVE_REST %rsp",
    "mov %rsp,%rdi",
    "call \\target",
    "ud2",
    ".endm",
    "SEH_CALLER_STUB peekgame_seh_raise_exception, peekgame_seh_raise",
    "SEH_CALLER_STUB peekgame_seh_rtl_unwind_ex, peekgame_seh_unwind_ex",
    "SEH_CALLER_STUB peekgame_seh_rtl_unwind, peekgame_seh_unwind",
    // RtlCaptureContext(rcx): the caller's state at the return address.
    ".text",
    ".p2align 4",
    ".globl peekgame_seh_capture_context",
    "peekgame_seh_capture_context:",
    "SEH_SAVE_INTEGER %rcx",
    "lea 8(%rsp),%rax",
    "mov %rax,0x98(%rcx)",
    "mov (%rsp),%rax",
    "mov %rax,0xf8(%rcx)",
    "SEH_SAVE_REST %rcx",
    "mov 0x78(%rcx),%rax",
    "ret",
    // RtlRestoreContext(rcx) / peekgame_seh_restore(rdi): load every register and
    // continue at Rip on Rsp. Rip, Rdi and EFlags go just below the target stack.
    ".text",
    ".p2align 4",
    ".globl peekgame_seh_restore_context",
    "peekgame_seh_restore_context:",
    "mov %rcx,%rdi",
    ".globl peekgame_seh_restore",
    "peekgame_seh_restore:",
    "fxrstor 0x100(%rdi)",
    "ldmxcsr 0x34(%rdi)",
    "mov 0x98(%rdi),%rax",
    "mov 0xf8(%rdi),%rcx",
    "mov %rcx,-8(%rax)",
    "mov 0xb0(%rdi),%rcx",
    "mov %rcx,-16(%rax)",
    "mov 0x44(%rdi),%ecx",
    "mov %rcx,-24(%rax)",
    "mov 0x88(%rdi),%rdx",
    "mov 0x90(%rdi),%rbx",
    "mov 0xa0(%rdi),%rbp",
    "mov 0xa8(%rdi),%rsi",
    "mov 0xb8(%rdi),%r8",
    "mov 0xc0(%rdi),%r9",
    "mov 0xc8(%rdi),%r10",
    "mov 0xd0(%rdi),%r11",
    "mov 0xd8(%rdi),%r12",
    "mov 0xe0(%rdi),%r13",
    "mov 0xe8(%rdi),%r14",
    "mov 0xf0(%rdi),%r15",
    "lea -24(%rax),%rsp",
    "mov 0x78(%rdi),%rax",
    "mov 0x80(%rdi),%rcx",
    "popfq",
    "pop %rdi",
    "ret",
    // A hardware fault continues here on the faulting thread's own stack.
    ".text",
    ".p2align 4",
    ".globl peekgame_seh_hardware_dispatch",
    "peekgame_seh_hardware_dispatch:",
    "and $-16,%rsp",
    "call peekgame_seh_hardware",
    "ud2",
    options(att_syntax)
);
unsafe extern "win64" {
    #[link_name = "peekgame_seh_raise_exception"]
    pub fn raise_exception_entry(code: u32, flags: u32, count: u32, arguments: *const u64);
    #[link_name = "peekgame_seh_rtl_unwind_ex"]
    pub fn unwind_ex_entry(
        frame: u64,
        target: u64,
        record: *mut ExceptionRecord,
        value: u64,
        context: *mut Context,
        history: usize,
    );
    #[link_name = "peekgame_seh_rtl_unwind"]
    pub fn unwind_entry(frame: u64, target: u64, record: *mut ExceptionRecord, value: u64);
    #[link_name = "peekgame_seh_capture_context"]
    pub fn capture_context_entry(context: *mut Context);
    #[link_name = "peekgame_seh_restore_context"]
    pub fn restore_context_entry(context: *mut Context, record: *mut ExceptionRecord);
}
unsafe extern "C" {
    fn peekgame_seh_restore(context: *const Context) -> !;
    fn peekgame_seh_hardware_dispatch();
}

// One per running dispatch or unwind on this thread: where it started.
thread_local! {static STARTS: Cell<Vec<usize>> = const { Cell::new(Vec::new()) };}
fn push_start(context: *const Context) {
    STARTS.with(|s| {
        let mut v = s.take();
        v.push(context as usize);
        s.set(v);
    });
}
fn pop_start() {
    STARTS.with(|s| {
        let mut v = s.take();
        v.pop();
        s.set(v);
    });
}
fn starts() -> Vec<usize> {
    STARTS.with(|s| {
        let v = s.take();
        s.set(v.clone());
        v
    })
}
/// StackBase (gs:0x8) or StackLimit (gs:0x10) from the TEB; 0 without one.
fn teb_field(offset: usize) -> u64 {
    let mut gs: u64 = 0;
    unsafe extern "C" {
        fn syscall(number: i64, ...) -> i64;
    }
    if unsafe { syscall(158, 0x1004i32, &mut gs) } != 0 || gs == 0 {
        return 0;
    }
    unsafe { ((gs as usize + offset) as *const u64).read() }
}

/// One frame up from `c`, filling `d`. False: no frame (end of the Windows frames).
fn step(kind: u32, d: &mut DispatcherContext, c: &mut Context) -> bool {
    d.control_pc = c.rip;
    d.language_handler = 0;
    d.handler_data = 0;
    let mut base = 0u64;
    let f = unsafe { tables::lookup(c.rip, &mut base, 0) };
    if !f.is_null() {
        d.image_base = base;
        d.function = f;
        d.language_handler = unsafe {
            virtual_unwind(
                kind,
                base,
                c.rip,
                f,
                c,
                &mut d.handler_data,
                &mut d.establisher_frame,
                0,
            )
        };
        return true;
    }
    d.image_base = 0;
    d.function = std::ptr::null_mut();
    if images::by_address(c.rip as usize).is_some() {
        // Leaf function: only the return address is on the stack.
        d.establisher_frame = c.rsp;
        c.rip = unsafe { (c.rsp as *const u64).read() };
        c.rsp += 8;
        return true;
    }
    // PeekGame's frames: continue where the dispatch began.
    for start in starts().into_iter().rev() {
        let start = unsafe { &*(start as *const Context) };
        if c.rsp < start.rsp {
            *c = *start;
            d.establisher_frame = c.rsp;
            return true;
        }
    }
    false
}
fn fatal(record: &ExceptionRecord, context: &Context, what: &str) -> ! {
    eprintln!(
        "STOP: {what}: exception {:#010x} at {} (flags {:#x}, {} parameters)",
        record.code,
        describe(context.rip as usize),
        record.flags,
        record.count
    );
    unsafe extern "C" {
        fn _exit(code: i32) -> !;
    }
    unsafe { _exit(134) }
}
/// "module+0xOFFSET" or the address.
pub(crate) fn describe(address: usize) -> String {
    match images::try_by_address(address) {
        Some(m) => format!("{}+{:#x}", m.name, address - m.base),
        None => format!("{address:#x}"),
    }
}
fn call_vectored(record: &mut ExceptionRecord, context: &mut Context) -> bool {
    let mut pointers = ExceptionPointers { record, context };
    vectored_handlers()
        .into_iter()
        .any(|h| h(&mut pointers) == -1)
}
/// The frame handlers, innermost first. True: a handler continued execution.
fn call_frame_handlers(record: &mut ExceptionRecord, original: &mut Context) -> bool {
    push_start(original);
    let mut c = *original;
    let mut d: DispatcherContext = unsafe { std::mem::zeroed() };
    d.context = &mut c;
    let stack_base = teb_field(0x8);
    let mut result = false;
    while step(UNW_FLAG_EHANDLER, &mut d, &mut c) {
        if d.establisher_frame == 0
            || d.establisher_frame & 7 != 0
            || (stack_base != 0 && d.establisher_frame >= stack_base)
        {
            record.flags |= EH_STACK_INVALID;
            break;
        }
        if d.language_handler != 0 {
            d.scope_index = 0;
            let handler: LanguageHandler = unsafe { std::mem::transmute(d.language_handler) };
            match handler(record, d.establisher_frame, original, &mut d) {
                CONTINUE_EXECUTION => {
                    if record.flags & EH_NONCONTINUABLE != 0 {
                        fatal(
                            record,
                            original,
                            "a handler continued a noncontinuable exception",
                        );
                    }
                    result = true;
                    break;
                }
                NESTED => record.flags |= EH_NESTED_CALL,
                CONTINUE_SEARCH => {}
                _ => fatal(
                    record,
                    original,
                    "collided unwind during dispatch is not supported",
                ),
            }
        }
        if stack_base != 0 && c.rsp >= stack_base {
            break;
        }
    }
    pop_start();
    result
}
fn dispatch(record: &mut ExceptionRecord, context: &mut Context) -> ! {
    eprintln!(
        "SEH dispatch: exception {:#010x} at {} (flags {:#x}, {} params: {:016x?})",
        record.code,
        describe(context.rip as usize),
        record.flags,
        record.count,
        &record.information[..record.count.min(4) as usize]
    );
    if record.code == 0xe000_0008 {
        for i in 0..32 {
            let val = unsafe { (context.rsp as *const u64).add(i).read_unaligned() } as usize;
            let desc = describe(val);
            if desc.contains('+') {
                eprintln!("  oom_stack[{i}] = {val:#x} ({desc})");
            }
        }
    }
    if (record.code == 0xC06D_007E || record.code == 0xC06D_007F)
        && record.count >= 1
        && record.information[0] != 0
    {
        let info = record.information[0] as *const u8;
        unsafe {
            let dll = info.add(24).cast::<*const std::ffi::c_char>().read_unaligned();
            let by_name = info.add(32).cast::<u32>().read_unaligned() != 0;
            let name = info.add(40).cast::<usize>().read_unaligned();
            let dll_name = if !dll.is_null() {
                std::ffi::CStr::from_ptr(dll).to_string_lossy().into_owned()
            } else {
                "unknown".into()
            };
            let what = if by_name && name != 0 {
                std::ffi::CStr::from_ptr(name as *const std::ffi::c_char)
                    .to_string_lossy()
                    .into_owned()
            } else {
                format!("#{name}")
            };
            eprintln!("DELAY-LOAD FAILURE: {dll_name}!{what} (exception {:#010x})", record.code);
        }
    }
    if call_vectored(record, context) || call_frame_handlers(record, context) {
        unsafe { peekgame_seh_restore(context) }
    }
    let filter = FILTER.load(Ordering::Acquire);
    if filter != 0 {
        // A delay-load failure (VcppException 0xC06D007E/7F) names the DLL and import.
        if (record.code == 0xC06D_007E || record.code == 0xC06D_007F)
            && record.count >= 1
            && record.information[0] != 0
        {
            let info = record.information[0] as *const u8;
            unsafe {
                let dll = info
                    .add(24)
                    .cast::<*const std::ffi::c_char>()
                    .read_unaligned();
                let by_name = info.add(32).cast::<u32>().read_unaligned() != 0;
                let name = info.add(40).cast::<usize>().read_unaligned();
                if !dll.is_null() {
                    let what = if by_name && name != 0 {
                        std::ffi::CStr::from_ptr(name as *const std::ffi::c_char)
                            .to_string_lossy()
                            .into_owned()
                    } else {
                        format!("#{name}")
                    };
                    eprintln!(
                        "PeekGame: delay-loaded {}!{} is missing",
                        std::ffi::CStr::from_ptr(dll).to_string_lossy(),
                        what
                    );
                }
            }
        }
        // The program's crash handler takes over from here: say what it caught.
        eprintln!(
            "PeekGame: unhandled exception {:#010x} at {} ({}), passed to the program's filter",
            record.code,
            describe(record.address),
            (0..record.count as usize)
                .map(|i| format!("{:#x}", record.information[i]))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let filter: extern "win64" fn(*mut ExceptionPointers) -> i32 =
            unsafe { std::mem::transmute(filter) };
        let mut pointers = ExceptionPointers { record, context };
        match filter(&mut pointers) {
            -1 => unsafe { peekgame_seh_restore(context) },
            1 => {
                // EXCEPTION_EXECUTE_HANDLER: the process ends.
                eprintln!(
                    "unhandled exception {:#010x} at {}: the program's filter ended the process",
                    record.code,
                    describe(record.address)
                );
                unsafe extern "C" {
                    fn _exit(code: i32) -> !;
                }
                unsafe { _exit((record.code & 255) as i32) }
            }
            _ => {}
        }
    }
    fatal(record, context, "unhandled exception")
}
/// RaiseException. MS_VC_EXCEPTION (0x406D1388) names a thread for a debugger; as
/// with a debugger attached, the name goes to the Linux thread and the call returns.
#[no_mangle]
unsafe extern "C" fn peekgame_seh_raise(context: *mut Context) -> ! {
    let context = unsafe { &mut *context };
    let (code, flags, count) = (context.rcx as u32, context.rdx as u32, context.r8 as u32);
    let arguments = context.r9 as *const u64;
    if code == 0x406D_1388
        && !arguments.is_null()
        && count >= 3
        && unsafe { *arguments } as u32 == 0x1000
    {
        let name = unsafe { *arguments.add(1) } as *const std::ffi::c_char;
        let tid = unsafe { *arguments.add(2) } as u32;
        if !name.is_null() && tid == u32::MAX {
            let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_bytes();
            let _ = std::fs::write("/proc/thread-self/comm", &name[..name.len().min(15)]);
        }
        unsafe { peekgame_seh_restore(context) }
    }
    let count = if arguments.is_null() {
        0
    } else {
        count.min(15)
    };
    let mut record = ExceptionRecord {
        code,
        flags: flags & EH_NONCONTINUABLE,
        next: std::ptr::null_mut(),
        address: context.rip as usize,
        count,
        information: [0; 15],
    };
    for i in 0..count as usize {
        record.information[i] = unsafe { *arguments.add(i) } as usize;
    }
    dispatch(&mut record, context)
}
/// RtlUnwindEx: calls the termination handlers of every frame up to `target_frame`,
/// then continues at `target_ip` in that frame with Rax = value. A target frame of
/// 0 unwinds every frame (exit unwind).
fn unwind(
    start: &Context,
    target_frame: u64,
    target_ip: u64,
    record: *mut ExceptionRecord,
    value: u64,
) -> ! {
    let mut local = ExceptionRecord {
        code: 0xC000_0027, // STATUS_UNWIND
        flags: 0,
        next: std::ptr::null_mut(),
        address: start.rip as usize,
        count: 0,
        information: [0; 15],
    };
    let record = if record.is_null() {
        &mut local
    } else {
        unsafe { &mut *record }
    };
    record.flags |= EH_UNWINDING | if target_frame == 0 { EH_EXIT_UNWIND } else { 0 };
    push_start(start);
    let mut current = *start;
    let mut d: DispatcherContext = unsafe { std::mem::zeroed() };
    d.target_ip = target_ip;
    loop {
        let mut next = current;
        if !step(UNW_FLAG_UHANDLER, &mut d, &mut next) {
            fatal(record, start, "unwind target frame not found");
        }
        if d.establisher_frame == 0 || d.establisher_frame & 7 != 0 {
            fatal(record, start, "invalid frame during unwind");
        }
        if d.language_handler != 0 {
            if target_frame != 0 && d.establisher_frame > target_frame {
                fatal(record, start, "unwind went past its target frame");
            }
            if d.establisher_frame == target_frame {
                record.flags |= EH_TARGET_UNWIND;
            }
            d.context = &mut current;
            d.scope_index = 0;
            let handler: LanguageHandler = unsafe { std::mem::transmute(d.language_handler) };
            if handler(record, d.establisher_frame, &mut current, &mut d) == COLLIDED {
                fatal(record, start, "collided unwind is not supported");
            }
            record.flags &= !EH_COLLIDED_UNWIND;
        }
        if target_frame != 0 && d.establisher_frame == target_frame {
            break;
        }
        current = next;
    }
    current.rax = value;
    current.rip = target_ip;
    // The dispatches whose frames are being abandoned end here.
    STARTS.with(|s| {
        let mut v = s.take();
        v.retain(|&c| unsafe { (*(c as *const Context)).rsp } > current.rsp);
        s.set(v);
    });
    unsafe { peekgame_seh_restore(&current) }
}
#[no_mangle]
unsafe extern "C" fn peekgame_seh_unwind_ex(caller: *mut Context) -> ! {
    let c = unsafe { &*caller };
    unwind(c, c.rcx, c.rdx, c.r8 as *mut ExceptionRecord, c.r9)
}
#[no_mangle]
unsafe extern "C" fn peekgame_seh_unwind(caller: *mut Context) -> ! {
    let c = unsafe { &*caller };
    unwind(c, c.rcx, c.rdx, c.r8 as *mut ExceptionRecord, c.r9)
}

/// __C_specific_handler: the MSVC handler for __try/__except/__finally scope tables
/// (HandlerData: Count, then {Begin, End, Handler, JumpTarget} RVAs).
pub unsafe extern "win64" fn c_specific_handler(
    record: *mut ExceptionRecord,
    frame: u64,
    context: *mut Context,
    d: *mut DispatcherContext,
) -> i32 {
    let (record, d) = unsafe { (&mut *record, &mut *d) };
    let table = d.handler_data as *const u32;
    let count = unsafe { *table };
    let scope = |i: u32| unsafe { std::slice::from_raw_parts(table.add(1 + 4 * i as usize), 4) };
    let (base, pc) = (d.image_base, d.control_pc);
    let covers = |s: &[u32]| pc >= base + s[0] as u64 && pc < base + s[1] as u64;
    if record.flags & (EH_UNWINDING | EH_EXIT_UNWIND) != 0 {
        for i in d.scope_index..count {
            let s = scope(i);
            if !covers(s) || s[3] != 0 {
                continue;
            }
            if record.flags & EH_TARGET_UNWIND != 0
                && d.target_ip >= base + s[0] as u64
                && d.target_ip < base + s[1] as u64
            {
                break;
            }
            d.scope_index = i + 1;
            let handler: extern "win64" fn(u8, u64) =
                unsafe { std::mem::transmute(base + s[2] as u64) };
            handler(1, frame);
        }
        return CONTINUE_SEARCH;
    }
    for i in d.scope_index..count {
        let s = scope(i);
        if !covers(s) || s[3] == 0 {
            continue;
        }
        if s[2] != 1 {
            // A filter function (1 is EXCEPTION_EXECUTE_HANDLER as a constant).
            let filter: extern "win64" fn(*mut ExceptionPointers, u64) -> i32 =
                unsafe { std::mem::transmute(base + s[2] as u64) };
            let mut pointers = ExceptionPointers { record, context };
            match filter(&mut pointers, frame) {
                -1 => return CONTINUE_EXECUTION,
                0 => continue,
                _ => {}
            }
        }
        // Unwind from the dispatch's start: PeekGame's frames have no unwind data.
        let start = starts().last().map_or(d.context, |&c| c as *mut Context);
        unwind(
            unsafe { &*start },
            frame,
            base + s[3] as u64,
            record,
            record.code as u64,
        );
    }
    CONTINUE_SEARCH
}

// ---- hardware exceptions ---------------------------------------------------

#[no_mangle]
unsafe extern "C" fn peekgame_seh_hardware(
    record: *mut ExceptionRecord,
    context: *mut Context,
) -> ! {
    dispatch(unsafe { &mut *record }, unsafe { &mut *context })
}
#[repr(C)]
struct SigInfo {
    signo: i32,
    errno: i32,
    code: i32,
    _pad: i32,
    address: usize,
}
// ucontext_t: gregs at 40 (REG_R8 0 .. REG_CR2 22), fpregs pointer at 224.
const GREGS: usize = 40;
const REG: [usize; 16] = [13, 14, 12, 11, 15, 10, 9, 8, 0, 1, 2, 3, 4, 5, 6, 7]; // rax..r15 -> gregs index
const REG_RIP: usize = 16;
const REG_RSP: usize = 15;
const REG_RDI: usize = 8;
const REG_RSI: usize = 9;
const REG_EFL: usize = 17;
const REG_ERR: usize = 19;
unsafe fn greg(u: *mut u8, i: usize) -> *mut u64 {
    unsafe { u.add(GREGS + i * 8).cast() }
}
/// Is `pc` in host code (PeekGame or a host library) rather than the program (its
/// images, or code it generated)?
fn host_code(pc: usize) -> bool {
    #[repr(C)]
    struct DlInfo {
        fname: *const u8,
        fbase: usize,
        sname: *const u8,
        saddr: usize,
    }
    unsafe extern "C" {
        fn dladdr(address: usize, info: *mut DlInfo) -> i32;
    }
    if images::try_by_address(pc).is_some() {
        return false;
    }
    let mut info: DlInfo = unsafe { std::mem::zeroed() };
    unsafe { dladdr(pc, &mut info) != 0 && !info.fname.is_null() }
}
unsafe extern "C" fn on_fault(signal: i32, info: *mut SigInfo, u: *mut u8) {
    let pc = unsafe { *greg(u, REG_RIP) } as usize;
    let sp = unsafe { *greg(u, REG_RSP) } as usize;
    if signal == 11 && unsafe { repair_foreign_stack_probe(info, u, pc) } {
        return;
    }
    if !host_code(pc) && unsafe { redirect(signal, info, u, pc, sp) } {
        return;
    }
    // A fault in PeekGame or a host library, or no room to dispatch: report and stop.
    let address = unsafe { (*info).address };
    let caller = if pc < 0x10000 {
        unsafe { (sp as *const usize).read() }
    } else {
        0
    };
    eprintln!(
        "STOP: signal {signal} at {} (address {address:#x}, rsp={sp:#x}{})",
        describe(pc),
        if caller != 0 {
            format!(", called from {}", describe(caller))
        } else {
            String::new()
        }
    );
    if images::try_by_address(pc).is_some() {
        let (stack_base, stack_limit, teb_tid): (usize, usize, usize);
        unsafe {
            std::arch::asm!("mov {}, gs:[8]", out(reg) stack_base);
            std::arch::asm!("mov {}, gs:[16]", out(reg) stack_limit);
            std::arch::asm!("mov {}, gs:[72]", out(reg) teb_tid);
        }
        unsafe extern "C" { fn syscall(number: i64, ...) -> i64; }
        let tid = unsafe { syscall(186) };
        eprintln!("STOP: TEB stack base={stack_base:#x} limit={stack_limit:#x} teb_tid={teb_tid} actual_tid={tid}");
    }
    unsafe extern "C" {
        fn _exit(code: i32) -> !;
    }
    unsafe { _exit(128 + signal) }
}
/// A native worker may inherit another thread's GS base and then run a PE
/// callback. MSVC's stack probe reads gs:StackLimit; with the inherited TEB it
/// writes below the *other* stack. Recognize that precise probe and restart it
/// after installing the current worker's TEB. Other faults take the normal path.
unsafe fn repair_foreign_stack_probe(info: *mut SigInfo, u: *mut u8, pc: usize) -> bool {
    if images::try_by_address(pc).is_none() || pc < 27 {
        return false;
    }
    const LOAD_LIMIT: [u8; 9] = [0x65, 0x4c, 0x8b, 0x1c, 0x25, 0x10, 0, 0, 0];
    const PROBE: [u8; 4] = [0x41, 0xc6, 0x03, 0];
    let bytes = unsafe { std::slice::from_raw_parts((pc - 27) as *const u8, 9) };
    let probe = unsafe { std::slice::from_raw_parts(pc as *const u8, 4) };
    if bytes != LOAD_LIMIT || probe != PROBE {
        return false;
    }
    unsafe extern "C" { fn syscall(number: i64, ...) -> i64; }
    let mut gs_base: usize = 0;
    if unsafe { syscall(158, 0x1004i32, &mut gs_base) } != 0 || gs_base == 0 {
        return false;
    }
    let teb_tid = unsafe { *((gs_base + 72) as *const usize) };
    let actual_tid = unsafe { syscall(186) } as usize;
    let old_limit = unsafe { *((gs_base + 16) as *const usize) };
    if teb_tid == actual_tid || (unsafe { (*info).address }) != old_limit.saturating_sub(4096) {
        return false;
    }
    if let Err(error) = crate::host::thread_template::attach_foreign_worker() {
        eprintln!("foreign PE worker TEB attach failed: {error}");
        return false;
    }
    unsafe { *greg(u, REG_RIP) = (pc - 27) as u64 };
    eprintln!("foreign PE worker {actual_tid}: repaired inherited TEB from thread {teb_tid}");
    true
}
/// Builds CONTEXT and EXCEPTION_RECORD below the faulting frame and makes the
/// thread continue in the dispatcher on its own stack, as on Windows.
unsafe fn redirect(signal: i32, info: *mut SigInfo, u: *mut u8, pc: usize, sp: usize) -> bool {
    let frame = (sp - 512 - size_of::<Context>() - size_of::<ExceptionRecord>()) & !63;
    let limit = teb_field(0x10) as usize;
    if limit != 0 && frame < limit + 16384 {
        return false; // no room left: a stack overflow
    }
    let context = frame as *mut Context;
    let record = (frame + size_of::<Context>()) as *mut ExceptionRecord;
    unsafe {
        let c = &mut *context;
        std::ptr::write_bytes(context, 0, 1);
        c.flags = 0x10000b; // CONTEXT_FULL
        for (reg, &index) in REG.iter().enumerate() {
            *c.int_reg(reg) = *greg(u, index);
        }
        c.rip = pc as u64;
        c.eflags = *greg(u, REG_EFL) as u32;
        let fpregs = *(u.add(224) as *const *const u8);
        if !fpregs.is_null() {
            std::ptr::copy_nonoverlapping(fpregs, c.extended.as_mut_ptr(), 512);
            c.mxcsr = (fpregs.add(24) as *const u32).read_unaligned();
        }
        std::ptr::write_bytes(record, 0, 1);
        let r = &mut *record;
        r.code = match signal {
            11 | 7 => {
                // STATUS_ACCESS_VIOLATION: read 0, write 1, execute 8.
                let error = *greg(u, REG_ERR);
                r.count = 2;
                r.information[0] = if error & 16 != 0 {
                    8
                } else if error & 2 != 0 {
                    1
                } else {
                    0
                };
                r.information[1] = (*info).address;
                0xC000_0005
            }
            4 => 0xC000_001D, // STATUS_ILLEGAL_INSTRUCTION
            5 => {
                // STATUS_BREAKPOINT, at the int3.
                if pc > 0 && *((pc - 1) as *const u8) == 0xcc {
                    c.rip = pc as u64 - 1;
                }
                0x8000_0003
            }
            8 => match (*info).code {
                1 => 0xC000_0094, // FPE_INTDIV
                2 => 0xC000_0095, // FPE_INTOVF
                _ => 0xC000_0090,
            },
            _ => return false,
        };
        r.address = c.rip as usize;
        *greg(u, REG_RIP) = peekgame_seh_hardware_dispatch as *const () as u64;
        *greg(u, REG_RSP) = frame as u64;
        *greg(u, REG_RDI) = record as u64;
        *greg(u, REG_RSI) = context as u64;
    }
    true
}
/// Installs the fault handlers once per process: SIGSEGV, SIGBUS, SIGILL, SIGFPE
/// and SIGTRAP become Windows exceptions in program code.
pub fn install_fault_handlers() {
    #[repr(C)]
    struct SigAction {
        handler: usize,
        mask: [u64; 16],
        flags: i32,
        restorer: usize,
    }
    unsafe extern "C" {
        fn sigaction(signal: i32, action: *const SigAction, old: *mut SigAction) -> i32;
    }
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        const SA_SIGINFO: i32 = 4;
        const SA_ONSTACK: i32 = 0x0800_0000;
        const SA_NODEFER: i32 = 0x4000_0000;
        for signal in [11, 7, 4, 8, 5] {
            let action = SigAction {
                handler: on_fault as *const () as usize,
                mask: [0; 16],
                flags: SA_SIGINFO | SA_ONSTACK | SA_NODEFER,
                restorer: 0,
            };
            unsafe { sigaction(signal, &action, std::ptr::null_mut()) };
        }
    });
}
/// A signal's ucontext as a Windows CONTEXT (integer registers, flags, FPU/SSE).
pub(crate) unsafe fn context_from_signal(u: *mut u8, c: &mut Context) {
    unsafe {
        std::ptr::write_bytes(c as *mut Context, 0, 1);
        c.flags = 0x10_000b; // CONTEXT_FULL
        for (reg, &index) in REG.iter().enumerate() {
            *c.int_reg(reg) = *greg(u, index);
        }
        c.rip = *greg(u, REG_RIP);
        c.eflags = *greg(u, REG_EFL) as u32;
        let fpregs = *(u.add(224) as *const *const u8);
        if !fpregs.is_null() {
            std::ptr::copy_nonoverlapping(fpregs, c.extended.as_mut_ptr(), 512);
            c.mxcsr = (fpregs.add(24) as *const u32).read_unaligned();
        }
    }
}
/// Writes a CONTEXT's integer registers and rip back into a signal's ucontext.
pub(crate) unsafe fn context_to_signal(c: &mut Context, u: *mut u8) {
    unsafe {
        for (reg, &index) in REG.iter().enumerate() {
            *greg(u, index) = *c.int_reg(reg);
        }
        *greg(u, REG_RIP) = c.rip;
        *greg(u, REG_EFL) = c.eflags as u64;
    }
}
