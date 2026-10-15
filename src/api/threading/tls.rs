use crate::api::system::error;
use std::{cell::RefCell, sync::Mutex};
static SLOTS: Mutex<[bool; 1088]> = Mutex::new([false; 1088]);
thread_local! {static VALUES:RefCell<[usize;1088]>=const{RefCell::new([0;1088])};}
pub extern "win64" fn alloc() -> u32 {
    let mut slots = SLOTS.lock().unwrap();
    if let Some(index) = slots.iter().position(|v| !*v) {
        slots[index] = true;
        VALUES.with(|values| values.borrow_mut()[index] = 0);
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
            1
        }
        _ => {
            error::set(87);
            0
        }
    }
}
fn valid(index: u32) -> bool {
    SLOTS
        .lock()
        .unwrap()
        .get(index as usize)
        .copied()
        .unwrap_or(false)
}
pub extern "win64" fn get(index: u32) -> usize {
    if !valid(index) {
        error::set(87);
        return 0;
    }
    error::set(0);
    VALUES.with(|values| values.borrow()[index as usize])
}
pub extern "win64" fn set(index: u32, value: usize) -> i32 {
    if !valid(index) {
        error::set(87);
        return 0;
    }
    VALUES.with(|values| values.borrow_mut()[index as usize] = value);
    1
}
