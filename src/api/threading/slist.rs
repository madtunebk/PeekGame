//! Interlocked singly linked lists (SLIST_HEADER, 16 bytes, 16-byte aligned),
//! lock-free as on Windows x64 (ported from peek's slist.inc). Header: qword 0 =
//! Depth (bits 0-15) | Sequence (bits 16-63); qword 1 holds the first entry in bits
//! 4-63. Every change bumps Sequence and swaps both qwords with cmpxchg16b, so a pop
//! never reuses a stale Next (ABA). Entries (SLIST_ENTRY: Next at 0) are 16-byte
//! aligned.
use std::arch::asm;

fn stop(message: &str) -> ! {
    eprintln!("STOP: {message}");
    std::process::exit(78)
}
fn aligned(p: usize, api: &str) {
    if !p.is_multiple_of(16) {
        stop(&format!(
            "{api}: SLIST header and entries must be 16-byte aligned"
        ));
    }
}
unsafe fn read(head: *mut u64) -> (u64, u64) {
    // A torn read only makes the swap fail and retry.
    unsafe { (head.read_volatile(), head.add(1).read_volatile()) }
}
unsafe fn swap(head: *mut u64, old: (u64, u64), new: (u64, u64)) -> bool {
    let ok: u8;
    unsafe {
        asm!(
            "xchg rbx, {new0}",
            "lock cmpxchg16b [{head}]",
            "sete {ok}",
            "mov rbx, {new0}",
            head = in(reg) head,
            new0 = inout(reg) new.0 => _,
            ok = out(reg_byte) ok,
            inout("rax") old.0 => _,
            inout("rdx") old.1 => _,
            in("rcx") new.1,
        );
    }
    ok != 0
}
fn next_header(q0: u64, delta: i64) -> u64 {
    let depth = (q0 as i64 + delta) as u64 & 0xffff;
    let sequence = ((q0 >> 16) + 1) & 0xffff_ffff_ffff;
    depth | sequence << 16
}

pub unsafe extern "win64" fn initialize(head: *mut u64) {
    aligned(head as usize, "InitializeSListHead");
    unsafe { head.write_bytes(0, 2) };
}
pub unsafe extern "win64" fn push(head: *mut u64, entry: *mut u64) -> usize {
    aligned(head as usize, "InterlockedPushEntrySList");
    aligned(entry as usize, "InterlockedPushEntrySList");
    loop {
        let (q0, q1) = unsafe { read(head) };
        unsafe { entry.write_volatile(q1 & !15) };
        if unsafe {
            swap(
                head,
                (q0, q1),
                (next_header(q0, 1), (q1 & 15) | entry as u64),
            )
        } {
            return (q1 & !15) as usize;
        }
    }
}
pub unsafe extern "win64" fn pop(head: *mut u64) -> usize {
    aligned(head as usize, "InterlockedPopEntrySList");
    loop {
        let (q0, q1) = unsafe { read(head) };
        let first = q1 & !15;
        if first == 0 {
            return 0;
        }
        let next = unsafe { (first as *const u64).read_volatile() } & !15;
        if unsafe { swap(head, (q0, q1), (next_header(q0, -1), (q1 & 15) | next)) } {
            return first as usize;
        }
    }
}
pub unsafe extern "win64" fn flush(head: *mut u64) -> usize {
    aligned(head as usize, "InterlockedFlushSList");
    loop {
        let (q0, q1) = unsafe { read(head) };
        if q1 & !15 == 0 {
            return 0;
        }
        let emptied = next_header(q0, -((q0 & 0xffff) as i64));
        if unsafe { swap(head, (q0, q1), (emptied, q1 & 15)) } {
            return (q1 & !15) as usize;
        }
    }
}
/// InterlockedPushListSListEx: a chain already linked from `list` to `end`.
pub unsafe extern "win64" fn push_list(
    head: *mut u64,
    list: *mut u64,
    end: *mut u64,
    count: u32,
) -> usize {
    for p in [head, list, end] {
        aligned(p as usize, "InterlockedPushListSListEx");
    }
    loop {
        let (q0, q1) = unsafe { read(head) };
        unsafe { end.write_volatile(q1 & !15) };
        if unsafe {
            swap(
                head,
                (q0, q1),
                (next_header(q0, count as i64), (q1 & 15) | list as u64),
            )
        } {
            return (q1 & !15) as usize;
        }
    }
}
pub unsafe extern "win64" fn depth(head: *mut u64) -> u16 {
    unsafe { read(head) }.0 as u16
}
