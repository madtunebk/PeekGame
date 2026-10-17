//! Global and local atom tables: a process-wide string<->u16 map (ported from the
//! classic Win32 atom API). Both GlobalAddAtom and AddAtom share one table here, as
//! they effectively do within a process. Integer atoms (0..0xC000, passed as
//! MAKEINTATOM) are returned unchanged and never stored. String atoms start at 0xC000.
use crate::api::{system::error, text};
use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    sync::Mutex,
};
struct Atoms {
    by_name: HashMap<String, u16>,
    names: HashMap<u16, (String, u32)>, // atom -> (name, reference count)
    next: u16,
}
static TABLE: Mutex<Option<Atoms>> = Mutex::new(None);
const FIRST: u16 = 0xC000;
fn with<T>(f: impl FnOnce(&mut Atoms) -> T) -> T {
    let mut guard = TABLE.lock().unwrap();
    let atoms = guard.get_or_insert_with(|| Atoms {
        by_name: HashMap::new(),
        names: HashMap::new(),
        next: FIRST,
    });
    f(atoms)
}
/// An integer atom (MAKEINTATOM): the pointer is a value below 0x10000.
fn integer(name: usize) -> Option<u16> {
    (name < 0x1_0000).then_some(name as u16)
}
fn add(name: String) -> u16 {
    with(|atoms| {
        if let Some(&atom) = atoms.by_name.get(&name) {
            atoms.names.get_mut(&atom).unwrap().1 += 1;
            return atom;
        }
        if atoms.next == 0xFFFF {
            error::set(8);
            return 0;
        }
        let atom = atoms.next;
        atoms.next += 1;
        atoms.by_name.insert(name.clone(), atom);
        atoms.names.insert(atom, (name, 1));
        error::set(0);
        atom
    })
}
fn find(name: &str) -> u16 {
    with(|atoms| match atoms.by_name.get(name) {
        Some(&atom) => atom,
        None => {
            error::set(0); // ERROR_SUCCESS; GetLastError stays 0 as on Windows
            0
        }
    })
}
fn delete(atom: u16) -> i32 {
    if atom < FIRST {
        return 1; // integer atoms are a no-op success
    }
    with(|atoms| {
        if let Some(entry) = atoms.names.get_mut(&atom) {
            entry.1 -= 1;
            if entry.1 == 0 {
                let name = entry.0.clone();
                atoms.names.remove(&atom);
                atoms.by_name.remove(&name);
            }
            1
        } else {
            error::set(203); // ERROR_NO_SYSTEM_RESOURCES... use ERROR_INVALID_HANDLE
            0
        }
    })
}
unsafe fn name_out_w(atom: u16, out: *mut u16, size: u32) -> u32 {
    if atom < FIRST {
        // An integer atom formats as "#nnnnn".
        let text = format!("#{atom}");
        return unsafe { write_w(&text, out, size) };
    }
    match with(|a| a.names.get(&atom).map(|e| e.0.clone())) {
        Some(name) => unsafe { write_w(&name, out, size) },
        None => {
            error::set(6);
            0
        }
    }
}
unsafe fn write_w(text: &str, out: *mut u16, size: u32) -> u32 {
    let wide: Vec<u16> = text.encode_utf16().collect();
    if out.is_null() || size == 0 {
        return 0;
    }
    let n = wide.len().min(size as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    n as u32
}
pub unsafe extern "win64" fn global_add_w(name: *const u16) -> u16 {
    match integer(name as usize) {
        Some(atom) => atom,
        None => match unsafe { text::string(name) } {
            Some(s) => add(s),
            None => {
                error::set(87);
                0
            }
        },
    }
}
pub unsafe extern "win64" fn global_add_a(name: *const c_char) -> u16 {
    match integer(name as usize) {
        Some(atom) => atom,
        None => match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => add(s.to_owned()),
            Err(_) => {
                error::set(87);
                0
            }
        },
    }
}
pub unsafe extern "win64" fn global_find_w(name: *const u16) -> u16 {
    match integer(name as usize) {
        Some(atom) => atom,
        None => match unsafe { text::string(name) } {
            Some(s) => find(&s),
            None => fail(87),
        },
    }
}
pub unsafe extern "win64" fn global_find_a(name: *const c_char) -> u16 {
    match integer(name as usize) {
        Some(atom) => atom,
        None => match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => find(s),
            Err(_) => fail(87),
        },
    }
}
pub unsafe extern "win64" fn global_name_w(atom: u16, out: *mut u16, size: u32) -> u32 {
    unsafe { name_out_w(atom, out, size) }
}
pub unsafe extern "win64" fn global_name_a(atom: u16, out: *mut c_char, size: u32) -> u32 {
    let name = if atom < FIRST {
        Some(format!("#{atom}"))
    } else {
        with(|a| a.names.get(&atom).map(|e| e.0.clone()))
    };
    match name {
        Some(name) => unsafe { text::copy_out_narrow(name.as_bytes(), out.cast(), size) },
        None => {
            error::set(6);
            0
        }
    }
}
pub extern "win64" fn global_delete(atom: u16) -> u16 {
    if delete(atom) != 0 {
        0 // success returns 0
    } else {
        atom // failure returns the atom
    }
}
pub extern "win64" fn global_flags(atom: u16) -> u32 {
    if atom < FIRST || with(|a| a.names.contains_key(&atom)) {
        0
    } else {
        error::set(6);
        0
    }
}
fn fail(code: u32) -> u16 {
    error::set(code);
    0
}
