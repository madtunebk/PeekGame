//! File handles and path operations (ported from peek's files.inc, find_file.inc,
//! file_info.inc and mutations.inc). Handles are tokens into a table of host files;
//! 0x100-0x102 stay the standard streams. Windows paths map to the host with drive
//! Z: as the root. Overlapped I/O completes immediately, at the given offset.
use crate::{
    api::{system::error, text},
    host::{paths, stdio},
};
use std::{
    collections::HashMap,
    ffi::c_char,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
const INVALID_HANDLE: usize = usize::MAX;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_INVALID_HANDLE: u32 = 6;
const ERROR_NO_MORE_FILES: u32 = 18;
const ERROR_HANDLE_EOF: u32 = 38;
const ERROR_FILE_EXISTS: u32 = 80;
const ERROR_ALREADY_EXISTS: u32 = 183;
const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OVERLAPPED: u32 = 0x4000_0000;
/// Seconds between 1601-01-01 and 1970-01-01.
const EPOCH_1601: i64 = 11_644_473_600;

struct Entry {
    file: File,
    overlapped: bool,
}
type Table = HashMap<usize, Arc<Entry>>;
static FILES: Mutex<Option<Table>> = Mutex::new(None);
static FINDS: Mutex<Option<HashMap<usize, Search>>> = Mutex::new(None);
static NEXT: AtomicUsize = AtomicUsize::new(0x4800_0000);

fn insert(entry: Entry) -> usize {
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    FILES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(handle, Arc::new(entry));
    handle
}
fn get(handle: usize) -> Option<Arc<Entry>> {
    FILES.lock().unwrap().as_ref()?.get(&handle).cloned()
}
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
/// CloseHandle for files, pipes and directory handles. False if not one.
pub fn close(handle: usize) -> bool {
    FILES
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|t| t.remove(&handle))
        .is_some()
}
/// Error for an I/O failure on `path`: ENOENT is FILE_NOT_FOUND when the parent
/// directory exists and PATH_NOT_FOUND otherwise.
fn path_error(path: &Path, e: &std::io::Error) -> u32 {
    match e.raw_os_error() {
        Some(2) => {
            if path
                .parent()
                .is_none_or(|p| p.as_os_str().is_empty() || p.is_dir())
            {
                ERROR_FILE_NOT_FOUND
            } else {
                ERROR_PATH_NOT_FOUND
            }
        }
        Some(17) => ERROR_ALREADY_EXISTS,
        Some(21 | 1 | 13 | 30) => ERROR_ACCESS_DENIED,
        Some(36) => 206,
        _ => paths::error(e),
    }
}
unsafe fn wide_path(name: *const u16) -> Result<PathBuf, u32> {
    let text = unsafe { text::string(name) }.ok_or(87u32)?;
    paths::host(&text)
}
unsafe fn narrow_path(name: *const c_char) -> Result<PathBuf, u32> {
    unsafe { paths::narrow(name) }.and_then(|s| paths::host(&s))
}
fn filetime(seconds: i64, nanoseconds: i64) -> u64 {
    ((seconds + EPOCH_1601) as u64) * 10_000_000 + nanoseconds as u64 / 100
}
fn attributes(meta: &std::fs::Metadata, name: Option<&str>) -> u32 {
    let mut a = if meta.is_dir() {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        0
    };
    if name.is_some_and(|n| n.starts_with('.') && n != "." && n != "..") {
        a |= FILE_ATTRIBUTE_HIDDEN;
    }
    if meta.mode() & 0o200 == 0 {
        a |= FILE_ATTRIBUTE_READONLY;
    }
    if a == 0 {
        FILE_ATTRIBUTE_NORMAL
    } else {
        a
    }
}

/// GetFileType for a file handle: disk, character device or pipe.
pub(crate) fn file_type(handle: usize) -> Option<u32> {
    use std::os::unix::fs::FileTypeExt;
    let kind = get(handle)?.file.metadata().ok()?.file_type();
    Some(if kind.is_char_device() {
        2
    } else if kind.is_fifo() || kind.is_socket() {
        3
    } else {
        1
    })
}

// ---- opening -------------------------------------------------------------

fn create(path: PathBuf, access: u32, disposition: u32, flags: u32) -> usize {
    // GENERIC_READ/WRITE/ALL or FILE_READ_DATA (1), FILE_WRITE_DATA (2), FILE_APPEND_DATA (4).
    let reads = access & 0x9000_0001 != 0;
    let writes = access & 0x5000_0006 != 0;
    let mut options = OpenOptions::new();
    options.read(reads || !writes).write(writes);
    if writes && access & 0x5000_0002 == 0 {
        options.append(true);
    }
    match disposition {
        1 => options.create_new(true),            // CREATE_NEW
        2 => options.create(true).truncate(true), // CREATE_ALWAYS
        3 => &mut options,                        // OPEN_EXISTING
        4 => options.create(true),                // OPEN_ALWAYS
        5 => options.truncate(true),              // TRUNCATE_EXISTING
        _ => return fail(87, INVALID_HANDLE),
    };
    if writes && matches!(disposition, 2 | 4) {
        options.write(true);
    }
    options.mode(0o644);
    let existed = matches!(disposition, 2 | 4) && path.exists();
    let file = match options.open(&path) {
        Ok(file) => file,
        // FILE_FLAG_BACKUP_SEMANTICS opens directories (read-only on Linux).
        Err(e) if e.raw_os_error() == Some(21) && flags & FILE_FLAG_BACKUP_SEMANTICS != 0 => {
            match File::open(&path) {
                Ok(file) => file,
                Err(e) => return fail(path_error(&path, &e), INVALID_HANDLE),
            }
        }
        Err(e) if e.raw_os_error() == Some(17) => return fail(ERROR_FILE_EXISTS, INVALID_HANDLE),
        Err(e) => return fail(path_error(&path, &e), INVALID_HANDLE),
    };
    if file.metadata().is_ok_and(|m| m.is_dir()) && flags & FILE_FLAG_BACKUP_SEMANTICS == 0 {
        return fail(ERROR_ACCESS_DENIED, INVALID_HANDLE);
    }
    // CREATE_ALWAYS and OPEN_ALWAYS report an existing file with ERROR_ALREADY_EXISTS.
    error::set(if existed { ERROR_ALREADY_EXISTS } else { 0 });
    insert(Entry {
        file,
        overlapped: flags & FILE_FLAG_OVERLAPPED != 0,
    })
}
pub unsafe extern "win64" fn create_w(
    name: *const u16,
    access: u32,
    _share: u32,
    _security: usize,
    disposition: u32,
    flags: u32,
    _template: usize,
) -> usize {
    match unsafe { wide_path(name) } {
        Ok(path) => create(path, access, disposition, flags),
        Err(code) => fail(code, INVALID_HANDLE),
    }
}
pub unsafe extern "win64" fn create_a(
    name: *const c_char,
    access: u32,
    _share: u32,
    _security: usize,
    disposition: u32,
    flags: u32,
    _template: usize,
) -> usize {
    match unsafe { narrow_path(name) } {
        Ok(path) => create(path, access, disposition, flags),
        Err(code) => fail(code, INVALID_HANDLE),
    }
}
/// CreatePipe: an anonymous pipe as two file handles.
pub unsafe extern "win64" fn create_pipe(
    read: *mut usize,
    write: *mut usize,
    _security: usize,
    _size: u32,
) -> i32 {
    unsafe extern "C" {
        fn pipe2(fds: *mut i32, flags: i32) -> i32;
    }
    if read.is_null() || write.is_null() {
        return fail(87, 0);
    }
    let mut fds = [0i32; 2];
    if unsafe { pipe2(fds.as_mut_ptr(), 0o2000000) } != 0 {
        return fail(8, 0);
    }
    use std::os::fd::FromRawFd;
    unsafe {
        *read = insert(Entry {
            file: File::from_raw_fd(fds[0]),
            overlapped: false,
        });
        *write = insert(Entry {
            file: File::from_raw_fd(fds[1]),
            overlapped: false,
        });
    }
    1
}

// ---- reading and writing ------------------------------------------------

/// OVERLAPPED transfer done now at its offset: Internal/InternalHigh are filled,
/// the event (if any) is signalled and a synchronous handle's pointer moves past it.
unsafe fn overlapped(
    entry: &Entry,
    buffer: *mut u8,
    n: u32,
    count: *mut u32,
    ov: *mut u8,
    writing: bool,
) -> i32 {
    let offset = unsafe {
        (ov.add(16) as *const u32).read_unaligned() as u64
            | ((ov.add(20) as *const u32).read_unaligned() as u64) << 32
    };
    let event = unsafe { (ov.add(24) as *const usize).read_unaligned() };
    let result = if writing {
        entry.file.write_at(
            unsafe { std::slice::from_raw_parts(buffer, n as usize) },
            offset,
        )
    } else {
        entry.file.read_at(
            unsafe { std::slice::from_raw_parts_mut(buffer, n as usize) },
            offset,
        )
    };
    let (status, moved): (i64, u64) = match &result {
        Ok(0) if !writing && n > 0 => (0xc000_0011u32 as i32 as i64, 0), // STATUS_END_OF_FILE
        Ok(done) => (0, *done as u64),
        Err(_) => (0xc000_0185u32 as i32 as i64, 0),
    };
    unsafe {
        (ov as *mut i64).write_unaligned(status);
        (ov.add(8) as *mut u64).write_unaligned(moved);
        if !count.is_null() {
            *count = moved as u32;
        }
    }
    if !entry.overlapped && result.is_ok() {
        let _ = (&entry.file).seek(SeekFrom::Start(offset + moved));
    }
    if event != 0 {
        crate::api::threading::events::set(event & !1);
    }
    match result {
        Ok(0) if !writing && n > 0 => fail(ERROR_HANDLE_EOF, 0),
        Ok(_) => 1,
        Err(e) => fail(paths::error(&e), 0),
    }
}
pub unsafe extern "win64" fn read(
    handle: usize,
    out: *mut u8,
    n: u32,
    count: *mut u32,
    ov: *mut u8,
) -> i32 {
    if !count.is_null() {
        unsafe { *count = 0 };
    }
    let Some(entry) = get(handle) else {
        if let Some(fd) = stdio::fd_for(handle) {
            let mut stream = unsafe {
                std::mem::ManuallyDrop::new(<File as std::os::fd::FromRawFd>::from_raw_fd(fd))
            };
            return match stream.read(unsafe { std::slice::from_raw_parts_mut(out, n as usize) }) {
                Ok(done) => {
                    if !count.is_null() {
                        unsafe { *count = done as u32 };
                    }
                    1
                }
                Err(e) => fail(paths::error(&e), 0),
            };
        }
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    if !ov.is_null() {
        return unsafe { overlapped(&entry, out, n, count, ov, false) };
    }
    if entry.overlapped {
        return fail(87, 0);
    }
    match (&entry.file).read(unsafe { std::slice::from_raw_parts_mut(out, n as usize) }) {
        Ok(done) => {
            if !count.is_null() {
                unsafe { *count = done as u32 };
            }
            1
        }
        Err(e) => fail(paths::error(&e), 0),
    }
}
/// WriteFile on a file handle; false if the handle is not a file (the caller then
/// tries the standard streams).
pub(crate) unsafe fn write(
    handle: usize,
    data: *const u8,
    n: u32,
    written: *mut u32,
    ov: *mut u8,
) -> Option<i32> {
    let entry = get(handle)?;
    if !ov.is_null() {
        return Some(unsafe { overlapped(&entry, data as *mut u8, n, written, ov, true) });
    }
    if entry.overlapped {
        return Some(fail(87, 0));
    }
    let data = if n == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(data, n as usize) }
    };
    Some(match (&entry.file).write(data) {
        Ok(done) => {
            if !written.is_null() {
                unsafe { *written = done as u32 };
            }
            1
        }
        Err(e) => fail(paths::error(&e), 0),
    })
}
pub extern "win64" fn flush(handle: usize) -> i32 {
    match get(handle) {
        Some(entry) => match entry.file.sync_all() {
            Ok(()) => 1,
            Err(e) => fail(paths::error(&e), 0),
        },
        None => fail(ERROR_INVALID_HANDLE, 0),
    }
}
fn seek(handle: usize, offset: i64, method: u32) -> Result<u64, u32> {
    let entry = get(handle).ok_or(ERROR_INVALID_HANDLE)?;
    let from = match method {
        0 => SeekFrom::Start(offset.try_into().map_err(|_| 131u32)?),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return Err(87),
    };
    (&entry.file).seek(from).map_err(|_| 131) // ERROR_NEGATIVE_SEEK
}
pub unsafe extern "win64" fn set_pointer(
    handle: usize,
    low: i32,
    high: *mut i32,
    method: u32,
) -> u32 {
    let offset = if high.is_null() {
        low as i64
    } else {
        ((unsafe { *high } as i64) << 32) | low as u32 as i64
    };
    match seek(handle, offset, method) {
        Ok(position) => {
            if !high.is_null() {
                unsafe { *high = (position >> 32) as i32 };
            }
            error::set(0);
            position as u32
        }
        Err(code) => fail(code, u32::MAX),
    }
}
pub unsafe extern "win64" fn set_pointer_ex(
    handle: usize,
    offset: i64,
    out: *mut i64,
    method: u32,
) -> i32 {
    match seek(handle, offset, method) {
        Ok(position) => {
            if !out.is_null() {
                unsafe { *out = position as i64 };
            }
            1
        }
        Err(code) => fail(code, 0),
    }
}
pub extern "win64" fn set_end_of_file(handle: usize) -> i32 {
    let Some(entry) = get(handle) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let result = (&entry.file)
        .stream_position()
        .and_then(|p| entry.file.set_len(p));
    match result {
        Ok(()) => 1,
        Err(e) => fail(paths::error(&e), 0),
    }
}
pub unsafe extern "win64" fn size(handle: usize, high: *mut u32) -> u32 {
    match get(handle).and_then(|e| e.file.metadata().ok()) {
        Some(meta) => {
            if !high.is_null() {
                unsafe { *high = (meta.len() >> 32) as u32 };
            }
            meta.len() as u32
        }
        None => fail(ERROR_INVALID_HANDLE, u32::MAX),
    }
}
pub unsafe extern "win64" fn size_ex(handle: usize, out: *mut i64) -> i32 {
    match get(handle).and_then(|e| e.file.metadata().ok()) {
        Some(meta) => {
            unsafe { *out = meta.len() as i64 };
            1
        }
        None => fail(ERROR_INVALID_HANDLE, 0),
    }
}
/// PeekNamedPipe: bytes available on a pipe (FIONREAD); no data is read.
pub unsafe extern "win64" fn peek_named_pipe(
    handle: usize,
    _buffer: usize,
    _size: u32,
    read: *mut u32,
    available: *mut u32,
    left: *mut u32,
) -> i32 {
    unsafe extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    use std::os::fd::AsRawFd;
    let Some(entry) = get(handle) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let mut n: i32 = 0;
    if unsafe { ioctl(entry.file.as_raw_fd(), 0x541B, &mut n) } != 0 {
        return fail(ERROR_INVALID_HANDLE, 0);
    }
    unsafe {
        for (p, v) in [(read, 0), (available, n as u32), (left, 0)] {
            if !p.is_null() {
                *p = v;
            }
        }
    }
    1
}
/// CancelIo / CancelIoEx: overlapped I/O always completes at once, so nothing is
/// pending (CancelIoEx reports ERROR_NOT_FOUND, as Windows does then).
pub extern "win64" fn cancel_io(handle: usize) -> i32 {
    if get(handle).is_some() {
        1
    } else {
        fail(ERROR_INVALID_HANDLE, 0)
    }
}
pub extern "win64" fn cancel_io_ex(handle: usize, _overlapped: usize) -> i32 {
    if get(handle).is_some() {
        fail(1168, 0)
    } else {
        fail(ERROR_INVALID_HANDLE, 0)
    }
}
/// GetOverlappedResult: the transfer already finished; report Internal/InternalHigh.
pub unsafe extern "win64" fn overlapped_result(
    handle: usize,
    ov: *const u8,
    transferred: *mut u32,
    _wait: i32,
) -> i32 {
    if get(handle).is_none() || ov.is_null() {
        return fail(ERROR_INVALID_HANDLE, 0);
    }
    let status = unsafe { (ov as *const i64).read_unaligned() } as u32;
    let moved = unsafe { (ov.add(8) as *const u64).read_unaligned() };
    if !transferred.is_null() {
        unsafe { *transferred = moved as u32 };
    }
    match status {
        0 => 1,
        0xc000_0011 => fail(ERROR_HANDLE_EOF, 0),
        _ => fail(31, 0), // ERROR_GEN_FAILURE
    }
}

// ---- metadata -------------------------------------------------------------

/// WIN32_FILE_ATTRIBUTE_DATA (36 bytes): attributes, three FILETIMEs, size.
unsafe fn store_attribute_data(meta: &std::fs::Metadata, name: Option<&str>, out: *mut u8) {
    let mut d = [0u8; 36];
    d[0..4].copy_from_slice(&attributes(meta, name).to_le_bytes());
    d[4..12].copy_from_slice(&filetime(meta.ctime(), meta.ctime_nsec()).to_le_bytes());
    d[12..20].copy_from_slice(&filetime(meta.atime(), meta.atime_nsec()).to_le_bytes());
    d[20..28].copy_from_slice(&filetime(meta.mtime(), meta.mtime_nsec()).to_le_bytes());
    let size = if meta.is_dir() { 0 } else { meta.len() };
    d[28..32].copy_from_slice(&((size >> 32) as u32).to_le_bytes());
    d[32..36].copy_from_slice(&(size as u32).to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), out, 36) };
}
pub unsafe extern "win64" fn attributes_ex_w(name: *const u16, level: i32, out: *mut u8) -> i32 {
    if level != 0 || out.is_null() {
        return fail(87, 0);
    }
    let path = match unsafe { wide_path(name) } {
        Ok(p) => p,
        Err(code) => return fail(code, 0),
    };
    match std::fs::metadata(&path) {
        Ok(meta) => {
            unsafe { store_attribute_data(&meta, path.file_name().and_then(|n| n.to_str()), out) };
            1
        }
        Err(e) => fail(path_error(&path, &e), 0),
    }
}
/// BY_HANDLE_FILE_INFORMATION (52 bytes).
pub unsafe extern "win64" fn information_by_handle(handle: usize, out: *mut u8) -> i32 {
    let Some(meta) = get(handle).and_then(|e| e.file.metadata().ok()) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    if out.is_null() {
        return fail(998, 0);
    }
    let mut d = [0u8; 52];
    unsafe { store_attribute_data(&meta, None, d.as_mut_ptr()) };
    // The size moves from @28 to @32; serial, links and index fill the gap.
    let size = if meta.is_dir() { 0 } else { meta.len() };
    d[28..32].copy_from_slice(&(meta.dev() as u32).to_le_bytes());
    d[32..36].copy_from_slice(&((size >> 32) as u32).to_le_bytes());
    d[36..40].copy_from_slice(&(size as u32).to_le_bytes());
    d[40..44].copy_from_slice(&(meta.nlink() as u32).to_le_bytes());
    d[44..48].copy_from_slice(&((meta.ino() >> 32) as u32).to_le_bytes());
    d[48..52].copy_from_slice(&(meta.ino() as u32).to_le_bytes());
    unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), out, 52) };
    1
}
/// GetFileInformationByHandleEx: FileBasicInfo (0), FileStandardInfo (1),
/// FileAttributeTagInfo (9), FileIdInfo (18).
pub unsafe extern "win64" fn information_by_handle_ex(
    handle: usize,
    class: i32,
    out: *mut u8,
    size: u32,
) -> i32 {
    let needed = match class {
        0 => 40,
        1 => 24,
        9 => 8,
        18 => 24,
        _ => {
            eprintln!("GetFileInformationByHandleEx: information class {class} is not supported");
            return fail(87, 0);
        }
    };
    if out.is_null() || size < needed {
        return fail(24, 0);
    }
    let Some(meta) = get(handle).and_then(|e| e.file.metadata().ok()) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let mut d = vec![0u8; needed as usize];
    let attr = attributes(&meta, None);
    match class {
        0 => {
            let (c, a, w) = (
                filetime(meta.ctime(), meta.ctime_nsec()),
                filetime(meta.atime(), meta.atime_nsec()),
                filetime(meta.mtime(), meta.mtime_nsec()),
            );
            for (i, v) in [c, a, w, c].iter().enumerate() {
                d[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
            }
            d[32..36].copy_from_slice(&attr.to_le_bytes());
        }
        1 => {
            d[0..8].copy_from_slice(&(meta.blocks() * 512).to_le_bytes());
            d[8..16].copy_from_slice(&(if meta.is_dir() { 0 } else { meta.len() }).to_le_bytes());
            d[16..20].copy_from_slice(&(meta.nlink() as u32).to_le_bytes());
            d[21] = meta.is_dir() as u8;
        }
        9 => d[0..4].copy_from_slice(&attr.to_le_bytes()),
        _ => {
            d[0..8].copy_from_slice(&meta.dev().to_le_bytes());
            d[8..16].copy_from_slice(&meta.ino().to_le_bytes());
        }
    }
    unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), out, d.len()) };
    1
}
/// SetFileTime: last access and last write (creation time cannot be set on Linux
/// and is ignored).
pub unsafe extern "win64" fn set_time(
    handle: usize,
    _created: *const u64,
    accessed: *const u64,
    written: *const u64,
) -> i32 {
    let Some(entry) = get(handle) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let to_time = |p: *const u64| {
        (!p.is_null() && unsafe { *p } != 0).then(|| {
            let ticks = unsafe { *p };
            std::time::UNIX_EPOCH
                + std::time::Duration::from_nanos((ticks - (EPOCH_1601 as u64) * 10_000_000) * 100)
        })
    };
    let mut times = std::fs::FileTimes::new();
    if let Some(t) = to_time(accessed) {
        times = times.set_accessed(t);
    }
    if let Some(t) = to_time(written) {
        times = times.set_modified(t);
    }
    match entry.file.set_times(times) {
        Ok(()) => 1,
        Err(e) => fail(paths::error(&e), 0),
    }
}

// ---- directory searches ---------------------------------------------------

struct Search {
    entries: std::vec::IntoIter<(String, std::fs::Metadata)>,
}
/// Windows wildcard match: `*` and `?`, case-insensitive; `*.*` also matches names
/// without an extension.
fn matches(pattern: &str, name: &str) -> bool {
    if pattern == "*.*" || pattern == "*" {
        return true;
    }
    fn go(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some('*'), _) => go(&p[1..], n) || (!n.is_empty() && go(p, &n[1..])),
            (Some('?'), Some(_)) => go(&p[1..], &n[1..]),
            (Some(a), Some(b)) => a.to_lowercase().eq(b.to_lowercase()) && go(&p[1..], &n[1..]),
            _ => false,
        }
    }
    go(
        &pattern.chars().collect::<Vec<_>>(),
        &name.chars().collect::<Vec<_>>(),
    )
}
/// WIN32_FIND_DATAW (592 bytes).
unsafe fn store_find_data(name: &str, meta: &std::fs::Metadata, out: *mut u8) -> bool {
    let wide = text::wide(name);
    if wide.len() >= 260 {
        return false;
    }
    let mut d = [0u8; 592];
    unsafe { store_attribute_data(meta, Some(name), d.as_mut_ptr()) };
    for (i, u) in wide.iter().enumerate() {
        d[44 + i * 2..46 + i * 2].copy_from_slice(&u.to_le_bytes());
    }
    unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), out, 592) };
    true
}
fn next(search: &mut Search, out: *mut u8) -> bool {
    for (name, meta) in search.entries.by_ref() {
        if unsafe { store_find_data(&name, &meta, out) } {
            return true;
        }
    }
    false
}
pub unsafe extern "win64" fn find_first_ex_w(
    pattern: *const u16,
    level: i32,
    out: *mut u8,
    op: i32,
    filter: usize,
    flags: u32,
) -> usize {
    if out.is_null()
        || !(0..=1).contains(&level)
        || !(0..=1).contains(&op)
        || filter != 0
        || flags & !2 != 0
    {
        return fail(87, INVALID_HANDLE);
    }
    let path = match unsafe { wide_path(pattern) } {
        Ok(p) => p,
        Err(code) => return fail(code, INVALID_HANDLE),
    };
    let glob = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if glob.is_empty() {
        return fail(87, INVALID_HANDLE);
    }
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // A pattern without wildcards names one entry, "." and ".." included.
    let mut entries = Vec::new();
    if !glob.contains(['*', '?']) {
        if let Ok(meta) = std::fs::metadata(&path) {
            entries.push((glob.clone(), meta));
        }
    } else {
        let listing = match std::fs::read_dir(directory) {
            Ok(l) => l,
            Err(e) => return fail(path_error(directory, &e), INVALID_HANDLE),
        };
        for name in [".", ".."] {
            if matches(&glob, name) {
                if let Ok(meta) = std::fs::metadata(directory.join(name)) {
                    entries.push((name.to_owned(), meta));
                }
            }
        }
        for entry in listing.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if matches(&glob, &name) {
                if let Ok(meta) = std::fs::metadata(entry.path()) {
                    entries.push((name, meta));
                }
            }
        }
    }
    if op == 1 {
        entries.retain(|(_, m)| m.is_dir()); // FindExSearchLimitToDirectories
    }
    let mut search = Search {
        entries: entries.into_iter(),
    };
    if !next(&mut search, out) {
        return fail(ERROR_FILE_NOT_FOUND, INVALID_HANDLE);
    }
    let handle = NEXT.fetch_add(16, Ordering::Relaxed);
    FINDS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(handle, search);
    handle
}
pub unsafe extern "win64" fn find_first_w(pattern: *const u16, out: *mut u8) -> usize {
    unsafe { find_first_ex_w(pattern, 0, out, 0, 0, 0) }
}
pub unsafe extern "win64" fn find_next_w(handle: usize, out: *mut u8) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let mut finds = FINDS.lock().unwrap();
    let Some(search) = finds.as_mut().and_then(|f| f.get_mut(&handle)) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    if next(search, out) {
        1
    } else {
        fail(ERROR_NO_MORE_FILES, 0)
    }
}
pub extern "win64" fn find_close(handle: usize) -> i32 {
    match FINDS
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|f| f.remove(&handle))
    {
        Some(_) => 1,
        None => fail(ERROR_INVALID_HANDLE, 0),
    }
}

// ---- path operations ------------------------------------------------------

fn result(path: &Path, r: std::io::Result<()>) -> i32 {
    match r {
        Ok(()) => 1,
        Err(e) => fail(path_error(path, &e), 0),
    }
}
pub unsafe extern "win64" fn create_directory_w(name: *const u16, _security: usize) -> i32 {
    match unsafe { wide_path(name) } {
        Ok(path) => result(&path, std::fs::create_dir(&path)),
        Err(code) => fail(code, 0),
    }
}
pub unsafe extern "win64" fn delete_w(name: *const u16) -> i32 {
    match unsafe { wide_path(name) } {
        Ok(path) => result(&path, std::fs::remove_file(&path)),
        Err(code) => fail(code, 0),
    }
}
pub unsafe extern "win64" fn remove_directory_w(name: *const u16) -> i32 {
    match unsafe { wide_path(name) } {
        Ok(path) => result(&path, std::fs::remove_dir(&path)),
        Err(code) => fail(code, 0),
    }
}
fn rename(from: &Path, to: &Path, replace: bool) -> i32 {
    unsafe extern "C" {
        fn syscall(number: i64, ...) -> i64;
    }
    use std::os::unix::ffi::OsStrExt;
    let (Ok(a), Ok(b)) = (
        std::ffi::CString::new(from.as_os_str().as_bytes()),
        std::ffi::CString::new(to.as_os_str().as_bytes()),
    ) else {
        return fail(87, 0);
    };
    const AT_FDCWD: i64 = -100;
    const RENAME_NOREPLACE: i64 = 1;
    let flags = if replace { 0 } else { RENAME_NOREPLACE };
    if unsafe { syscall(316, AT_FDCWD, a.as_ptr(), AT_FDCWD, b.as_ptr(), flags) } == 0 {
        return 1;
    }
    let e = std::io::Error::last_os_error();
    fail(path_error(if from.exists() { to } else { from }, &e), 0)
}
pub unsafe extern "win64" fn move_ex_w(from: *const u16, to: *const u16, flags: u32) -> i32 {
    if flags & !1 != 0 || to.is_null() {
        return fail(if to.is_null() { 87 } else { 50 }, 0);
    }
    match (unsafe { wide_path(from) }, unsafe { wide_path(to) }) {
        (Ok(a), Ok(b)) => rename(&a, &b, flags & 1 != 0),
        (Err(code), _) | (_, Err(code)) => fail(code, 0),
    }
}
/// ReplaceFileW without a backup: the replacement takes the replaced file's name.
pub unsafe extern "win64" fn replace_w(
    replaced: *const u16,
    replacement: *const u16,
    backup: *const u16,
    _flags: u32,
    _a: usize,
    _b: usize,
) -> i32 {
    if !backup.is_null() {
        eprintln!("ReplaceFileW: backup files are not supported");
        return fail(50, 0);
    }
    match (unsafe { wide_path(replaced) }, unsafe {
        wide_path(replacement)
    }) {
        (Ok(a), Ok(b)) => {
            if !a.exists() {
                return fail(1176, 0); // ERROR_UNABLE_TO_REMOVE_REPLACED
            }
            rename(&b, &a, true)
        }
        (Err(code), _) | (_, Err(code)) => fail(code, 0),
    }
}
pub unsafe extern "win64" fn copy_w(from: *const u16, to: *const u16, fail_if_exists: i32) -> i32 {
    match (unsafe { wide_path(from) }, unsafe { wide_path(to) }) {
        (Ok(a), Ok(b)) => {
            if fail_if_exists != 0 && b.exists() {
                return fail(ERROR_FILE_EXISTS, 0);
            }
            result(&a, std::fs::copy(&a, &b).map(|_| ()))
        }
        (Err(code), _) | (_, Err(code)) => fail(code, 0),
    }
}
/// SetFileAttributesW: READONLY clears or restores write permission; ARCHIVE,
/// NORMAL, TEMPORARY, NOT_CONTENT_INDEXED and DIRECTORY carry no state on Linux.
pub unsafe extern "win64" fn set_attributes_w(name: *const u16, attributes: u32) -> i32 {
    let path = match unsafe { wide_path(name) } {
        Ok(p) => p,
        Err(code) => return fail(code, 0),
    };
    if attributes & !(0x1 | 0x10 | 0x20 | 0x80 | 0x100 | 0x2000) != 0 {
        eprintln!(
            "SetFileAttributesW({}, {attributes:#x}): hidden/system attributes are not supported",
            path.display()
        );
        return fail(50, 0);
    }
    let meta = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(e) => return fail(path_error(&path, &e), 0),
    };
    let mode = meta.mode() & 0o7777;
    let wanted = if attributes & 1 != 0 {
        mode & !0o222
    } else {
        mode | ((mode & 0o444) >> 1)
    };
    if wanted == mode {
        return 1;
    }
    result(
        &path,
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(wanted)),
    )
}
/// GetTempFileNameW: PREFIX + hex unique + ".tmp" in `directory`; with unique 0 a
/// free name is found and the file created.
pub unsafe extern "win64" fn temp_file_name_w(
    directory: *const u16,
    prefix: *const u16,
    unique: u32,
    out: *mut u16,
) -> u32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let dir = unsafe { text::string(directory) }.unwrap_or_default();
    let prefix: String = unsafe { text::string(prefix) }
        .unwrap_or_default()
        .chars()
        .take(3)
        .collect();
    let host = match paths::host(&dir) {
        Ok(p) => p,
        Err(code) => return fail(code, 0),
    };
    let base = if unique != 0 {
        unique
    } else {
        (std::process::id() ^ crate::api::system::time::tick_count()) & 0xffff
    };
    for step in 0..0x10000u32 {
        let number = if unique != 0 {
            unique
        } else {
            ((base + step) & 0xffff).max(1)
        };
        let name = format!("{prefix}{number:X}.tmp");
        let path = host.join(&name);
        if unique == 0 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => {}
                Err(e) if e.raw_os_error() == Some(17) => continue,
                Err(e) => return fail(path_error(&path, &e), 0),
            }
        }
        let windows = format!("{}\\{name}", dir.trim_end_matches(['\\', '/']));
        let wide = text::wide(&windows);
        unsafe { text::copy_out(&wide, out, 260) };
        return number;
    }
    fail(80, 0)
}
/// GetDiskFreeSpaceExW from statvfs of the path.
pub unsafe extern "win64" fn disk_free_space_ex_w(
    name: *const u16,
    available: *mut u64,
    total: *mut u64,
    free: *mut u64,
) -> i32 {
    #[repr(C)]
    struct Statvfs {
        bsize: u64,
        frsize: u64,
        blocks: u64,
        bfree: u64,
        bavail: u64,
        rest: [u64; 8],
    }
    unsafe extern "C" {
        fn statvfs(path: *const c_char, out: *mut Statvfs) -> i32;
    }
    let path = if name.is_null() {
        std::env::current_dir().unwrap_or_default()
    } else {
        match unsafe { wide_path(name) } {
            Ok(p) => p,
            Err(code) => return fail(code, 0),
        }
    };
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return fail(87, 0);
    };
    let mut s = Statvfs {
        bsize: 0,
        frsize: 0,
        blocks: 0,
        bfree: 0,
        bavail: 0,
        rest: [0; 8],
    };
    if unsafe { statvfs(c.as_ptr(), &mut s) } != 0 {
        return fail(ERROR_PATH_NOT_FOUND, 0);
    }
    let unit = if s.frsize != 0 { s.frsize } else { s.bsize };
    unsafe {
        for (p, v) in [
            (available, s.bavail * unit),
            (total, s.blocks * unit),
            (free, s.bfree * unit),
        ] {
            if !p.is_null() {
                *p = v;
            }
        }
    }
    1
}
/// GetDriveTypeW: Z: (the host root) is a fixed drive; other drives do not exist.
pub unsafe extern "win64" fn drive_type_w(root: *const u16) -> u32 {
    let root = unsafe { text::string(root) }.unwrap_or_default();
    if root.is_empty() || root.starts_with(['Z', 'z', '\\', '/']) {
        3 // DRIVE_FIXED
    } else {
        1 // DRIVE_NO_ROOT_DIR
    }
}
