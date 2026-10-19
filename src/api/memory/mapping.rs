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
static STEAM_SHM_PTR: AtomicUsize = AtomicUsize::new(0);

pub fn is_steam_response_event(handle: usize) -> bool {
    let ptr = STEAM_SHM_PTR.load(Ordering::Acquire);
    if ptr != 0 {
        unsafe {
            let p = ptr as *const u8;
            let target_handle = *(p.add(0xa0) as *const usize);
            let state = *(p.add(0x94) as *const u32);
            if target_handle == handle && state == 2 {
                return true;
            }
        }
    }
    false
}

fn with<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut state = STATE.lock().unwrap();
    f(state.get_or_insert_with(|| {
        unsafe { prctl(0x59616d61, usize::MAX, 0, 0, 0) };
        let base_h = 0x4c00_0000usize + ((std::process::id() as usize & 0xfff) << 12);
        NEXT.fetch_max(base_h, Ordering::Relaxed);
        let mut handles = HashMap::new();
        if let Ok(inherited) = std::env::var("PEEKGAME_INHERITED_MAPPINGS") {
            let mut max_h = base_h;
            for item in inherited.split(',') {
                let mut parts = item.split(':');
                if let (Some(h_s), Some(fd_s), Some(sz_s), Some(pr_s)) =
                    (parts.next(), parts.next(), parts.next(), parts.next())
                {
                    if let (Ok(h), Ok(fd), Ok(size), Ok(protection)) = (
                        usize::from_str_radix(h_s, 16),
                        fd_s.parse::<i32>(),
                        sz_s.parse::<u64>(),
                        pr_s.parse::<u32>(),
                    ) {
                        let duped = unsafe { dup(fd) };
                        if duped >= 0 {
                            use std::os::fd::FromRawFd;
                            let file = unsafe { File::from_raw_fd(duped) };
                            handles.insert(
                                h,
                                Arc::new(Mapping {
                                    file,
                                    size,
                                    protection,
                                    name: None,
                                }),
                            );
                            if h >= max_h {
                                max_h = h + 16;
                            }
                        }
                    }
                }
            }
            NEXT.fetch_max(max_h, Ordering::Relaxed);
        }
        State {
            handles,
            views: HashMap::new(),
        }
    }))
}
fn get(handle: usize) -> Option<Arc<Mapping>> {
    if let Some(m) = with(|s| s.handles.get(&handle).cloned()) {
        return Some(m);
    }
    if !(0x4c00_0000..0x4e00_0000).contains(&handle) {
        return None;
    }
    let text = std::fs::read_to_string("/tmp/peekgame_handles.tsv").ok()?;
    let prefix = format!("M\t{handle:x}\t");
    for line in text.lines().rev() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            let mut parts = rest.split('\t');
            let pid = parts.next()?.parse::<i32>().ok()?;
            let remote_fd = parts.next()?.parse::<i32>().ok()?;
            let size = parts.next()?.parse::<u64>().ok()?;
            let protection = parts.next()?.parse::<u32>().ok()?;
            let pidfd = unsafe { syscall(434, pid, 0usize) } as i32;
            if pidfd < 0 {
                continue;
            }
            let local_fd = unsafe { syscall(438, pidfd, remote_fd, 0usize) } as i32;
            unsafe { close_fd(pidfd) };
            if local_fd >= 0 {
                use std::os::fd::FromRawFd;
                let file = unsafe { File::from_raw_fd(local_fd) };
                let mapping = Arc::new(Mapping {
                    file,
                    size,
                    protection,
                    name: None,
                });
                with(|s| s.handles.insert(handle, mapping.clone()));
                return Some(mapping);
            }
        }
    }
    None
}
pub fn export_inherited() -> String {
    with(|s| {
        let mut out = Vec::new();
        for (&h, m) in &s.handles {
            let fd = m.file.as_raw_fd();
            unsafe { fcntl(fd, 2, 0) }; // F_SETFD = 2, clear FD_CLOEXEC
            out.push(format!("{h:x}:{fd}:{}:{}", m.size, m.protection));
        }
        out.join(",")
    })
}
fn insert(mapping: Arc<Mapping>) -> usize {
    with(|_| {});
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    with(|s| s.handles.insert(handle, mapping));
    handle
}
unsafe extern "C" {
    fn dup(fd: i32) -> i32;
    #[link_name = "close"]
    fn close_fd(fd: i32) -> i32;
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    fn memfd_create(name: *const u8, flags: u32) -> i32;
    fn prctl(option: i32, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32;
    fn syscall(num: i64, ...) -> i64;
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
    if files::tracing() {
        eprintln!("[files] CreateFileMapping fd={} protection={protection:#x} size={size:#x}", file.as_raw_fd());
    }
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
    let clean = name.strip_prefix("Local\\").unwrap_or(&name);
    if clean.eq_ignore_ascii_case("SteamStart_SharedMemFile") {
        let existing = with(|s| {
            s.handles
                .values()
                .find(|m| m.name.as_deref().map(|n| n.strip_prefix("Local\\").unwrap_or(n).eq_ignore_ascii_case("SteamStart_SharedMemFile")).unwrap_or(false))
                .cloned()
        });
        if let Some(m) = existing {
            return insert(m);
        }
        let h = create(usize::MAX, 4, 0, 0x1000, Some("Local\\SteamStart_SharedMemFile".into()));
        let view = map_view_ex(h, 2, 0, 0, 0x1000, 0);
        if view != 0 {
            unsafe {
                let ptr = view as *mut u8;
                *(ptr.add(0x90) as *mut u32) = 2;
                *(ptr.add(0x94) as *mut u32) = 0;
            }
            STEAM_SHM_PTR.store(view, Ordering::Release);
        }
        return h;
    }
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
pub unsafe extern "win64" fn open_a(_access: u32, _inherit: i32, name: *const i8) -> usize {
    let Some(name) = (!name.is_null()).then(|| {
        unsafe { std::ffi::CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
    }) else {
        error::set(87);
        return 0;
    };
    let clean = name.strip_prefix("Local\\").unwrap_or(&name);
    if clean.eq_ignore_ascii_case("SteamStart_SharedMemFile") {
        let existing = with(|s| {
            s.handles
                .values()
                .find(|m| m.name.as_deref().map(|n| n.strip_prefix("Local\\").unwrap_or(n).eq_ignore_ascii_case("SteamStart_SharedMemFile")).unwrap_or(false))
                .cloned()
        });
        if let Some(m) = existing {
            return insert(m);
        }
        let h = create(usize::MAX, 4, 0, 0x1000, Some("Local\\SteamStart_SharedMemFile".into()));
        let view = map_view_ex(h, 2, 0, 0, 0x1000, 0);
        if view != 0 {
            unsafe {
                let ptr = view as *mut u8;
                *(ptr.add(0x90) as *mut u32) = 2;
                *(ptr.add(0x94) as *mut u32) = 0;
            }
            STEAM_SHM_PTR.store(view, Ordering::Release);
        }
        return h;
    }
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
    let Some(mapping) = get(handle) else {
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
    if files::tracing() {
        eprintln!("[files] MapViewOfFile fd={} access={access:#x} offset={offset:#x} length={length:#x} -> {base:#x}", mapping.file.as_raw_fd());
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

pub fn is_mapping(handle: usize) -> bool {
    with(|s| s.handles.contains_key(&handle))
}
/// CloseHandle for mappings. False if not one.
pub fn close(handle: usize) -> bool {
    with(|s| s.handles.remove(&handle)).is_some()
}
/// DuplicateHandle for mappings.
pub fn duplicate(handle: usize, access: u32, options: u32) -> Option<Result<usize, u32>> {
    let mapping = get(handle)?;
    let page = mapping.protection & 0xff;
    let writable = page == 4 || page == 0x40;
    let new_protection = if options & 2 != 0 {
        mapping.protection
    } else {
        let wants_write = access & (0x2 | 0x4000_0000) != 0;
        if wants_write && !writable {
            return Some(Err(5)); // ERROR_ACCESS_DENIED
        }
        if wants_write {
            mapping.protection
        } else {
            2 // PAGE_READONLY
        }
    };
    let Ok(file) = mapping.file.try_clone() else {
        return Some(Err(8));
    };
    let export_fd = unsafe { dup(file.as_raw_fd()) };
    let target = insert(Arc::new(Mapping {
        file,
        size: mapping.size,
        protection: new_protection,
        name: mapping.name.clone(),
    }));
    if export_fd >= 0 {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/peekgame_handles.tsv")
        {
            let _ = writeln!(
                f,
                "M\t{target:x}\t{}\t{export_fd}\t{}\t{new_protection}",
                std::process::id(),
                mapping.size
            );
        }
    }
    Some(Ok(target))
}

/// NtQuerySection(SectionHandle, SectionInformationClass, SectionInformation, SectionInformationLength, ResultLength)
pub extern "win64" fn query_section(
    handle: usize,
    class: u32,
    buffer: *mut u8,
    length: usize,
    return_length: *mut usize,
) -> i32 {
    let Some(mapping) = get(handle) else {
        return -0x3fff_fff8; // STATUS_INVALID_HANDLE (0xC0000008)
    };
    match class {
        0 => {
            // SectionBasicInformation (24 bytes on x64):
            //   0x00: PVOID BaseAddress (8 bytes)
            //   0x08: ULONG AllocationAttributes (4 bytes, SEC_COMMIT = 0x08000000)
            //   0x10: LARGE_INTEGER MaximumSize (8 bytes)
            if !return_length.is_null() {
                unsafe { return_length.write_unaligned(24) };
            }
            if length < 24 || buffer.is_null() {
                return -0x3fff_fffc; // STATUS_INFO_LENGTH_MISMATCH (0xC0000004)
            }
            unsafe {
                buffer.write_bytes(0, 24);
                (buffer.add(8) as *mut u32).write_unaligned(0x0800_0000); // SEC_COMMIT
                (buffer.add(16) as *mut u64).write_unaligned(mapping.size);
            }
            0 // STATUS_SUCCESS
        }
        _ => -0x3fff_fffd, // STATUS_INVALID_INFO_CLASS (0xC0000003)
    }
}

