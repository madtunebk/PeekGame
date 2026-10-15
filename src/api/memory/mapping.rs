//! File mappings (ported from peek's file_mapping.inc): CreateFileMapping,
//! OpenFileMapping, MapViewOfFile(Ex), UnmapViewOfFile, FlushViewOfFile.
//! A mapping keeps its own descriptor: the file's (duplicated) or, for
//! INVALID_HANDLE_VALUE, a memfd of the requested size (pagefile-backed memory).
//! Views are mmap()s: FILE_MAP_COPY is MAP_PRIVATE, other access MAP_SHARED, so
//! writes reach the file as on Windows. Named mappings live in this process's
//! namespace (there is no other process). Views outlive the mapping handle.
use crate::api::{filesystem::files, system::error, text};
use std::{
    collections::HashMap,
    fs::File,
    os::fd::AsRawFd,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
struct Mapping {
    file: File,
    size: u64,
    protection: u32,
    name: Option<String>,
}
struct State {
    handles: HashMap<usize, Arc<Mapping>>,
    views: HashMap<usize, usize>,
}
static STATE: Mutex<Option<State>> = Mutex::new(None);
static NEXT: AtomicUsize = AtomicUsize::new(0x4c00_0000);
fn with<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut state = STATE.lock().unwrap();
    f(state.get_or_insert_with(|| State {
        handles: HashMap::new(),
        views: HashMap::new(),
    }))
}
fn insert(mapping: Arc<Mapping>) -> usize {
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    with(|s| s.handles.insert(handle, mapping));
    handle
}
unsafe extern "C" {
    fn memfd_create(name: *const u8, flags: u32) -> i32;
    fn mmap(
        address: *mut std::ffi::c_void,
        length: usize,
        protection: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut std::ffi::c_void;
    fn munmap(address: *mut std::ffi::c_void, length: usize) -> i32;
    fn msync(address: usize, length: usize, flags: i32) -> i32;
}
const ERROR_ALREADY_EXISTS: u32 = 183;
fn create(file: usize, protection: u32, high: u32, low: u32, name: Option<String>) -> usize {
    let page = protection & 0xff;
    if ![2, 4, 8, 0x20, 0x40, 0x80].contains(&page) || protection & !0xff & !0x0800_0000 != 0 {
        eprintln!("CreateFileMapping: protection {protection:#x} is not supported");
        error::set(87);
        return 0;
    }
    if let Some(name) = &name {
        let existing = with(|s| {
            s.handles
                .values()
                .find(|m| m.name.as_deref() == Some(name.as_str()))
                .cloned()
        });
        if let Some(existing) = existing {
            let handle = insert(existing);
            error::set(ERROR_ALREADY_EXISTS);
            return handle;
        }
    }
    let writable = page == 4 || page == 0x40;
    let mut size = (high as u64) << 32 | low as u64;
    let file = if file == usize::MAX {
        if size == 0 {
            error::set(87);
            return 0;
        }
        let fd = unsafe { memfd_create(c"peekgame-mapping".as_ptr().cast(), 1) };
        if fd < 0 {
            error::set(8);
            return 0;
        }
        let file = unsafe { <File as std::os::fd::FromRawFd>::from_raw_fd(fd) };
        if file.set_len(size).is_err() {
            error::set(8);
            return 0;
        }
        file
    } else {
        let Some(file) = files::clone_file(file) else {
            error::set(6);
            return 0;
        };
        let length = file.metadata().map(|m| m.len()).unwrap_or(0);
        if size == 0 {
            size = length;
        }
        if size == 0 {
            error::set(1006); // ERROR_FILE_INVALID: an empty file
            return 0;
        }
        if size > length && (!writable || file.set_len(size).is_err()) {
            error::set(8);
            return 0;
        }
        file
    };
    error::set(0);
    insert(Arc::new(Mapping {
        file,
        size,
        protection: page,
        name,
    }))
}
pub unsafe extern "win64" fn create_w(
    file: usize,
    _security: usize,
    protection: u32,
    high: u32,
    low: u32,
    name: *const u16,
) -> usize {
    let name = unsafe { text::string(name) }.filter(|n| !n.is_empty());
    create(file, protection, high, low, name)
}
pub unsafe extern "win64" fn create_a(
    file: usize,
    _security: usize,
    protection: u32,
    high: u32,
    low: u32,
    name: *const i8,
) -> usize {
    let name = (!name.is_null())
        .then(|| {
            unsafe { std::ffi::CStr::from_ptr(name) }
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| !n.is_empty());
    create(file, protection, high, low, name)
}
pub unsafe extern "win64" fn open_w(_access: u32, _inherit: i32, name: *const u16) -> usize {
    let Some(name) = (unsafe { text::string(name) }) else {
        error::set(87);
        return 0;
    };
    match with(|s| {
        s.handles
            .values()
            .find(|m| m.name.as_deref() == Some(name.as_str()))
            .cloned()
    }) {
        Some(mapping) => insert(mapping),
        None => {
            error::set(2);
            0
        }
    }
}
/// FILE_MAP_COPY 1, FILE_MAP_WRITE 2, FILE_MAP_READ 4, FILE_MAP_EXECUTE 0x20.
pub extern "win64" fn map_view_ex(
    handle: usize,
    access: u32,
    high: u32,
    low: u32,
    length: usize,
    address: usize,
) -> usize {
    let Some(mapping) = with(|s| s.handles.get(&handle).cloned()) else {
        error::set(6);
        return 0;
    };
    let offset = (high as u64) << 32 | low as u64;
    if !offset.is_multiple_of(65536) || !address.is_multiple_of(65536) {
        error::set(1132); // ERROR_MAPPED_ALIGNMENT
        return 0;
    }
    if offset >= mapping.size || (length != 0 && length as u64 > mapping.size - offset) {
        error::set(8);
        return 0;
    }
    let length = if length == 0 {
        (mapping.size - offset) as usize
    } else {
        length
    };
    let copy = access & 1 != 0 && access & 2 == 0;
    let write = access & 2 != 0 || copy;
    if access & 2 != 0 && mapping.protection != 4 && mapping.protection != 0x40 {
        error::set(5);
        return 0;
    }
    let protection = 1 | if write { 2 } else { 0 } | if access & 0x20 != 0 { 4 } else { 0 };
    const MAP_SHARED: i32 = 1;
    const MAP_PRIVATE: i32 = 2;
    const MAP_FIXED_NOREPLACE: i32 = 0x10_0000;
    let flags = if copy { MAP_PRIVATE } else { MAP_SHARED }
        | if address != 0 { MAP_FIXED_NOREPLACE } else { 0 };
    let base = unsafe {
        mmap(
            address as *mut _,
            length,
            protection,
            flags,
            mapping.file.as_raw_fd(),
            offset as i64,
        )
    } as isize;
    if base < 0 || (address != 0 && base as usize != address) {
        error::set(if address != 0 { 487 } else { 8 });
        return 0;
    }
    with(|s| s.views.insert(base as usize, length));
    base as usize
}
pub extern "win64" fn map_view(
    handle: usize,
    access: u32,
    high: u32,
    low: u32,
    length: usize,
) -> usize {
    map_view_ex(handle, access, high, low, length, 0)
}
pub extern "win64" fn unmap_view(base: usize) -> i32 {
    match with(|s| s.views.remove(&base)) {
        Some(length) => {
            unsafe { munmap(base as *mut _, length) };
            1
        }
        None => {
            error::set(487); // ERROR_INVALID_ADDRESS
            0
        }
    }
}
/// FlushViewOfFile: writes the pages of the view containing `address` (length 0:
/// to the end of the view).
pub extern "win64" fn flush_view(address: usize, length: usize) -> i32 {
    let view = with(|s| {
        s.views
            .iter()
            .find(|(&b, &l)| address >= b && address < b + l)
            .map(|(&b, &l)| (b, l))
    });
    let Some((base, size)) = view else {
        error::set(487);
        return 0;
    };
    let start = address & !4095;
    let end = if length == 0 {
        base + size
    } else {
        (address + length).min(base + size)
    };
    unsafe { msync(start, end - start, 4) }; // MS_SYNC
    1
}
/// CloseHandle for mappings. False if not one.
pub fn close(handle: usize) -> bool {
    with(|s| s.handles.remove(&handle)).is_some()
}
