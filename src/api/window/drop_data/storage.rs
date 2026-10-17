use super::Medium;
use crate::api::{memory::global, system::error};
pub(crate) unsafe extern "win64" fn release_medium(medium: *mut Medium) {
    if medium.is_null() {
        return;
    }
    let m = unsafe { &mut *medium };
    if m.owner != 0 {
        unsafe {
            let vt = *(m.owner as *const *const usize);
            let release: extern "win64" fn(usize) -> u32 = std::mem::transmute(*vt.add(2));
            release(m.owner);
        }
    } else if m.kind == 1 {
        global::free(m.handle);
    } else if m.kind != 0 {
        eprintln!(
            "STOP: ReleaseStgMedium: storage type {} is not supported",
            m.kind
        );
        std::process::exit(78);
    }
    unsafe {
        medium.write_bytes(0, 1);
    }
}
pub(crate) unsafe extern "win64" fn query_file(
    drop: usize,
    index: u32,
    out: *mut u16,
    capacity: u32,
) -> u32 {
    let Some(bytes) = global::snapshot(drop) else {
        error::set(6);
        return 0;
    };
    if bytes.len() < 24 {
        error::set(87);
        return 0;
    }
    let offset = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
    if offset < 20
        || offset % 2 != 0
        || offset > bytes.len() - 4
        || u32::from_le_bytes(bytes[16..20].try_into().unwrap()) != 1
    {
        error::set(87);
        return 0;
    }
    let units: Vec<u16> = bytes[offset..]
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let mut names = Vec::new();
    let mut pos = 0;
    while pos < units.len() && units[pos] != 0 {
        let Some(length) = units[pos..].iter().position(|&v| v == 0) else {
            error::set(87);
            return 0;
        };
        names.push(&units[pos..pos + length]);
        pos += length + 1;
    }
    if pos >= units.len() {
        error::set(87);
        return 0;
    }
    if index == u32::MAX {
        return names.len() as u32;
    }
    let Some(name) = names.get(index as usize) else {
        return 0;
    };
    if out.is_null() {
        return name.len() as u32;
    }
    if capacity == 0 {
        return 0;
    }
    let n = name.len().min(capacity as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(name.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    n as u32
}
pub(crate) extern "win64" fn finish(drop: usize) {
    global::free(drop);
}
const _: () = assert!(std::mem::size_of::<Medium>() == 24);
