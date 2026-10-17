//! Owned Windows IDataObject / CF_HDROP. All vtable entries use Win64 ABI.
mod uri;
use crate::api::memory::global;
use std::{
    ffi::c_void,
    sync::atomic::{AtomicU32, Ordering},
};
const POINTER: i32 = 0x80004003u32 as i32;
const FORMAT: i32 = 0x80040064u32 as i32;
const NOT_IMPL: i32 = 0x80004001u32 as i32;
#[repr(C)]
pub(crate) struct Medium {
    pub kind: u32,
    pub handle: usize,
    pub owner: usize,
}
#[repr(C)]
struct DataObject {
    table: *const usize,
    refs: AtomicU32,
    files: Vec<String>,
}
// Function addresses are stored as pointers in the actual table (not const ints).
struct VTable([*const c_void; 12]);
unsafe impl Sync for VTable {}
static TABLE: VTable = VTable([
    qi as _,
    add_ref as _,
    release as _,
    get as _,
    get_here as _,
    query as _,
    canonical as _,
    set as _,
    enumerate as _,
    advise as _,
    unadvise as _,
    enum_advise as _,
]);
const UNKNOWN: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];
const DATA: [u8; 16] = [0x0e, 1, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46];
pub(crate) struct Owned(usize);
impl Owned {
    pub(crate) fn from_uri(bytes: &[u8]) -> Option<Self> {
        let files = uri::parse(bytes)?;
        Some(Self(Box::into_raw(Box::new(DataObject {
            table: TABLE.0.as_ptr().cast(),
            refs: AtomicU32::new(1),
            files,
        })) as usize))
    }
    pub(crate) fn pointer(&self) -> usize {
        self.0
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe {
            release(self.0 as *mut DataObject);
        }
    }
}
unsafe extern "win64" fn qi(this: *mut DataObject, iid: *const u8, out: *mut usize) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    unsafe {
        *out = 0;
    }
    if iid.is_null() {
        return 0x80070057u32 as i32;
    }
    let id = unsafe { std::slice::from_raw_parts(iid, 16) };
    if id != UNKNOWN && id != DATA {
        return 0x80004002u32 as i32;
    }
    unsafe {
        *out = this as usize;
        add_ref(this);
    }
    0
}
unsafe extern "win64" fn add_ref(this: *mut DataObject) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}
unsafe extern "win64" fn release(this: *mut DataObject) -> u32 {
    let left = unsafe { (*this).refs.fetch_sub(1, Ordering::AcqRel) - 1 };
    if left == 0 {
        unsafe {
            drop(Box::from_raw(this));
        }
    }
    left
}
fn supported(format: *const u8) -> bool {
    if format.is_null() {
        return false;
    }
    unsafe {
        format.cast::<u16>().read_unaligned() == 15
            && format.add(16).cast::<u32>().read_unaligned() == 1
            && format.add(20).cast::<i32>().read_unaligned() == -1
            && format.add(24).cast::<u32>().read_unaligned() & 1 != 0
    }
}
unsafe extern "win64" fn query(_this: *mut DataObject, format: *const u8) -> i32 {
    if supported(format) {
        0
    } else {
        FORMAT
    }
}
unsafe extern "win64" fn get(this: *mut DataObject, format: *const u8, out: *mut Medium) -> i32 {
    if out.is_null() {
        return POINTER;
    }
    unsafe {
        out.write_bytes(0, 1);
    }
    if !supported(format) {
        return FORMAT;
    }
    let files = unsafe { &(*this).files };
    let mut units = Vec::<u16>::new();
    for file in files {
        units.extend(file.encode_utf16());
        units.push(0);
    }
    units.push(0);
    let handle = global::alloc(0x42, 20 + units.len() * 2);
    if handle == 0 {
        return 0x8007000eu32 as i32;
    }
    let data = global::lock(handle) as *mut u8;
    if data.is_null() {
        global::free(handle);
        return 0x8007000eu32 as i32;
    }
    unsafe {
        data.cast::<u32>().write_unaligned(20);
        data.add(16).cast::<u32>().write_unaligned(1);
        std::ptr::copy_nonoverlapping(units.as_ptr().cast::<u8>(), data.add(20), units.len() * 2);
    }
    global::unlock(handle);
    unsafe {
        out.write(Medium {
            kind: 1,
            handle,
            owner: 0,
        });
    }
    0
}
unsafe extern "win64" fn get_here(_t: usize, _f: usize, _m: usize) -> i32 {
    NOT_IMPL
}
unsafe extern "win64" fn canonical(_t: usize, _f: usize, out: *mut u8) -> i32 {
    if !out.is_null() {
        unsafe {
            out.add(8).cast::<usize>().write_unaligned(0);
        }
    }
    NOT_IMPL
}
unsafe extern "win64" fn set(_t: usize, _f: usize, _m: usize, _release: i32) -> i32 {
    NOT_IMPL
}
unsafe extern "win64" fn enumerate(_t: usize, _direction: u32, out: *mut usize) -> i32 {
    if !out.is_null() {
        unsafe {
            *out = 0;
        }
    }
    NOT_IMPL
}
unsafe extern "win64" fn advise(
    _t: usize,
    _f: usize,
    _flags: u32,
    _sink: usize,
    out: *mut u32,
) -> i32 {
    if !out.is_null() {
        unsafe {
            *out = 0;
        }
    }
    0x80040003u32 as i32
}
unsafe extern "win64" fn unadvise(_t: usize, _connection: u32) -> i32 {
    0x80040003u32 as i32
}
unsafe extern "win64" fn enum_advise(_t: usize, out: *mut usize) -> i32 {
    if !out.is_null() {
        unsafe {
            *out = 0;
        }
    }
    0x80040003u32 as i32
}

mod storage;
pub(crate) use storage::{finish, query_file, release_medium};
