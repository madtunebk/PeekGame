//! RtlVirtualUnwind over the documented x64 unwind format (UNWIND_INFO), ported from
//! peek's seh.inc as Wine and Windows implement it, epilog detection included.
use super::{tables::RuntimeFunction, Context};
pub const UNW_FLAG_EHANDLER: u32 = 1;
pub const UNW_FLAG_UHANDLER: u32 = 2;
const UNW_FLAG_CHAININFO: u8 = 4;

unsafe fn read<T: Copy>(address: u64) -> T {
    unsafe { (address as *const T).read_unaligned() }
}
fn opcode_slots(code: &[u8]) -> usize {
    match code[1] & 15 {
        1 => {
            if code[1] >> 4 != 0 {
                3
            } else {
                2
            }
        }
        4 | 6 | 8 => 2,
        5 | 9 => 3,
        _ => 1,
    }
}
fn in_function(pc: u64, base: u64, f: &RuntimeFunction) -> bool {
    let rva = pc.wrapping_sub(base);
    rva >= f.begin as u64 && rva < f.end as u64
}
/// Is `pc` inside an epilog (only stack adjustment, pops and a return follow)?
unsafe fn inside_epilog(mut pc: u64, base: u64, f: &RuntimeFunction) -> bool {
    let b = |p: u64| unsafe { read::<u8>(p) };
    if b(pc) & 0xf8 == 0x48 {
        match b(pc + 1) {
            0x81 if b(pc) == 0x48 && b(pc + 2) == 0xc4 => pc += 7, // add $nnnn,%rsp
            0x83 if b(pc) == 0x48 && b(pc + 2) == 0xc4 => pc += 4, // add $n,%rsp
            0x8d => {
                // lea n(reg),%rsp
                let modrm = b(pc + 2);
                if b(pc) & 0x06 != 0 || (modrm >> 3) & 7 != 4 || modrm & 7 == 4 {
                    return false;
                }
                match modrm >> 6 {
                    1 => pc += 4,
                    2 => pc += 7,
                    _ => return false,
                }
            }
            0x81 | 0x83 => return false,
            _ => {}
        }
    }
    loop {
        if b(pc) & 0xf0 == 0x40 {
            pc += 1;
        }
        match b(pc) {
            0x58..=0x5f => pc += 1,
            0xc2 | 0xc3 => return true,
            0xe9 => {
                pc = pc
                    .wrapping_add(5)
                    .wrapping_add(unsafe { read::<i32>(pc + 1) } as i64 as u64);
                if !in_function(pc, base, f) {
                    return false;
                }
            }
            0xeb => {
                pc = pc
                    .wrapping_add(2)
                    .wrapping_add(b(pc + 1) as i8 as i64 as u64);
                if !in_function(pc, base, f) {
                    return false;
                }
            }
            0xf3 => return b(pc + 1) == 0xc3,
            _ => return false,
        }
    }
}
unsafe fn interpret_epilog(mut pc: u64, c: &mut Context) {
    let b = |p: u64| unsafe { read::<u8>(p) };
    loop {
        let mut rex = 0u8;
        if b(pc) & 0xf0 == 0x40 {
            rex = b(pc) & 0x0f;
            pc += 1;
        }
        match b(pc) {
            op @ 0x58..=0x5f => {
                let reg = (op - 0x58) as usize + (rex & 1) as usize * 8;
                *c.int_reg(reg) = unsafe { read(c.rsp) };
                c.rsp += 8;
                pc += 1;
            }
            0x81 => {
                c.rsp = c
                    .rsp
                    .wrapping_add(unsafe { read::<i32>(pc + 2) } as i64 as u64);
                pc += 6;
            }
            0x83 => {
                c.rsp = c.rsp.wrapping_add(b(pc + 2) as i8 as i64 as u64);
                pc += 3;
            }
            0x8d => {
                let base = *c.int_reg((b(pc + 1) & 7) as usize + (rex & 1) as usize * 8);
                if b(pc + 1) >> 6 == 1 {
                    c.rsp = base.wrapping_add(b(pc + 2) as i8 as i64 as u64);
                    pc += 3;
                } else {
                    c.rsp = base.wrapping_add(unsafe { read::<i32>(pc + 2) } as i64 as u64);
                    pc += 6;
                }
            }
            0xc2 => {
                let n = unsafe { read::<u16>(pc + 1) } as u64;
                c.rip = unsafe { read(c.rsp) };
                c.rsp += 8 + n;
                return;
            }
            0xc3 | 0xf3 => {
                c.rip = unsafe { read(c.rsp) };
                c.rsp += 8;
                return;
            }
            0xe9 => {
                pc = pc
                    .wrapping_add(5)
                    .wrapping_add(unsafe { read::<i32>(pc + 1) } as i64 as u64)
            }
            0xeb => {
                pc = pc
                    .wrapping_add(2)
                    .wrapping_add(b(pc + 1) as i8 as i64 as u64)
            }
            _ => return,
        }
    }
}
/// RtlVirtualUnwind: undoes the prolog of the function containing `pc` in `c` (one
/// frame up) and returns its handler of kind `kind` with `*data`, or NULL.
pub unsafe extern "win64" fn virtual_unwind(
    kind: u32,
    base: u64,
    pc: u64,
    mut f: *const RuntimeFunction,
    c: &mut Context,
    data: *mut usize,
    frame_out: *mut u64,
    _pointers: usize,
) -> usize {
    let mut frame = c.rsp;
    let mut machine_frame = false;
    let mut prolog_offset;
    let mut info;
    loop {
        let function = unsafe { &*f };
        info = base + function.unwind as u64;
        let header: [u8; 4] = unsafe { read(info) };
        let (version, count, frame_reg, frame_offset) = (
            header[0] & 7,
            header[2] as usize,
            header[3] & 15,
            (header[3] >> 4) as u64,
        );
        let codes = info + 4;
        if version != 1 && version != 2 {
            eprintln!("RtlVirtualUnwind: unwind info version {version} at {pc:#x}");
            return 0;
        }
        if frame_reg != 0 {
            frame = c
                .int_reg(frame_reg as usize)
                .wrapping_sub(frame_offset * 16);
        }
        let start = base + function.begin as u64;
        if pc >= start && pc < start + header[1] as u64 {
            prolog_offset = (pc - start) as u32;
        } else {
            prolog_offset = u32::MAX;
            if count != 0 && unsafe { inside_epilog(pc, base, function) } {
                unsafe { interpret_epilog(pc, c) };
                if !frame_out.is_null() {
                    unsafe { *frame_out = frame };
                }
                return 0;
            }
        }
        let ops = unsafe { std::slice::from_raw_parts(codes as *const u8, count * 2 + 4) };
        let mut i = 0;
        while i < count {
            let op = &ops[2 * i..];
            let (code, reg) = (op[1] & 15, (op[1] >> 4) as usize);
            let slots = opcode_slots(op);
            if prolog_offset >= op[0] as u32 {
                let u16v = || u16::from_le_bytes([op[2], op[3]]) as u64;
                let u32v = || u32::from_le_bytes([op[2], op[3], op[4], op[5]]) as u64;
                match code {
                    0 => {
                        *c.int_reg(reg) = unsafe { read(c.rsp) };
                        c.rsp += 8;
                    }
                    1 => c.rsp += if reg != 0 { u32v() } else { u16v() * 8 },
                    2 => c.rsp += (reg as u64 + 1) * 8,
                    3 => c.rsp = frame,
                    4 => *c.int_reg(reg) = unsafe { read(frame + u16v() * 8) },
                    5 => *c.int_reg(reg) = unsafe { read(frame + u32v()) },
                    6 => {}
                    8 => *c.xmm_reg(reg) = unsafe { read(frame + u16v() * 16) },
                    9 => *c.xmm_reg(reg) = unsafe { read(frame + u32v()) },
                    10 => {
                        if reg != 0 {
                            c.rsp += 8;
                        }
                        c.rip = unsafe { read(c.rsp) };
                        c.rsp = unsafe { read(c.rsp + 24) };
                        machine_frame = true;
                    }
                    _ => eprintln!("RtlVirtualUnwind: unknown unwind code {code}"),
                }
            }
            i += slots;
        }
        if (header[0] >> 3) & UNW_FLAG_CHAININFO == 0 {
            break;
        }
        // Chained: continue with the parent's info.
        f = (codes + 2 * ((count as u64 + 1) & !1)) as *const RuntimeFunction;
    }
    if !machine_frame {
        c.rip = unsafe { read(c.rsp) };
        c.rsp += 8;
    }
    if !frame_out.is_null() {
        unsafe { *frame_out = frame };
    }
    let header: [u8; 4] = unsafe { read(info) };
    if ((header[0] >> 3) as u32 & kind) == 0 || prolog_offset != u32::MAX {
        return 0;
    }
    let handler = info + 4 + 2 * ((header[2] as u64 + 1) & !1);
    if !data.is_null() {
        unsafe { *data = (handler + 4) as usize };
    }
    (base + unsafe { read::<u32>(handler) } as u64) as usize
}
