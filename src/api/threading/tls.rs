//! TlsAlloc and friends. The values live in the TEB (TlsSlots, TlsExpansionSlots)
//! as on Windows, where programs and JIT compilers also read them through gs.
use crate::{api::system::error, host::teb};
use std::sync::Mutex;
static SLOTS: Mutex<[bool; teb::TLS_INDICES]> = Mutex::new([false; teb::TLS_INDICES]);
pub extern "win64" fn alloc() -> u32 {
    let mut slots = SLOTS.lock().unwrap();
    if let Some(index) = slots.iter().position(|v| !*v) {
        slots[index] = true;
        teb::clear_tls_slot(index);
        index as u32
    } else {
        error::set(8);
        u32::MAX
    }
}
pub extern "win64" fn free(index: u32) -> i32 {
    let mut slots = SLOTS.lock().unwrap();
    match slots.get_mut(index as usize) {
        Some(slot) if *slot => {
            *slot = false;
            teb::clear_tls_slot(index as usize);
            1
        }
        _ => {
            error::set(87);
            0
        }
    }
}
fn cell(index: u32) -> Option<*mut usize> {
    let allocated = SLOTS
        .lock()
        .unwrap()
        .get(index as usize)
        .copied()
        .unwrap_or(false);
    allocated.then(|| teb::tls_slot(index as usize)).flatten()
}
pub extern "win64" fn get(index: u32) -> usize {
    let Some(cell) = cell(index) else {
        error::set(87);
        return 0;
    };
    error::set(0);
    unsafe { cell.read() }
}
pub extern "win64" fn set(index: u32, value: usize) -> i32 {
    let Some(cell) = cell(index) else {
        error::set(87);
        return 0;
    };
    unsafe { cell.write(value) };
    1
}
