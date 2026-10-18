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
const FILE_FLAG_DELETE_ON_CLOSE: u32 = 0x0400_0000;
/// Seconds between 1601-01-01 and 1970-01-01.
const EPOCH_1601: i64 = 11_644_473_600;

pub(super) struct Entry {
    pub(super) file: File,
    pub(super) directory: Mutex<Option<super::directory::Scan>>,
    overlapped: bool,
    completion: Mutex<Option<crate::api::threading::iocp::Binding>>,
    completion_modes: std::sync::atomic::AtomicU8,
    /// The host path the handle was opened with (follows renames through the
    /// handle); None for pipes and standard streams.
    path: Mutex<Option<PathBuf>>,
    /// FILE_FLAG_DELETE_ON_CLOSE or a delete disposition: the file is removed when
    /// the last handle to this entry closes (duplicated handles share the entry).
    delete_on_close: std::sync::atomic::AtomicBool,
}
impl Entry {
    fn new(file: File, path: Option<PathBuf>, overlapped: bool) -> Entry {
        Entry {
            file,
            directory: Mutex::new(None),
            overlapped,
            completion: Mutex::new(None),
            completion_modes: 0.into(),
            path: Mutex::new(path),
            delete_on_close: false.into(),
        }
    }
}
impl Drop for Entry {
    fn drop(&mut self) {
        if self.delete_on_close.load(Ordering::Acquire) {
            if let Some(path) = self.path.get_mut().unwrap().take() {
                let _ = if path.is_dir() { std::fs::remove_dir(&path) } else { std::fs::remove_file(&path) };
            }
        }
    }
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
pub(super) fn get(handle: usize) -> Option<Arc<Entry>> {
    FILES.lock().unwrap().as_ref()?.get(&handle).cloned()
}
/// `PEEKGAME_TRACE_FILES`: log file opens, reads and mappings to stderr.
pub(crate) fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_FILES").is_some())
}
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
/// CloseHandle for files, pipes and directory handles. False if not one.
pub fn close(handle: usize) -> bool {
    if super::pipes::close(handle) {
        return true;
    }
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
pub(crate) fn filetime(seconds: i64, nanoseconds: i64) -> u64 {
    ((seconds + EPOCH_1601) as u64) * 10_000_000 + nanoseconds as u64 / 100
}
pub(crate) fn attributes(meta: &std::fs::Metadata, name: Option<&str>) -> u32 {
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

/// DuplicateHandle for files, pipes and the standard streams: a new handle on a
/// duplicated descriptor (shared file position, as on Windows).
pub(crate) fn duplicate(handle: usize) -> Option<usize> {
    if let Some(target) = super::pipes::duplicate(handle) {
        return Some(target);
    }
    if let Some(entry) = get(handle) {
        let target = NEXT.fetch_add(16, Ordering::Relaxed);
        FILES
            .lock()
            .unwrap()
            .get_or_insert_with(HashMap::new)
            .insert(target, entry);
        return Some(target);
    }
    let fd = stdio::fd_for(handle)?;
    let borrowed = unsafe { std::os::fd::BorrowedFd::borrow_raw(fd) };
    let file = File::from(borrowed.try_clone_to_owned().ok()?);
    Some(insert(Entry::new(file, None, false)))
}
/// A file handle's file, as a new descriptor (CreateFileMapping keeps its own).
pub(crate) fn clone_file(handle: usize) -> Option<File> {
    get(handle)?.file.try_clone().ok()
}
/// GetFileType for a file handle: disk, character device or pipe.
pub(crate) fn file_type(handle: usize) -> Option<u32> {
    if super::pipes::known(handle) {
        return Some(3);
    }
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
    // Rust refuses to create without write access, Windows does not (GENERIC_READ
    // with OPEN_ALWAYS/CREATE_NEW/CREATE_ALWAYS makes the file): open(2) flags then.
    const O_CREAT: i32 = 0o100;
    const O_EXCL: i32 = 0o200;
    const O_TRUNC: i32 = 0o1000;
    match (disposition, writes) {
        (1, true) => options.create_new(true),            // CREATE_NEW
        (2, true) => options.create(true).truncate(true), // CREATE_ALWAYS
        (4, true) => options.create(true),                // OPEN_ALWAYS
        (1, false) => options.custom_flags(O_CREAT | O_EXCL),
        (2, false) => options.custom_flags(O_CREAT | O_TRUNC),
        (4, false) => options.custom_flags(O_CREAT),
        (3, _) => &mut options,                           // OPEN_EXISTING
        (5, true) => options.truncate(true),              // TRUNCATE_EXISTING
        _ => return fail(87, INVALID_HANDLE),             // TRUNCATE_EXISTING needs write access
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
        Err(e) => {
            if tracing() {
                eprintln!("[files] open {} access={access:#x} disp={disposition} flags={flags:#x} -> {e}", path.display());
            }
            return fail(path_error(&path, &e), INVALID_HANDLE);
        }
    };
    if file.metadata().is_ok_and(|m| m.is_dir()) && flags & FILE_FLAG_BACKUP_SEMANTICS == 0 {
        return fail(ERROR_ACCESS_DENIED, INVALID_HANDLE);
    }
    // CREATE_ALWAYS and OPEN_ALWAYS report an existing file with ERROR_ALREADY_EXISTS.
    error::set(if existed { ERROR_ALREADY_EXISTS } else { 0 });
    let fd = std::os::fd::AsRawFd::as_raw_fd(&file);
    let entry = Entry::new(file, Some(path.clone()), flags & FILE_FLAG_OVERLAPPED != 0);
    entry.delete_on_close.store(flags & FILE_FLAG_DELETE_ON_CLOSE != 0, Ordering::Release);
    let handle = insert(entry);
    if tracing() {
        eprintln!("[files] open {} access={access:#x} disp={disposition} flags={flags:#x} -> {handle:#x} fd={fd}", path.display());
    }
    handle
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
    if let Ok(raw) = unsafe { paths::wide(name) } {
        if let Some(handle) = super::pipes::open(&raw, access, disposition, flags) {
            return handle;
        }
    }
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
    if let Ok(raw) = unsafe { paths::narrow(name) } {
        if let Some(handle) = super::pipes::open(&raw, access, disposition, flags) {
            return handle;
        }
    }
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
        *read = insert(Entry::new(File::from_raw_fd(fds[0]), None, false));
        *write = insert(Entry::new(File::from_raw_fd(fds[1]), None, false));
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
    unsafe {
        overlapped_with(entry, n, count, ov, writing, |offset| {
            if writing {
                entry.file.write_at(std::slice::from_raw_parts(buffer, n as usize), offset)
            } else {
                entry.file.read_at(std::slice::from_raw_parts_mut(buffer, n as usize), offset)
            }
        })
    }
}
/// The completion half of an OVERLAPPED transfer; `transfer` moves the bytes at the
/// OVERLAPPED offset.
unsafe fn overlapped_with(
    entry: &Entry,
    n: u32,
    count: *mut u32,
    ov: *mut u8,
    writing: bool,
    transfer: impl FnOnce(u64) -> std::io::Result<usize>,
) -> i32 {
    let offset = unsafe {
        (ov.add(16) as *const u32).read_unaligned() as u64
            | ((ov.add(20) as *const u32).read_unaligned() as u64) << 32
    };
    let event = unsafe { (ov.add(24) as *const usize).read_unaligned() };
    let result = transfer(offset);
    if tracing() {
        eprintln!(
            "[files] {} ov fd={} offset={offset:#x} n={n:#x} event={event:#x} -> {result:?}",
            if writing { "write" } else { "read" },
            std::os::fd::AsRawFd::as_raw_fd(&entry.file),
        );
    }
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
    if event & 1 == 0 && !(status == 0 && entry.completion_modes.load(Ordering::Acquire) & 1 != 0) {
        if let Some(binding) = entry.completion.lock().unwrap().as_ref() {
            binding.complete(ov as usize, status as usize, moved as u32);
        }
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
    if let Some(result) = unsafe { super::pipes::io(handle, out, n, count, ov, false) } {
        return result;
    }
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
    let slice = unsafe { std::slice::from_raw_parts_mut(out, n as usize) };
    let mut total = 0;
    let mut err = None;
    while total < slice.len() {
        match (&entry.file).read(&mut slice[total..]) {
            Ok(0) => break,
            Ok(done) => total += done,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                err = Some(e);
                break;
            }
        }
    }
    if let Some(e) = err {
        if total == 0 {
            return fail(paths::error(&e), 0);
        }
    }
    if !count.is_null() {
        unsafe { *count = total as u32 };
    }
    1
}
/// ReadFileScatter: one overlapped read at the OVERLAPPED offset into page-sized
/// buffers (FILE_SEGMENT_ELEMENT, a 64-bit pointer each), done now with preadv.
/// Like Windows, the handle must be overlapped and `n` a multiple of the page size.
pub unsafe extern "win64" fn read_scatter(
    handle: usize,
    segments: *const u64,
    n: u32,
    reserved: *mut u32,
    ov: *mut u8,
) -> i32 {
    unsafe extern "C" {
        fn preadv(fd: i32, iov: *const [usize; 2], count: i32, offset: i64) -> isize;
        fn sysconf(name: i32) -> i64;
    }
    let page = unsafe { sysconf(30) } as usize; // _SC_PAGESIZE
    let Some(entry) = get(handle) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    if segments.is_null() || !reserved.is_null() || ov.is_null() || !entry.overlapped {
        return fail(87, 0);
    }
    if !(n as usize).is_multiple_of(page) {
        return fail(87, 0);
    }
    let pages: Vec<[usize; 2]> = (0..n as usize / page)
        .map(|i| [unsafe { *segments.add(i) } as usize, page])
        .collect();
    if pages.iter().any(|p| p[0] == 0 || p[0] % page != 0) {
        return fail(998, 0); // ERROR_NOACCESS
    }
    let fd = std::os::fd::AsRawFd::as_raw_fd(&entry.file);
    unsafe {
        overlapped_with(&entry, n, std::ptr::null_mut(), ov, false, |offset| {
            // preadv may stop early; continue from the first unfilled byte.
            let mut done = 0usize;
            while done < n as usize {
                let first = done / page;
                let mut iov = pages[first..].to_vec();
                iov[0][0] += done % page;
                iov[0][1] -= done % page;
                let got = preadv(fd, iov.as_ptr(), iov.len().min(1024) as i32, (offset + done as u64) as i64);
                if got < 0 {
                    let error = std::io::Error::last_os_error();
                    if error.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(error);
                }
                if got == 0 {
                    break;
                }
                done += got as usize;
            }
            if tracing() {
                eprintln!("[files] read scatter fd={fd} offset={offset:#x} n={n:#x} pages={} -> {done:#x}", pages.len());
            }
            Ok(done)
        })
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
    if let Some(result) = unsafe { super::pipes::io(handle, data as *mut u8, n, written, ov, true) }
    {
        return Some(result);
    }
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
    let result = (&entry.file).seek(from).map_err(|_| 131); // ERROR_NEGATIVE_SEEK
    if tracing() {
        let fd = std::os::fd::AsRawFd::as_raw_fd(&entry.file);
        eprintln!("[files] seek fd={fd} {from:?} -> {result:?}");
    }
    result
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
    buffer: usize,
    size: u32,
    read: *mut u32,
    available: *mut u32,
    left: *mut u32,
) -> i32 {
    if let Some(res) = unsafe { super::pipes::peek(handle, buffer, size, read, available, left) } {
        return res;
    }
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
/// Pipe I/O can pend and is cancelled by thread (CancelIo) or request (CancelIoEx).
/// Disk transfers finish immediately and have no outstanding request to cancel.
pub extern "win64" fn cancel_io(handle: usize) -> i32 {
    if let Some(result) = super::pipes::cancel_current(handle) {
        return result;
    }
    if get(handle).is_some() {
        1
    } else {
        fail(ERROR_INVALID_HANDLE, 0)
    }
}
pub extern "win64" fn cancel_io_ex(handle: usize, _overlapped: usize) -> i32 {
    if let Some(result) = super::pipes::cancel(handle, _overlapped) {
        return result;
    }
    if get(handle).is_some() {
        fail(1168, 0)
    } else {
        fail(ERROR_INVALID_HANDLE, 0)
    }
}
pub extern "win64" fn cancel_synchronous_io(_thread: usize) -> i32 {
    fail(1168, 0)
}
unsafe fn io_ex(
    handle: usize,
    buffer: *mut u8,
    n: u32,
    ov: *mut u8,
    routine: Option<crate::api::threading::apc::IoRoutine>,
    writing: bool,
) -> i32 {
    if ov.is_null() {
        return fail(87, 0);
    }
    let Some(entry) = get(handle) else {
        if super::pipes::known(handle) {
            let mut moved = 0u32;
            let ok = unsafe {
                super::pipes::io(handle, buffer, n, &mut moved, std::ptr::null_mut(), writing)
            }
            .unwrap_or(0);
            let err = if ok != 0 { 0 } else { error::get_last_error() };
            unsafe {
                (ov as *mut i64)
                    .write_unaligned(if err == 0 { 0 } else { 0xc000_0185u32 as i32 as i64 });
                (ov.add(8) as *mut u64).write_unaligned(moved as u64);
            }
            if let Some(routine) = routine {
                crate::api::threading::apc::queue_io(routine, err, moved, ov as usize);
            }
            return ok;
        }
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let offset = unsafe {
        (ov.add(16) as *const u32).read_unaligned() as u64
            | ((ov.add(20) as *const u32).read_unaligned() as u64) << 32
    };
    let result = unsafe {
        if writing {
            entry
                .file
                .write_at(std::slice::from_raw_parts(buffer, n as usize), offset)
        } else {
            entry
                .file
                .read_at(std::slice::from_raw_parts_mut(buffer, n as usize), offset)
        }
    };
    let (status, err, moved): (i64, u32, u32) = match &result {
        Ok(0) if !writing && n > 0 => (0xc000_0011u32 as i32 as i64, ERROR_HANDLE_EOF, 0),
        Ok(done) => (0, 0, *done as u32),
        Err(e) => (0xc000_0185u32 as i32 as i64, paths::error(e), 0),
    };
    unsafe {
        (ov as *mut i64).write_unaligned(status);
        (ov.add(8) as *mut u64).write_unaligned(moved as u64);
    }
    if let Some(routine) = routine {
        crate::api::threading::apc::queue_io(routine, err, moved, ov as usize);
    }
    if err != 0 && err != ERROR_HANDLE_EOF {
        return fail(err, 0);
    }
    error::set(0);
    1
}
pub unsafe extern "win64" fn read_ex(
    handle: usize,
    out: *mut u8,
    n: u32,
    ov: *mut u8,
    routine: Option<crate::api::threading::apc::IoRoutine>,
) -> i32 {
    unsafe { io_ex(handle, out, n, ov, routine, false) }
}
pub unsafe extern "win64" fn write_ex(
    handle: usize,
    data: *const u8,
    n: u32,
    ov: *mut u8,
    routine: Option<crate::api::threading::apc::IoRoutine>,
) -> i32 {
    unsafe { io_ex(handle, data as *mut u8, n, ov, routine, true) }
}
/// GetOverlappedResult: disk transfers finish immediately; pipe transfers may pend.
pub unsafe extern "win64" fn overlapped_result(
    handle: usize,
    ov: *const u8,
    transferred: *mut u32,
    _wait: i32,
) -> i32 {
    if super::pipes::known(handle) {
        return unsafe { super::pipes::result(handle, ov, transferred, _wait) };
    }
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
pub unsafe extern "win64" fn overlapped_result_ex(
    handle: usize,
    ov: *const u8,
    transferred: *mut u32,
    ms: u32,
    alertable: i32,
) -> i32 {
    if alertable != 0 && crate::api::threading::apc::run() {
        return fail(crate::api::threading::apc::WAIT_IO_COMPLETION, 0);
    }
    unsafe { overlapped_result(handle, ov, transferred, (ms != 0) as i32) }
}
pub unsafe extern "win64" fn final_path_name_by_handle_w(
    handle: usize,
    out: *mut u16,
    cch: u32,
    flags: u32,
) -> u32 {
    let Some(entry) = get(handle) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let Some(path) = entry.path.lock().unwrap().clone() else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let win = paths::windows(&path);
    let formatted = match flags & 0x7 {
        0x4 => win
            .strip_prefix("Z:")
            .or_else(|| win.strip_prefix("C:"))
            .unwrap_or(&win)
            .to_string(),
        _ => format!("\\\\?\\{win}"),
    };
    let wide: Vec<u16> = formatted.encode_utf16().collect();
    let needed = wide.len() as u32 + 1;
    if out.is_null() || cch < needed {
        error::set(122);
        return needed;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), out, wide.len());
        *out.add(wide.len()) = 0;
    }
    error::set(0);
    wide.len() as u32
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
/// GetFileTime: retrieves creation, last access, and last write times.
pub unsafe extern "win64" fn get_time(
    handle: usize,
    created: *mut u64,
    accessed: *mut u64,
    written: *mut u64,
) -> i32 {
    let Some(meta) = get(handle).and_then(|e| e.file.metadata().ok()) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    if !created.is_null() {
        unsafe { *created = filetime(meta.ctime(), meta.ctime_nsec()) };
    }
    if !accessed.is_null() {
        unsafe { *accessed = filetime(meta.atime(), meta.atime_nsec()) };
    }
    if !written.is_null() {
        unsafe { *written = filetime(meta.mtime(), meta.mtime_nsec()) };
    }
    1
}
/// A FILETIME for SetFileTime/FileBasicInfo: Ok(None) leaves the time unchanged
/// (0, and the special 0xFFFFFFFF_FFFFFFFF / 0xFFFFFFFF_FFFFFFFE values); times
/// before 1970 are valid. Err for a time the host cannot represent.
fn host_time(ticks: u64) -> Result<Option<std::time::SystemTime>, u32> {
    if ticks == 0 || ticks >= 0xFFFF_FFFF_FFFF_FFFE {
        return Ok(None);
    }
    let since_unix = ticks as i128 - EPOCH_1601 as i128 * 10_000_000;
    let span = std::time::Duration::from_nanos((since_unix.unsigned_abs() * 100) as u64);
    let time = if since_unix >= 0 {
        std::time::UNIX_EPOCH.checked_add(span)
    } else {
        std::time::UNIX_EPOCH.checked_sub(span)
    };
    time.map(Some).ok_or(87)
}
/// Sets access/write times and the read-only attribute through a handle (creation
/// and change times cannot be set on Linux; attributes 0 leaves them).
pub(super) fn set_basic(handle: usize, accessed: u64, written: u64, attributes: u32) -> Result<(), u32> {
    let entry = get(handle).ok_or(ERROR_INVALID_HANDLE)?;
    let mut times = std::fs::FileTimes::new();
    let mut any = false;
    if let Some(t) = host_time(accessed)? {
        times = times.set_accessed(t);
        any = true;
    }
    if let Some(t) = host_time(written)? {
        times = times.set_modified(t);
        any = true;
    }
    if any {
        entry.file.set_times(times).map_err(|e| paths::error(&e))?;
    }
    if attributes != 0 {
        let meta = entry.file.metadata().map_err(|e| paths::error(&e))?;
        let mut mode = meta.permissions().mode();
        mode = if attributes & FILE_ATTRIBUTE_READONLY != 0 { mode & !0o222 } else { mode | 0o200 };
        entry
            .file
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|e| paths::error(&e))?;
    }
    Ok(())
}
/// Delete disposition: the file goes when the last handle closes; `false` cancels.
pub(super) fn mark_delete(handle: usize, delete: bool) -> Result<(), u32> {
    let entry = get(handle).ok_or(ERROR_INVALID_HANDLE)?;
    let path = entry.path.lock().unwrap().clone().ok_or(ERROR_ACCESS_DENIED)?;
    if delete && path.is_dir() && std::fs::read_dir(&path).map_err(|e| paths::error(&e))?.next().is_some() {
        return Err(145); // ERROR_DIR_NOT_EMPTY
    }
    entry.delete_on_close.store(delete, Ordering::Release);
    Ok(())
}
/// Renames the file open on `handle` to the Windows path `name` (relative to the
/// directory handle `root` when given). The handle keeps working and follows it.
pub(super) fn rename_handle(handle: usize, root: usize, name: &str, replace: bool) -> Result<(), u32> {
    let entry = get(handle).ok_or(ERROR_INVALID_HANDLE)?;
    let from = entry.path.lock().unwrap().clone().ok_or(ERROR_ACCESS_DENIED)?;
    let name = name.strip_prefix(r"\??\").unwrap_or(name);
    let to = if root != 0 {
        let base = get(root).and_then(|r| r.path.lock().unwrap().clone()).ok_or(ERROR_INVALID_HANDLE)?;
        base.join(name.replace('\\', "/"))
    } else {
        paths::host(name)?
    };
    match rename_raw(&from, &to, replace) {
        Ok(()) => {
            *entry.path.lock().unwrap() = Some(to);
            Ok(())
        }
        Err(e) if e.raw_os_error() == Some(17) => Err(ERROR_ALREADY_EXISTS),
        Err(e) if e.raw_os_error() == Some(18) => Err(17), // ERROR_NOT_SAME_DEVICE
        Err(e) => Err(path_error(&to, &e)),
    }
}
/// FILE_RENAME_INFO / FILE_RENAME_INFORMATION: ReplaceIfExists (or Flags bit 1) @0,
/// RootDirectory @8, FileNameLength (bytes) @16, FileName @20.
pub(super) unsafe fn rename_from_info(handle: usize, info: *const u8, size: u32, flags_word: bool) -> Result<(), u32> {
    if size < 20 {
        return Err(24); // ERROR_BAD_LENGTH
    }
    let replace = if flags_word {
        (unsafe { (info as *const u32).read_unaligned() } & 1) != 0
    } else {
        (unsafe { *info }) != 0
    };
    let root = unsafe { (info.add(8) as *const usize).read_unaligned() };
    let bytes = unsafe { (info.add(16) as *const u32).read_unaligned() } as usize;
    if bytes == 0 || !bytes.is_multiple_of(2) || 20 + bytes > size as usize {
        return Err(87);
    }
    let units = unsafe { std::slice::from_raw_parts(info.add(20) as *const u16, bytes / 2) };
    let name = String::from_utf16(units).map_err(|_| 123u32)?; // ERROR_INVALID_NAME
    rename_handle(handle, root, &name, replace)
}
/// SetFileTime: access and write times (creation time cannot be set on Linux).
pub unsafe extern "win64" fn set_time(
    handle: usize,
    _created: *const u64,
    accessed: *const u64,
    written: *const u64,
) -> i32 {
    let read = |p: *const u64| if p.is_null() { 0 } else { unsafe { p.read_unaligned() } };
    match set_basic(handle, read(accessed), read(written), 0) {
        Ok(()) => 1,
        Err(code) => fail(code, 0),
    }
}
/// Byte-range locks as Linux open-file-description locks: like Windows locks they
/// belong to the open file, so two handles to one file conflict even in one process,
/// and duplicated handles share them. They are advisory (Windows' are mandatory: a
/// locked range is not enforced against plain reads and writes). A zero-length range
/// locks nothing, as on Windows.
fn lock_range(handle: usize, offset: u64, length: u64, kind: i16, wait: bool) -> Result<(), u32> {
    #[repr(C)]
    struct Flock {
        kind: i16,
        whence: i16,
        start: i64,
        length: i64,
        pid: i32,
    }
    unsafe extern "C" {
        fn fcntl(fd: i32, command: i32, ...) -> i32;
    }
    const F_OFD_SETLK: i32 = 37;
    const F_OFD_SETLKW: i32 = 38;
    let entry = get(handle).ok_or(ERROR_INVALID_HANDLE)?;
    if length == 0 {
        return Ok(());
    }
    let start = i64::try_from(offset).map_err(|_| 87u32)?;
    // A range reaching past i64::MAX locks to the end of all offsets (length 0).
    let length = i64::try_from(length).ok().filter(|l| start.checked_add(*l).is_some()).unwrap_or(0);
    let lock = Flock { kind, whence: 0, start, length, pid: 0 };
    let fd = std::os::fd::AsRawFd::as_raw_fd(&entry.file);
    loop {
        if unsafe { fcntl(fd, if wait { F_OFD_SETLKW } else { F_OFD_SETLK }, &lock) } == 0 {
            return Ok(());
        }
        match std::io::Error::last_os_error().raw_os_error() {
            Some(4) => continue,               // EINTR
            Some(11 | 13) => return Err(33),   // EAGAIN/EACCES: ERROR_LOCK_VIOLATION
            Some(35) => return Err(33),        // EDEADLK
            _ => return Err(ERROR_INVALID_HANDLE),
        }
    }
}
const F_RDLCK: i16 = 0;
const F_WRLCK: i16 = 1;
const F_UNLCK: i16 = 2;
fn join(low: u32, high: u32) -> u64 {
    low as u64 | (high as u64) << 32
}
/// The OVERLAPPED offset of a LockFileEx/UnlockFileEx call, which completes now.
unsafe fn lock_offset(ov: *mut u8) -> Option<u64> {
    (!ov.is_null()).then(|| unsafe {
        join((ov.add(16) as *const u32).read_unaligned(), (ov.add(20) as *const u32).read_unaligned())
    })
}
unsafe fn lock_done(ov: *mut u8, result: Result<(), u32>) -> i32 {
    match result {
        Ok(()) => {
            unsafe {
                (ov as *mut i64).write_unaligned(0);
                (ov.add(8) as *mut u64).write_unaligned(0);
                let event = (ov.add(24) as *const usize).read_unaligned();
                if event != 0 {
                    crate::api::threading::events::set(event & !1);
                }
            }
            1
        }
        Err(code) => fail(code, 0),
    }
}
/// LockFile: an exclusive lock that fails at once (ERROR_LOCK_VIOLATION) on conflict.
pub unsafe extern "win64" fn lock_file(handle: usize, offset_low: u32, offset_high: u32, count_low: u32, count_high: u32) -> i32 {
    match lock_range(handle, join(offset_low, offset_high), join(count_low, count_high), F_WRLCK, false) {
        Ok(()) => 1,
        Err(code) => fail(code, 0),
    }
}
/// LockFileEx: LOCKFILE_EXCLUSIVE_LOCK (2) or shared, LOCKFILE_FAIL_IMMEDIATELY (1)
/// or wait; the offset comes from the OVERLAPPED. Waiting blocks the caller.
pub unsafe extern "win64" fn lock_file_ex(handle: usize, flags: u32, reserved: u32, count_low: u32, count_high: u32, ov: *mut u8) -> i32 {
    if reserved != 0 || flags & !3 != 0 {
        return fail(87, 0);
    }
    let Some(offset) = (unsafe { lock_offset(ov) }) else {
        return fail(87, 0);
    };
    let kind = if flags & 2 != 0 { F_WRLCK } else { F_RDLCK };
    unsafe { lock_done(ov, lock_range(handle, offset, join(count_low, count_high), kind, flags & 1 == 0)) }
}
pub unsafe extern "win64" fn unlock_file(handle: usize, offset_low: u32, offset_high: u32, count_low: u32, count_high: u32) -> i32 {
    match lock_range(handle, join(offset_low, offset_high), join(count_low, count_high), F_UNLCK, false) {
        Ok(()) => 1,
        Err(code) => fail(code, 0),
    }
}
pub unsafe extern "win64" fn unlock_file_ex(handle: usize, reserved: u32, count_low: u32, count_high: u32, ov: *mut u8) -> i32 {
    if reserved != 0 {
        return fail(87, 0);
    }
    let Some(offset) = (unsafe { lock_offset(ov) }) else {
        return fail(87, 0);
    };
    unsafe { lock_done(ov, lock_range(handle, offset, join(count_low, count_high), F_UNLCK, false)) }
}
/// The host path behind a file handle (directory handles included).
pub(super) fn handle_path(handle: usize) -> Option<PathBuf> {
    get(handle)?.path.lock().unwrap().clone()
}

// ---- directory searches ---------------------------------------------------

struct Search {
    entries: std::vec::IntoIter<(String, std::fs::Metadata)>,
}
/// Windows wildcard match: `*` and `?`, case-insensitive; `*.*` also matches names
/// without an extension.
pub(super) fn matches(pattern: &str, name: &str) -> bool {
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
/// Convert WIN32_FIND_DATAW (592 bytes) into WIN32_FIND_DATAA (320 bytes).
unsafe fn wide_find_to_narrow(w: &[u8; 592], out: *mut u8) {
    let mut units = [0u16; 260];
    for (i, slot) in units.iter_mut().enumerate() {
        *slot = u16::from_le_bytes([w[44 + i * 2], w[45 + i * 2]]);
    }
    let len = units.iter().position(|&c| c == 0).unwrap_or(260);
    let name = String::from_utf16_lossy(&units[..len]);
    unsafe {
        out.write_bytes(0, 320);
        std::ptr::copy_nonoverlapping(w.as_ptr(), out, 44);
        text::copy_out_narrow(name.as_bytes(), out.add(44), 260);
    }
}
pub unsafe extern "win64" fn find_first_ex_a(
    pattern: *const c_char,
    level: i32,
    out: *mut u8,
    op: i32,
    filter: usize,
    flags: u32,
) -> usize {
    if out.is_null() {
        return fail(87, INVALID_HANDLE);
    }
    let Ok(s) = (unsafe { paths::narrow(pattern) }) else {
        return fail(87, INVALID_HANDLE);
    };
    let mut wide_pat = text::wide(&s);
    wide_pat.push(0);
    let mut w = [0u8; 592];
    let handle = unsafe {
        find_first_ex_w(wide_pat.as_ptr(), level, w.as_mut_ptr(), op, filter, flags)
    };
    if handle != INVALID_HANDLE {
        unsafe { wide_find_to_narrow(&w, out) };
    }
    handle
}
pub unsafe extern "win64" fn find_first_a(pattern: *const c_char, out: *mut u8) -> usize {
    unsafe { find_first_ex_a(pattern, 0, out, 0, 0, 0) }
}
pub unsafe extern "win64" fn find_next_a(handle: usize, out: *mut u8) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let mut w = [0u8; 592];
    let r = unsafe { find_next_w(handle, w.as_mut_ptr()) };
    if r != 0 {
        unsafe { wide_find_to_narrow(&w, out) };
    }
    r
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

// ---- directory change notifications (inotify-backed) ----------------------
unsafe extern "C" {
    fn inotify_init1(flags: i32) -> i32;
    fn inotify_add_watch(fd: i32, path: *const c_char, mask: u32) -> i32;
    #[link_name = "poll"]
    fn inotify_poll(fds: *mut PollFd, count: u64, timeout: i32) -> i32;
    #[link_name = "read"]
    fn inotify_read(fd: i32, buffer: *mut std::ffi::c_void, count: usize) -> isize;
    #[link_name = "close"]
    fn inotify_close(fd: i32) -> i32;
}
#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}
/// A watch: the inotify fd, and a flag the reader thread checks to exit.
struct ChangeWatch {
    inotify: i32,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
static WATCHES: std::sync::Mutex<Option<HashMap<usize, ChangeWatch>>> = std::sync::Mutex::new(None);
/// FILE_NOTIFY_CHANGE_* to an inotify mask.
fn inotify_mask(filter: u32) -> u32 {
    const IN_MODIFY: u32 = 0x2;
    const IN_ATTRIB: u32 = 0x4;
    const IN_CLOSE_WRITE: u32 = 0x8;
    const IN_MOVED_FROM: u32 = 0x40;
    const IN_MOVED_TO: u32 = 0x80;
    const IN_CREATE: u32 = 0x100;
    const IN_DELETE: u32 = 0x200;
    let names = IN_CREATE | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO;
    let mut mask = 0;
    if filter & 0x01 != 0 { mask |= names; } // FILE_NAME
    if filter & 0x02 != 0 { mask |= names; } // DIR_NAME
    if filter & 0x04 != 0 { mask |= IN_ATTRIB; } // ATTRIBUTES
    if filter & 0x08 != 0 { mask |= IN_MODIFY | IN_CLOSE_WRITE; } // SIZE
    if filter & 0x10 != 0 { mask |= IN_CLOSE_WRITE | IN_MODIFY; } // LAST_WRITE
    if filter & 0x40 != 0 { mask |= IN_CREATE; } // CREATION
    if filter & 0x100 != 0 { mask |= IN_ATTRIB; } // SECURITY
    if mask == 0 { names | IN_MODIFY } else { mask }
}
/// The shared body: watches `dir` with inotify and signals a manual-reset event when a
/// matching change happens. inotify is not recursive, so `watch_subtree` only covers
/// the named directory here. Returns INVALID_HANDLE on failure.
fn change_notification(dir: PathBuf, watch_subtree: i32, filter: u32) -> usize {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    const IN_NONBLOCK: i32 = 0x800;
    const IN_CLOEXEC: i32 = 0x8_0000;
    const POLLIN: i16 = 1;
    let Ok(c_dir) = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()) else {
        return fail(ERROR_PATH_NOT_FOUND, INVALID_HANDLE);
    };
    let inotify = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if inotify < 0 {
        return fail(crate::host::paths::error(&std::io::Error::last_os_error()), INVALID_HANDLE);
    }
    if unsafe { inotify_add_watch(inotify, c_dir.as_ptr(), inotify_mask(filter)) } < 0 {
        let code = crate::host::paths::error(&std::io::Error::last_os_error());
        unsafe { inotify_close(inotify) };
        return fail(code, INVALID_HANDLE);
    }
    let handle = crate::api::threading::events::create(true, false, None);
    if handle == 0 {
        unsafe { inotify_close(inotify) };
        return INVALID_HANDLE;
    }
    if tracing() {
        eprintln!("[files] watching {} (subtree={watch_subtree}, filter={filter:#x})", dir.display());
    }
    let stop = Arc::new(AtomicBool::new(false));
    WATCHES.lock().unwrap().get_or_insert_with(HashMap::new).insert(handle, ChangeWatch { inotify, stop: stop.clone() });
    // One reader thread per watch: poll the inotify fd in short slices so close() is
    // seen promptly, drain the events, and signal the handle on any change.
    std::thread::Builder::new().name("inotify-watch".into()).spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }
            let mut fds = PollFd { fd: inotify, events: POLLIN, revents: 0 };
            let ready = unsafe { inotify_poll(&mut fds, 1, 200) };
            if stop.load(Ordering::Acquire) {
                return;
            }
            if ready <= 0 || fds.revents & POLLIN == 0 {
                continue;
            }
            let n = unsafe { inotify_read(inotify, buffer.as_mut_ptr().cast(), buffer.len()) };
            if n > 0 {
                crate::api::threading::events::set(handle);
            }
        }
    }).ok();
    handle
}
pub unsafe extern "win64" fn find_first_change_notification_w(
    path: *const u16,
    watch_subtree: i32,
    filter: u32,
) -> usize {
    let text = match unsafe { crate::api::text::string(path) } {
        Some(text) => text,
        None => return fail(ERROR_PATH_NOT_FOUND, INVALID_HANDLE),
    };
    match crate::host::paths::host(&text) {
        Ok(dir) => change_notification(dir, watch_subtree, filter),
        Err(code) => fail(code, INVALID_HANDLE),
    }
}
pub unsafe extern "win64" fn find_first_change_notification_a(
    path: *const c_char,
    watch_subtree: i32,
    filter: u32,
) -> usize {
    match unsafe { narrow_path(path) } {
        Ok(dir) => change_notification(dir, watch_subtree, filter),
        Err(code) => fail(code, INVALID_HANDLE),
    }
}
/// Re-arms the watch: clears the signalled state so the next change signals again.
pub extern "win64" fn find_next_change_notification(handle: usize) -> i32 {
    if WATCHES.lock().unwrap().as_ref().is_some_and(|w| w.contains_key(&handle)) {
        crate::api::threading::events::reset(handle);
        1
    } else {
        fail(ERROR_INVALID_HANDLE, 0)
    }
}
pub extern "win64" fn find_close_change_notification(handle: usize) -> i32 {
    let watch = WATCHES.lock().unwrap().as_mut().and_then(|w| w.remove(&handle));
    match watch {
        Some(watch) => {
            watch.stop.store(true, std::sync::atomic::Ordering::Release);
            unsafe { inotify_close(watch.inotify) };
            crate::api::threading::events::close(handle);
            1
        }
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
/// renameat2: atomic, refusing an existing destination unless `replace`.
fn rename_raw(from: &Path, to: &Path, replace: bool) -> std::io::Result<()> {
    unsafe extern "C" {
        fn syscall(number: i64, ...) -> i64;
    }
    use std::os::unix::ffi::OsStrExt;
    let invalid = || std::io::Error::from_raw_os_error(22);
    let a = std::ffi::CString::new(from.as_os_str().as_bytes()).map_err(|_| invalid())?;
    let b = std::ffi::CString::new(to.as_os_str().as_bytes()).map_err(|_| invalid())?;
    const AT_FDCWD: i64 = -100;
    const RENAME_NOREPLACE: i64 = 1;
    let flags = if replace { 0 } else { RENAME_NOREPLACE };
    if unsafe { syscall(316, AT_FDCWD, a.as_ptr(), AT_FDCWD, b.as_ptr(), flags) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
fn rename(from: &Path, to: &Path, replace: bool) -> i32 {
    match rename_raw(from, to, replace) {
        Ok(()) => 1,
        Err(e) => fail(path_error(if from.exists() { to } else { from }, &e), 0),
    }
}
/// MoveFileW: atomic rename without replacing the destination, UTF-16 Windows paths.
/// Cross-filesystem copy/delete is not supported by this backend yet.
pub unsafe extern "win64" fn move_w(from: *const u16, to: *const u16) -> i32 {
    let parse = |p| unsafe { paths::wide(p) }.and_then(|s| paths::host(&s));
    match (parse(from), parse(to)) {
        (Ok(a), Ok(b)) => rename(&a, &b, false),
        (Err(code), _) | (_, Err(code)) => fail(code, 0),
    }
}
/// MoveFileExW: MOVEFILE_REPLACE_EXISTING (1), MOVEFILE_COPY_ALLOWED (2) and
/// MOVEFILE_WRITE_THROUGH (8). Like Windows, the destination folder must exist, and
/// only a move across filesystems may become copy + delete, with COPY_ALLOWED.
/// MOVEFILE_DELAY_UNTIL_REBOOT has no equivalent: ERROR_NOT_SUPPORTED.
pub unsafe extern "win64" fn move_ex_w(from: *const u16, to: *const u16, flags: u32) -> i32 {
    const REPLACE: u32 = 1;
    const COPY_ALLOWED: u32 = 2;
    const DELAY_UNTIL_REBOOT: u32 = 4;
    const WRITE_THROUGH: u32 = 8;
    if flags & DELAY_UNTIL_REBOOT != 0 {
        return fail(50, 0);
    }
    if to.is_null() || flags & !(REPLACE | COPY_ALLOWED | WRITE_THROUGH) != 0 {
        return fail(87, 0);
    }
    let replace = flags & REPLACE != 0;
    let (a, b) = match (unsafe { wide_path(from) }, unsafe { wide_path(to) }) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(code), _) | (_, Err(code)) => return fail(code, 0),
    };
    match rename_raw(&a, &b, replace) {
        Ok(()) => 1,
        Err(e) if e.raw_os_error() == Some(18) && flags & COPY_ALLOWED != 0 && a.is_file() => {
            // EXDEV: copy, then remove the source; a failed removal undoes the copy.
            if !replace && b.exists() {
                return fail(ERROR_FILE_EXISTS, 0);
            }
            if let Err(e) = std::fs::copy(&a, &b) {
                return fail(path_error(&b, &e), 0);
            }
            if let Err(e) = std::fs::remove_file(&a) {
                let _ = std::fs::remove_file(&b);
                return fail(path_error(&a, &e), 0);
            }
            1
        }
        Err(e) if e.raw_os_error() == Some(18) => fail(17, 0), // ERROR_NOT_SAME_DEVICE
        Err(e) => fail(path_error(if a.exists() { &b } else { &a }, &e), 0),
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
/// ANSI (code page = UTF-8 here) to a NUL-terminated wide string for the A->W wrappers.
unsafe fn ansi_to_wide(p: *const c_char) -> Option<Vec<u16>> {
    if p.is_null() {
        return None;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(p) }.to_str().ok()?;
    Some(text.encode_utf16().chain([0]).collect())
}
pub unsafe extern "win64" fn create_directory_a(name: *const c_char, security: usize) -> i32 {
    match unsafe { ansi_to_wide(name) } {
        Some(w) => unsafe { create_directory_w(w.as_ptr(), security) },
        None => fail(87, 0),
    }
}
pub unsafe extern "win64" fn delete_a(name: *const c_char) -> i32 {
    match unsafe { ansi_to_wide(name) } {
        Some(w) => unsafe { delete_w(w.as_ptr()) },
        None => fail(87, 0),
    }
}
pub unsafe extern "win64" fn remove_directory_a(name: *const c_char) -> i32 {
    match unsafe { ansi_to_wide(name) } {
        Some(w) => unsafe { remove_directory_w(w.as_ptr()) },
        None => fail(87, 0),
    }
}
pub unsafe extern "win64" fn attributes_ex_a(name: *const c_char, level: i32, out: *mut u8) -> i32 {
    match unsafe { ansi_to_wide(name) } {
        Some(w) => unsafe { attributes_ex_w(w.as_ptr(), level, out) },
        None => fail(87, 0),
    }
}
pub unsafe extern "win64" fn copy_a(from: *const c_char, to: *const c_char, fail_if_exists: i32) -> i32 {
    match (unsafe { ansi_to_wide(from) }, unsafe { ansi_to_wide(to) }) {
        (Some(a), Some(b)) => unsafe { copy_w(a.as_ptr(), b.as_ptr(), fail_if_exists) },
        _ => fail(87, 0),
    }
}
pub unsafe extern "win64" fn get_volume_information_a(
    root: *const c_char,
    volume_name: *mut u16,
    volume_name_size: u32,
    serial_number: *mut u32,
    max_component_len: *mut u32,
    flags: *mut u32,
    fs_name: *mut u16,
    fs_name_size: u32,
) -> i32 {
    // The strings this returns are ASCII (NTFS, the volume label is empty), so the
    // wide buffers double as narrow output: call W, then pack each down in place.
    let root_wide = unsafe { ansi_to_wide(root) };
    let result = unsafe {
        get_volume_information_w(
            root_wide.as_ref().map_or(std::ptr::null(), |w| w.as_ptr()),
            volume_name, volume_name_size, serial_number, max_component_len, flags, fs_name, fs_name_size,
        )
    };
    if result != 0 {
        for (buffer, size) in [(volume_name, volume_name_size), (fs_name, fs_name_size)] {
            if !buffer.is_null() && size > 0 {
                let wide = unsafe { text::units(buffer) }.to_vec();
                let mut narrow: Vec<u8> = wide.iter().map(|&u| u as u8).collect();
                narrow.push(0);
                unsafe { std::ptr::copy_nonoverlapping(narrow.as_ptr(), buffer.cast::<u8>(), narrow.len()) };
            }
        }
    }
    result
}
/// SearchPath: finds `file` (adding `extension` when it has none) in `path`, or, when
/// `path` is NULL, the EXE directory then the current directory. Returns the length
/// of the full Windows path (0 on failure), writing it when it fits.
pub unsafe extern "win64" fn search_path_w(
    path: *const u16,
    file: *const u16,
    extension: *const u16,
    buffer_len: u32,
    buffer: *mut u16,
    file_part: *mut *mut u16,
) -> u32 {
    if !file_part.is_null() {
        unsafe { *file_part = std::ptr::null_mut() };
    }
    let Some(mut name) = (unsafe { text::string(file) }) else {
        return fail(87, 0);
    };
    if let Some(ext) = unsafe { extension.as_ref().and_then(|_| text::string(extension)) } {
        if !name.contains('.') {
            name.push_str(&ext);
        }
    }
    let dirs: Vec<PathBuf> = match unsafe { text::string(path) }.filter(|p| !p.is_empty()) {
        Some(p) => p.split(';').filter(|d| !d.is_empty()).filter_map(|d| paths::host(d).ok()).collect(),
        None => {
            let mut v = Vec::new();
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    v.push(parent.to_path_buf());
                }
            }
            v.push(PathBuf::from("."));
            v
        }
    };
    for dir in dirs {
        let candidate = crate::host::paths::resolve_case(dir.join(&name));
        if candidate.is_file() {
            let text = crate::host::paths::windows(&candidate);
            let wide: Vec<u16> = text.encode_utf16().collect();
            if (wide.len() as u32) < buffer_len && !buffer.is_null() {
                unsafe {
                    std::ptr::copy_nonoverlapping(wide.as_ptr(), buffer, wide.len());
                    *buffer.add(wide.len()) = 0;
                }
                if !file_part.is_null() {
                    let slash = text.rfind('\\').map_or(0, |i| text[..i + 1].encode_utf16().count());
                    unsafe { *file_part = buffer.add(slash) };
                }
            }
            error::set(0);
            return wide.len() as u32;
        }
    }
    fail(ERROR_FILE_NOT_FOUND, 0)
}
pub unsafe extern "win64" fn search_path_a(
    path: *const c_char,
    file: *const c_char,
    extension: *const c_char,
    buffer_len: u32,
    buffer: *mut u8,
    file_part: *mut *mut u8,
) -> u32 {
    if !file_part.is_null() {
        unsafe { *file_part = std::ptr::null_mut() };
    }
    let (pw, fw, ew) = (unsafe { ansi_to_wide(path) }, unsafe { ansi_to_wide(file) }, unsafe { ansi_to_wide(extension) });
    let Some(fw) = fw else { return fail(87, 0) };
    let mut wide = vec![0u16; buffer_len as usize];
    let len = unsafe {
        search_path_w(
            pw.as_ref().map_or(std::ptr::null(), |w| w.as_ptr()),
            fw.as_ptr(),
            ew.as_ref().map_or(std::ptr::null(), |w| w.as_ptr()),
            buffer_len,
            wide.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    if len > 0 && len < buffer_len && !buffer.is_null() {
        let text = String::from_utf16_lossy(&wide[..len as usize]);
        let bytes = text.as_bytes();
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
            *buffer.add(bytes.len()) = 0;
            if !file_part.is_null() {
                let slash = text.rfind('\\').map_or(0, |i| i + 1);
                *file_part = buffer.add(slash);
            }
        }
    }
    len
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
/// GetDiskFreeSpaceW: sectors per cluster, bytes per sector, and cluster counts.
pub unsafe extern "win64" fn disk_free_space_w(
    name: *const u16,
    sectors_per_cluster: *mut u32,
    bytes_per_sector: *mut u32,
    free_clusters: *mut u32,
    total_clusters: *mut u32,
) -> i32 {
    let mut avail = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    if unsafe { disk_free_space_ex_w(name, &mut avail, &mut total, &mut free) } == 0 {
        return 0;
    }
    const SECTOR_SIZE: u32 = 512;
    const SECTORS_PER_CLUSTER: u32 = 8;
    let cluster_size = (SECTOR_SIZE as u64) * (SECTORS_PER_CLUSTER as u64);
    let total_c = (total / cluster_size).min(u32::MAX as u64) as u32;
    let free_c = (avail / cluster_size).min(u32::MAX as u64) as u32;
    unsafe {
        if !sectors_per_cluster.is_null() {
            *sectors_per_cluster = SECTORS_PER_CLUSTER;
        }
        if !bytes_per_sector.is_null() {
            *bytes_per_sector = SECTOR_SIZE;
        }
        if !free_clusters.is_null() {
            *free_clusters = free_c;
        }
        if !total_clusters.is_null() {
            *total_clusters = total_c;
        }
    }
    1
}
/// GetDiskFreeSpaceA: ANSI version of GetDiskFreeSpaceW.
pub unsafe extern "win64" fn disk_free_space_a(
    root: *const c_char,
    sectors_per_cluster: *mut u32,
    bytes_per_sector: *mut u32,
    free_clusters: *mut u32,
    total_clusters: *mut u32,
) -> i32 {
    if root.is_null() {
        return unsafe {
            disk_free_space_w(
                std::ptr::null(),
                sectors_per_cluster,
                bytes_per_sector,
                free_clusters,
                total_clusters,
            )
        };
    }
    let Ok(s) = (unsafe { paths::narrow(root) }) else {
        return fail(87, 0);
    };
    let mut wide = text::wide(&s);
    wide.push(0);
    unsafe {
        disk_free_space_w(
            wide.as_ptr(),
            sectors_per_cluster,
            bytes_per_sector,
            free_clusters,
            total_clusters,
        )
    }
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
/// GetDriveTypeA: ANSI version of GetDriveTypeW.
pub unsafe extern "win64" fn drive_type_a(root: *const c_char) -> u32 {
    if root.is_null() {
        return unsafe { drive_type_w(std::ptr::null()) };
    }
    let Ok(s) = (unsafe { paths::narrow(root) }) else {
        return 1; // DRIVE_NO_ROOT_DIR
    };
    let mut wide = text::wide(&s);
    wide.push(0);
    unsafe { drive_type_w(wide.as_ptr()) }
}

/// Completion association belongs to the file object, including duplicated handles.
pub(crate) fn associate_completion(
    handle: usize,
    binding: crate::api::threading::iocp::Binding,
) -> Result<(), u32> {
    if let Some(result) = super::pipes::associate(handle, binding.clone()) {
        return result;
    }
    let entry = get(handle).ok_or_else(|| {
        if crate::api::network::sockets::is_socket(handle) {
            50u32
        } else {
            6u32
        }
    })?;
    if !entry.overlapped {
        return Err(87);
    }
    let mut current = entry.completion.lock().unwrap();
    if let Some(previous) = current.as_ref() {
        return if previous.same_port(&binding) {
            Ok(())
        } else {
            Err(87)
        };
    }
    *current = Some(binding);
    Ok(())
}
pub(crate) fn set_completion_modes(handle: usize, modes: u8) -> Result<(), u32> {
    if let Some(result) = super::pipes::modes(handle, modes) {
        return result;
    }
    let entry = get(handle).ok_or(6u32)?;
    if !entry.overlapped {
        return Err(87);
    }
    entry.completion_modes.fetch_or(modes, Ordering::AcqRel);
    Ok(())
}

pub extern "win64" fn get_logical_drives() -> u32 {
    0x04
}

pub unsafe extern "win64" fn get_volume_information_w(
    _root: *const u16,
    volume_name: *mut u16,
    volume_name_size: u32,
    serial_number: *mut u32,
    max_component_len: *mut u32,
    flags: *mut u32,
    fs_name: *mut u16,
    fs_name_size: u32,
) -> i32 {
    if !volume_name.is_null() && volume_name_size > 0 {
        unsafe { *volume_name = 0 };
    }
    if !serial_number.is_null() {
        unsafe { *serial_number = 0x1234_5678 };
    }
    if !max_component_len.is_null() {
        unsafe { *max_component_len = 255 };
    }
    if !flags.is_null() {
        unsafe { *flags = 6 };
    }
    if !fs_name.is_null() && fs_name_size >= 5 {
        let ntfs: [u16; 5] = [b'N' as u16, b'T' as u16, b'F' as u16, b'S' as u16, 0];
        unsafe { std::ptr::copy_nonoverlapping(ntfs.as_ptr(), fs_name, 5) };
    }
    1
}

/// SetFileInformationByHandle: FileBasicInfo (0: access/write times, read-only),
/// FileRenameInfo (3) and FileRenameInfoEx (22), FileDispositionInfo (4) and
/// FileDispositionInfoEx (21: DELETE, POSIX semantics treated alike),
/// FileAllocationInfo (5, a hint), FileEndOfFileInfo (6). Others: ERROR_NOT_SUPPORTED.
pub unsafe extern "win64" fn set_file_information_by_handle(
    handle: usize,
    class: u32,
    info: *const std::ffi::c_void,
    size: u32,
) -> i32 {
    if info.is_null() {
        return fail(87, 0);
    }
    let Some(entry) = get(handle) else {
        return fail(ERROR_INVALID_HANDLE, 0);
    };
    let info = info as *const u8;
    let result = match class {
        0 if size >= 40 => unsafe {
            set_basic(
                handle,
                (info.add(8) as *const u64).read_unaligned(),
                (info.add(16) as *const u64).read_unaligned(),
                (info.add(32) as *const u32).read_unaligned(),
            )
        },
        3 => unsafe { rename_from_info(handle, info, size, false) },
        22 => unsafe { rename_from_info(handle, info, size, true) },
        4 if size >= 1 => mark_delete(handle, unsafe { *info } != 0),
        21 if size >= 4 => mark_delete(handle, unsafe { (info as *const u32).read_unaligned() } & 1 != 0),
        5 if size >= 8 => Ok(()),
        6 if size >= 8 => {
            let len = unsafe { (info as *const i64).read_unaligned() };
            if len < 0 {
                Err(87)
            } else {
                entry.file.set_len(len as u64).map_err(|e| paths::error(&e))
            }
        }
        0 | 4 | 5 | 6 | 21 => Err(24), // ERROR_BAD_LENGTH
        _ => Err(50),
    };
    match result {
        Ok(()) => 1,
        Err(code) => fail(code, 0),
    }
}

// ---- Windows 8 file entry points and links ---------------------------------------
/// CreateFile2: CreateFileW with CREATEFILE2_EXTENDED_PARAMETERS (x64: dwSize @0,
/// dwFileAttributes @4, dwFileFlags @8, dwSecurityQosFlags @12, lpSecurityAttributes
/// @16, hTemplateFile @24).
pub unsafe extern "win64" fn create_2(
    name: *const u16, access: u32, share: u32, disposition: u32, parameters: *const u8,
) -> usize {
    let (attributes, flags, security, template) = if parameters.is_null() {
        (0, 0, 0, 0)
    } else {
        unsafe {
            if parameters.cast::<u32>().read_unaligned() < 32 {
                return fail(87, usize::MAX);
            }
            (
                parameters.add(4).cast::<u32>().read_unaligned(),
                parameters.add(8).cast::<u32>().read_unaligned(),
                parameters.add(16).cast::<usize>().read_unaligned(),
                parameters.add(24).cast::<usize>().read_unaligned(),
            )
        }
    };
    unsafe { create_w(name, access, share, security, disposition, attributes | flags, template) }
}
/// CopyFile2: COPY_FILE_FAIL_IF_EXISTS only; progress routines, cancellation and other
/// copy flags return HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED). COPYFILE2_EXTENDED_PARAMETERS
/// (x64): dwSize @0, dwCopyFlags @4, pfCancel @8, pProgressRoutine @16, context @24.
pub unsafe extern "win64" fn copy_2(from: *const u16, to: *const u16, parameters: *const u8) -> i32 {
    const COPY_FILE_FAIL_IF_EXISTS: u32 = 1;
    let hresult = |code: u32| if code == 0 { 0 } else { (0x8007_0000 | (code & 0xffff)) as i32 };
    let mut flags = 0;
    if !parameters.is_null() {
        unsafe {
            if parameters.cast::<u32>().read_unaligned() < 32 {
                return hresult(87);
            }
            flags = parameters.add(4).cast::<u32>().read_unaligned();
            let cancel = parameters.add(8).cast::<usize>().read_unaligned();
            let progress = parameters.add(16).cast::<usize>().read_unaligned();
            if flags & !COPY_FILE_FAIL_IF_EXISTS != 0 || cancel != 0 || progress != 0 {
                eprintln!("CopyFile2: flags {flags:#x}, cancel or progress callbacks are not supported");
                error::set(50);
                return hresult(50);
            }
        }
    }
    if unsafe { copy_w(from, to, (flags & COPY_FILE_FAIL_IF_EXISTS != 0) as i32) } != 0 {
        return 0;
    }
    hresult(crate::api::system::error::get_last_error())
}
/// CreateSymbolicLinkW (file or directory; unprivileged creation is the default here).
/// Relative targets stay relative to the link's directory.
pub unsafe extern "win64" fn create_symbolic_link_w(link: *const u16, target: *const u16, flags: u32) -> u8 {
    const KNOWN: u32 = 1 | 2; // SYMBOLIC_LINK_FLAG_DIRECTORY, _ALLOW_UNPRIVILEGED_CREATE
    if flags & !KNOWN != 0 {
        return fail(87, 0);
    }
    let Some(target_text) = (unsafe { text::string(target) }) else {
        return fail(87, 0);
    };
    let absolute = target_text.as_bytes().get(1) == Some(&b':') || target_text.starts_with(['\\', '/']);
    let target_path = if absolute {
        match paths::host(&target_text) {
            Ok(path) => path,
            Err(code) => return fail(code, 0),
        }
    } else {
        PathBuf::from(target_text.replace('\\', "/"))
    };
    let link_path = match unsafe { wide_path(link) } {
        Ok(path) => path,
        Err(code) => return fail(code, 0),
    };
    match std::os::unix::fs::symlink(&target_path, &link_path) {
        Ok(()) => 1,
        Err(e) => fail(path_error(&link_path, &e), 0),
    }
}
pub unsafe extern "win64" fn create_hard_link_w(new: *const u16, existing: *const u16, _security: usize) -> i32 {
    match (unsafe { wide_path(new) }, unsafe { wide_path(existing) }) {
        (Ok(new), Ok(existing)) => result(&new, std::fs::hard_link(&existing, &new)),
        (Err(code), _) | (_, Err(code)) => fail(code, 0),
    }
}
