//! Synchronous NT reads and writes at the current file position. No APC/IOCP submission.
use crate::api::system::error;
use std::ffi::c_void;

#[repr(C)]
pub struct IoStatus {
    pub status: usize,
    pub information: usize,
}

/// Win32 error -> NTSTATUS (the reverse of RtlNtStatusToDosError for the codes
/// the file layer produces); anything else is STATUS_IO_DEVICE_ERROR.
fn status(code: u32) -> u32 {
    match code {
        0 => 0,
        2 => 0xc0000034,         // STATUS_OBJECT_NAME_NOT_FOUND
        3 => 0xc000003a,         // STATUS_OBJECT_PATH_NOT_FOUND
        5 => 0xc0000022,         // STATUS_ACCESS_DENIED
        6 => 0xc0000008,         // STATUS_INVALID_HANDLE
        17 => 0xc00000d4,        // STATUS_NOT_SAME_DEVICE
        24 => 0xc0000004,        // STATUS_INFO_LENGTH_MISMATCH
        32 => 0xc0000043,        // STATUS_SHARING_VIOLATION
        38 => 0xc0000011,        // STATUS_END_OF_FILE
        50 => 0xc00000bb,        // STATUS_NOT_SUPPORTED
        80 | 183 => 0xc0000035,  // STATUS_OBJECT_NAME_COLLISION
        87 => 0xc000000d,        // STATUS_INVALID_PARAMETER
        109 | 232 => 0xc00000b1, // STATUS_PIPE_CLOSING
        112 => 0xc000007f,       // STATUS_DISK_FULL
        123 => 0xc0000033,       // STATUS_OBJECT_NAME_INVALID
        145 => 0xc0000101,       // STATUS_DIRECTORY_NOT_EMPTY
        _ => 0xc0000185,
    }
}

pub unsafe extern "win64" fn write(
    file: usize, event: usize, apc: usize, context: usize,
    io: *mut IoStatus, buffer: *const c_void, length: u32,
    offset: *const i64, key: *const u32,
) -> i32 {
    if io.is_null() { return 0xc000000du32 as i32; }
    let mut bytes: u32 = 0;
    let saved = error::get_last_error();
    let result = if buffer.is_null() && length != 0 {
        0xc000000d
    } else if event != 0 || apc != 0 || context != 0 || !key.is_null() {
        0xc00000bb
    } else if !offset.is_null() && unsafe { offset.read_unaligned() } >= 0 {
        let off = unsafe { offset.read_unaligned() } as u64;
        if let Some(entry) = super::files::get(file) {
            use std::os::unix::fs::FileExt;
            let slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, length as usize) };
            match entry.file.write_at(slice, off) {
                Ok(got) => {
                    bytes = got as u32;
                    0
                }
                Err(e) => status(crate::host::paths::error(&e)),
            }
        } else {
            0xc0000008
        }
    } else if !offset.is_null() && unsafe { offset.read_unaligned() } != -2 {
        0xc000000d
    } else if unsafe { super::write::write_file(file, buffer, length, &mut bytes, std::ptr::null_mut()) } != 0 {
        0
    } else {
        status(error::get_last_error())
    };
    error::set(saved);
    unsafe { io.write(IoStatus { status: result as usize, information: if result == 0 { bytes as usize } else { 0 } }); }
    result as i32
}

pub unsafe extern "win64" fn read(
    file: usize, event: usize, apc: usize, context: usize,
    io: *mut IoStatus, buffer: *mut c_void, length: u32,
    offset: *const i64, key: *const u32,
) -> i32 {
    if io.is_null() { return 0xc000000du32 as i32; }
    let mut bytes: u32 = 0;
    let saved = error::get_last_error();
    let result = if buffer.is_null() && length != 0 {
        0xc000000d
    } else if event != 0 || apc != 0 || context != 0 || !key.is_null() {
        0xc00000bb
    } else if !offset.is_null() && unsafe { offset.read_unaligned() } >= 0 {
        let off = unsafe { offset.read_unaligned() } as u64;
        if let Some(entry) = super::files::get(file) {
            use std::os::unix::fs::FileExt;
            let slice = unsafe { std::slice::from_raw_parts_mut(buffer as *mut u8, length as usize) };
            match entry.file.read_at(slice, off) {
                Ok(0) if length > 0 => 0xc0000011,
                Ok(got) => {
                    bytes = got as u32;
                    0
                }
                Err(e) => status(crate::host::paths::error(&e)),
            }
        } else {
            0xc0000008
        }
    } else if !offset.is_null() && unsafe { offset.read_unaligned() } != -2 {
        0xc000000d
    } else if unsafe { super::files::read(file, buffer as *mut u8, length, &mut bytes, std::ptr::null_mut()) } != 0 {
        0
    } else {
        status(error::get_last_error())
    };
    error::set(saved);
    unsafe { io.write(IoStatus { status: result as usize, information: if result == 0 { bytes as usize } else { 0 } }); }
    result as i32
}

#[repr(C)]
pub struct UnicodeString {
    pub length: u16,
    pub maximum_length: u16,
    pub buffer: *const u16,
}

#[repr(C)]
pub struct ObjectAttributes {
    pub length: u32,
    pub root_directory: usize,
    pub object_name: *const UnicodeString,
    pub attributes: u32,
    pub security_descriptor: *const c_void,
    pub security_quality_of_service: *const c_void,
}

pub unsafe extern "win64" fn create(
    file_handle: *mut usize,
    access: u32,
    attr: *const ObjectAttributes,
    io_status: *mut IoStatus,
    _alloc_size: *const i64,
    file_attrs: u32,
    share_access: u32,
    disposition: u32,
    _options: u32,
    _ea: *const c_void,
    _ea_len: u32,
) -> i32 {
    if file_handle.is_null() || io_status.is_null() || attr.is_null() {
        return 0xC000000Du32 as i32;
    }
    let obj_name = unsafe { (*attr).object_name };
    if obj_name.is_null() {
        return 0xC000000Du32 as i32;
    }
    let ustr = unsafe { &*obj_name };
    if ustr.buffer.is_null() {
        return 0xC000000Du32 as i32;
    }
    let units_len = (ustr.length / 2) as usize;
    let slice = unsafe { std::slice::from_raw_parts(ustr.buffer, units_len) };
    let mut path: Vec<u16> = slice.to_vec();
    if path.starts_with(&[b'\\' as u16, b'?' as u16, b'?' as u16, b'\\' as u16])
        || path.starts_with(&[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16])
    {
        path.drain(0..4);
    }
    path.push(0);

    let name_str = String::from_utf16_lossy(&path);
    let lower_name = name_str.to_ascii_lowercase();
    if lower_name.starts_with(r"\device\afd") || lower_name.starts_with(r"device\afd") {
        let handle = crate::api::network::afd::create();
        unsafe {
            *file_handle = handle;
            io_status.write(IoStatus {
                status: 0,
                information: 1, // FILE_OPENED
            });
        }
        return 0;
    }

    // FILE_SUPERSEDE 0, FILE_OPEN 1, FILE_CREATE 2, FILE_OPEN_IF 3, FILE_OVERWRITE 4,
    // FILE_OVERWRITE_IF 5 as CreateFile dispositions.
    let win32_disp = match disposition {
        0 | 5 => 2,
        1 => 3,
        2 => 1,
        3 => 4,
        4 => 5,
        _ => return 0xC000000Du32 as i32,
    };
    const FILE_DIRECTORY_FILE: u32 = 0x1;
    const FILE_NON_DIRECTORY_FILE: u32 = 0x40;
    const FILE_DELETE_ON_CLOSE: u32 = 0x1000;
    // RootDirectory: the name is relative to that directory handle.
    let root = unsafe { (*attr).root_directory };
    let text = String::from_utf16_lossy(&path[..path.len() - 1]);
    let host = if root != 0 {
        let Some(base) = super::files::handle_path(root) else {
            return 0xC0000008u32 as i32; // STATUS_INVALID_HANDLE
        };
        crate::host::paths::resolve_case(base.join(text.replace('\\', "/")))
    } else {
        match crate::host::paths::host(&text) {
            Ok(p) => p,
            Err(code) => return status(code) as i32,
        }
    };
    let existed = host.symlink_metadata().ok();
    if _options & FILE_DIRECTORY_FILE != 0 && existed.as_ref().is_some_and(|m| !m.is_dir()) {
        return 0xC0000103u32 as i32; // STATUS_NOT_A_DIRECTORY
    }
    if _options & FILE_NON_DIRECTORY_FILE != 0 && existed.as_ref().is_some_and(|m| m.is_dir()) {
        return 0xC00000BAu32 as i32; // STATUS_FILE_IS_A_DIRECTORY
    }
    let mut flags = file_attrs & 0xffff;
    if _options & FILE_DIRECTORY_FILE != 0 {
        flags |= 0x0200_0000; // FILE_FLAG_BACKUP_SEMANTICS
    }
    if _options & FILE_DELETE_ON_CLOSE != 0 {
        flags |= 0x0400_0000;
    }
    let host_name: Vec<u16> = crate::host::paths::windows(&host).encode_utf16().chain([0]).collect();
    let handle = unsafe {
        super::files::create_w(
            host_name.as_ptr(),
            access,
            share_access,
            0,
            win32_disp,
            flags,
            0,
        )
    };

    if handle != usize::MAX {
        // FILE_SUPERSEDED 0, FILE_OPENED 1, FILE_CREATED 2, FILE_OVERWRITTEN 3.
        let information = match (existed.is_some(), disposition) {
            (false, _) => 2,
            (true, 0) => 0,
            (true, 4 | 5) => 3,
            (true, _) => 1,
        };
        unsafe {
            *file_handle = handle;
            io_status.write(IoStatus {
                status: 0,
                information,
            });
        }
        0
    } else {
        let code = error::get_last_error();
        let s = status(code);
        unsafe {
            io_status.write(IoStatus {
                status: s as usize,
                information: 0,
            });
        }
        s as i32
    }
}

pub unsafe extern "win64" fn open(
    file_handle: *mut usize,
    access: u32,
    attr: *const ObjectAttributes,
    io_status: *mut IoStatus,
    share_access: u32,
    options: u32,
) -> i32 {
    unsafe {
        create(
            file_handle,
            access,
            attr,
            io_status,
            std::ptr::null(),
            0x80, // FILE_ATTRIBUTE_NORMAL
            share_access,
            1,    // FILE_OPEN (OPEN_EXISTING)
            options,
            std::ptr::null(),
            0,
        )
    }
}

pub unsafe extern "win64" fn cancel_io_file_ex(
    file_handle: usize,
    target_iosb: *const IoStatus,
    result_iosb: *mut IoStatus,
) -> i32 {
    if crate::api::network::afd::is_afd(file_handle) {
        unsafe { crate::api::network::afd::cancel_io(file_handle, target_iosb, result_iosb) }
    } else {
        let s = if super::files::get(file_handle).is_some() {
            0xC0000225u32 as i32 // STATUS_NOT_FOUND
        } else {
            0xC0000008u32 as i32 // STATUS_INVALID_HANDLE
        };
        if !result_iosb.is_null() {
            unsafe {
                result_iosb.write(IoStatus {
                    status: s as usize,
                    information: 0,
                });
            }
        }
        s
    }
}

pub unsafe extern "win64" fn device_io_control_file(
    file: usize,
    event: usize,
    apc: usize,
    context: usize,
    io_status: *mut IoStatus,
    code: u32,
    input: *mut c_void,
    input_len: u32,
    output: *mut c_void,
    output_len: u32,
) -> i32 {
    if crate::api::network::afd::is_afd(file) {
        unsafe {
            crate::api::network::afd::device_io_control(
                file, event, apc, context, io_status, code, input, input_len, output, output_len,
            )
        }
    } else {
        0xC0000008u32 as i32 // STATUS_INVALID_HANDLE
    }
}

/// Query the volume associated with an open file. Unsupported information
/// classes fail explicitly so callers can use their documented fallback.
pub unsafe extern "win64" fn query_volume_information_file(
    handle: usize,
    io: *mut IoStatus,
    buffer: *mut c_void,
    length: u32,
    class: i32,
) -> i32 {
    const STATUS_INVALID_PARAMETER: u32 = 0xC000000D;
    const STATUS_INVALID_HANDLE: u32 = 0xC0000008;
    const STATUS_INFO_LENGTH_MISMATCH: u32 = 0xC0000004;
    const STATUS_INVALID_INFO_CLASS: u32 = 0xC0000003;
    if io.is_null() || buffer.is_null() {
        return STATUS_INVALID_PARAMETER as i32;
    }
    let (status, written) = if super::files::get(handle).is_none() {
        (STATUS_INVALID_HANDLE, 0)
    } else if class == 1 {
        // FileFsVolumeInformation
        if length < 18 {
            (STATUS_INFO_LENGTH_MISMATCH, 0)
        } else {
            let label = "WINDOWS\0".encode_utf16().collect::<Vec<_>>();
            let label_bytes = (label.len() - 1) * 2;
            let mut info = vec![0u8; 18 + label_bytes];
            info[0..8].copy_from_slice(&132_000_000_000_000_000u64.to_le_bytes());
            info[8..12].copy_from_slice(&0x1234_5678u32.to_le_bytes());
            info[12..16].copy_from_slice(&(label_bytes as u32).to_le_bytes());
            info[16] = 1;
            let copy_bytes = (length as usize - 18).min(label_bytes);
            if copy_bytes > 0 {
                let src_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(label.as_ptr() as *const u8, copy_bytes)
                };
                info[18..18 + copy_bytes].copy_from_slice(src_bytes);
            }
            let written = 18 + copy_bytes;
            unsafe { std::ptr::copy_nonoverlapping(info.as_ptr(), buffer.cast::<u8>(), written) };
            (0, written)
        }
    } else if class == 3 {
        // FileFsSizeInformation: TotalAllocationUnits, AvailableAllocationUnits, SectorsPerAllocationUnit, BytesPerSector
        if length < 24 {
            (STATUS_INFO_LENGTH_MISMATCH, 0)
        } else {
            let mut info = [0u8; 24];
            info[0..8].copy_from_slice(&100_000_000u64.to_le_bytes()); // ~400 GB
            info[8..16].copy_from_slice(&50_000_000u64.to_le_bytes());  // ~200 GB
            info[16..20].copy_from_slice(&8u32.to_le_bytes());          // 8 sectors
            info[20..24].copy_from_slice(&512u32.to_le_bytes());        // 512 bytes = 4KB clusters
            unsafe { std::ptr::copy_nonoverlapping(info.as_ptr(), buffer.cast::<u8>(), 24) };
            (0, 24)
        }
    } else if class == 4 {
        // FileFsDeviceInformation: DEVICE_TYPE and Characteristics.
        if length < 8 {
            (STATUS_INFO_LENGTH_MISMATCH, 0)
        } else {
            let mut info = [0u8; 8];
            info[..4].copy_from_slice(&7u32.to_le_bytes()); // FILE_DEVICE_DISK
            info[4..].copy_from_slice(&0x20u32.to_le_bytes()); // FILE_DEVICE_IS_MOUNTED
            unsafe { std::ptr::copy_nonoverlapping(info.as_ptr(), buffer.cast::<u8>(), info.len()) };
            (0, info.len())
        }
    } else if class == 7 {
        // FileFsAttributeInformation: FileSystemAttributes, MaximumComponentNameLength, FileSystemNameLength, FileSystemName
        if length < 12 {
            (STATUS_INFO_LENGTH_MISMATCH, 0)
        } else {
            let fs_name = "NTFS\0".encode_utf16().collect::<Vec<_>>();
            let fs_bytes = (fs_name.len() - 1) * 2;
            let mut info = vec![0u8; 12 + fs_bytes];
            info[0..4].copy_from_slice(&0x0000_0003u32.to_le_bytes()); // FILE_CASE_SENSITIVE_SEARCH | FILE_CASE_PRESERVED_NAMES
            info[4..8].copy_from_slice(&255u32.to_le_bytes());           // MaximumComponentNameLength
            info[8..12].copy_from_slice(&(fs_bytes as u32).to_le_bytes());
            let copy_bytes = (length as usize - 12).min(fs_bytes);
            if copy_bytes > 0 {
                let src_bytes: &[u8] = unsafe {
                    std::slice::from_raw_parts(fs_name.as_ptr() as *const u8, copy_bytes)
                };
                info[12..12 + copy_bytes].copy_from_slice(src_bytes);
            }
            let written = 12 + copy_bytes;
            unsafe { std::ptr::copy_nonoverlapping(info.as_ptr(), buffer.cast::<u8>(), written) };
            (0, written)
        }
    } else {
        eprintln!("NtQueryVolumeInformationFile: unsupported class {class}");
        (STATUS_INVALID_INFO_CLASS, 0)
    };
    unsafe { io.write(IoStatus { status: status as usize, information: written }) };
    status as i32
}

pub unsafe extern "win64" fn query_information_file(
    handle: usize,
    io: *mut IoStatus,
    buffer: *mut c_void,
    length: u32,
    class: i32,
) -> i32 {
    use std::io::Seek;
    use std::os::unix::fs::MetadataExt;

    if io.is_null() || buffer.is_null() {
        return 0xC000000Du32 as i32; // STATUS_INVALID_PARAMETER
    }
    let Some(meta) = super::files::get(handle).and_then(|e| e.file.metadata().ok()) else {
        return 0xC0000008u32 as i32; // STATUS_INVALID_HANDLE
    };
    let (status, bytes) = match class {
        // FileBasicInformation (4): 40 bytes
        4 => {
            if length < 40 {
                (0xC0000004u32, 0) // STATUS_INFO_LENGTH_MISMATCH
            } else {
                let attr = super::files::attributes(&meta, None);
                let (c, a, w) = (
                    super::files::filetime(meta.ctime(), meta.ctime_nsec()),
                    super::files::filetime(meta.atime(), meta.atime_nsec()),
                    super::files::filetime(meta.mtime(), meta.mtime_nsec()),
                );
                let mut d = [0u8; 40];
                for (i, v) in [c, a, w, c].iter().enumerate() {
                    d[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                }
                d[32..36].copy_from_slice(&attr.to_le_bytes());
                unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), buffer as *mut u8, 40) };
                (0, 40)
            }
        }
        // FileStandardInformation (5): 24 bytes
        5 => {
            if length < 24 {
                (0xC0000004u32, 0)
            } else {
                let mut d = [0u8; 24];
                d[0..8].copy_from_slice(&(meta.blocks() * 512).to_le_bytes()); // AllocationSize
                d[8..16].copy_from_slice(&(if meta.is_dir() { 0 } else { meta.len() }).to_le_bytes()); // EndOfFile
                d[16..20].copy_from_slice(&(meta.nlink() as u32).to_le_bytes()); // NumberOfLinks
                d[20] = 0; // DeletePending
                d[21] = meta.is_dir() as u8; // Directory
                unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), buffer as *mut u8, 24) };
                (0, 24)
            }
        }
        // FileInternalInformation (6): 8 bytes (IndexNumber)
        6 => {
            if length < 8 {
                (0xC0000004u32, 0)
            } else {
                let ino = meta.ino();
                unsafe { std::ptr::copy_nonoverlapping(ino.to_le_bytes().as_ptr(), buffer as *mut u8, 8) };
                (0, 8)
            }
        }
        // FilePositionInformation (14): 8 bytes
        14 => {
            if length < 8 {
                (0xC0000004u32, 0)
            } else {
                let pos = super::files::get(handle)
                    .and_then(|e| (&e.file).stream_position().ok())
                    .unwrap_or(0);
                unsafe { std::ptr::copy_nonoverlapping(pos.to_le_bytes().as_ptr(), buffer as *mut u8, 8) };
                (0, 8)
            }
        }
        // FileNetworkOpenInformation (34): 56 bytes
        34 => {
            if length < 56 {
                (0xC0000004u32, 0)
            } else {
                let attr = super::files::attributes(&meta, None);
                let (c, a, w) = (
                    super::files::filetime(meta.ctime(), meta.ctime_nsec()),
                    super::files::filetime(meta.atime(), meta.atime_nsec()),
                    super::files::filetime(meta.mtime(), meta.mtime_nsec()),
                );
                let mut d = [0u8; 56];
                for (i, v) in [c, a, w, c].iter().enumerate() {
                    d[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                }
                d[32..40].copy_from_slice(&(meta.blocks() * 512).to_le_bytes()); // AllocationSize
                d[40..48].copy_from_slice(&(if meta.is_dir() { 0 } else { meta.len() }).to_le_bytes()); // EndOfFile
                d[48..52].copy_from_slice(&attr.to_le_bytes());
                unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), buffer as *mut u8, 56) };
                (0, 56)
            }
        }
        // FileAllInformation (18): 100 bytes minimum
        18 => {
            if length < 100 {
                (0xC0000004u32, 0) // STATUS_INFO_LENGTH_MISMATCH
            } else {
                let attr = super::files::attributes(&meta, None);
                let (c, a, w) = (
                    super::files::filetime(meta.ctime(), meta.ctime_nsec()),
                    super::files::filetime(meta.atime(), meta.atime_nsec()),
                    super::files::filetime(meta.mtime(), meta.mtime_nsec()),
                );
                let pos = super::files::get(handle)
                    .and_then(|e| (&e.file).stream_position().ok())
                    .unwrap_or(0);
                let mut d = vec![0u8; length as usize];
                // BasicInformation: 40 bytes at offset 0
                for (i, v) in [c, a, w, c].iter().enumerate() {
                    d[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
                }
                d[32..36].copy_from_slice(&attr.to_le_bytes());
                // StandardInformation: 24 bytes at offset 40
                d[40..48].copy_from_slice(&(meta.blocks() * 512).to_le_bytes()); // AllocationSize
                d[48..56].copy_from_slice(&(if meta.is_dir() { 0 } else { meta.len() }).to_le_bytes()); // EndOfFile
                d[56..60].copy_from_slice(&(meta.nlink() as u32).to_le_bytes()); // NumberOfLinks
                d[60] = 0; // DeletePending
                d[61] = meta.is_dir() as u8; // Directory
                // InternalInformation: 8 bytes at offset 64
                d[64..72].copy_from_slice(&meta.ino().to_le_bytes());
                // EaInformation: 4 bytes at offset 72 (ea_size = 0)
                // AccessInformation: 4 bytes at offset 76 (FILE_ALL_ACCESS)
                d[76..80].copy_from_slice(&0x001f_01ffu32.to_le_bytes());
                // PositionInformation: 8 bytes at offset 80
                d[80..88].copy_from_slice(&pos.to_le_bytes());
                // ModeInformation: 4 bytes at offset 88 (0)
                // AlignmentInformation: 4 bytes at offset 92 (0)
                // NameInformation: offset 96
                let win_path = super::files::handle_path(handle)
                    .map(|p| crate::host::paths::windows(&p))
                    .unwrap_or_default();
                let name_u16: Vec<u16> = win_path.encode_utf16().collect();
                let name_bytes = name_u16.len() * 2;
                d[96..100].copy_from_slice(&(name_bytes as u32).to_le_bytes());
                let max_name_copy = (length as usize - 100).min(name_bytes);
                if max_name_copy > 0 {
                    let src_bytes: &[u8] = unsafe {
                        std::slice::from_raw_parts(name_u16.as_ptr() as *const u8, max_name_copy)
                    };
                    d[100..100 + max_name_copy].copy_from_slice(src_bytes);
                }
                let total = 100 + max_name_copy;
                unsafe { std::ptr::copy_nonoverlapping(d.as_ptr(), buffer as *mut u8, total) };
                (0, total as u32)
            }
        }
        _ => {
            eprintln!("NtQueryInformationFile: unsupported class {class}");
            (0xC0000003u32, 0) // STATUS_INVALID_INFO_CLASS
        }
    };
    unsafe {
        io.write(IoStatus {
            status: status as usize,
            information: bytes as usize,
        });
    }
    status as i32
}

/// NtSetInformationFile: FileBasicInformation (4), FileRenameInformation (10) and
/// Ex (65), FileDispositionInformation (13) and Ex (64), FilePositionInformation
/// (14), FileAllocationInformation (19, a hint), FileEndOfFileInformation (20).
/// Any other class is STATUS_INVALID_INFO_CLASS: nothing is reported done that was not.
pub unsafe extern "win64" fn set_information_file(
    handle: usize,
    io: *mut IoStatus,
    buffer: *const c_void,
    length: u32,
    class: i32,
) -> i32 {
    use std::io::Seek;
    const LENGTH_MISMATCH: u32 = 0xC0000004;
    const INVALID_PARAMETER: u32 = 0xC000000D;
    if io.is_null() || buffer.is_null() {
        return INVALID_PARAMETER as i32;
    }
    let Some(entry) = super::files::get(handle) else {
        return 0xC0000008u32 as i32; // STATUS_INVALID_HANDLE
    };
    let b = buffer as *const u8;
    let done = |r: Result<(), u32>| r.map_or_else(status, |()| 0);
    let status = match class {
        4 if length >= 40 => done(super::files::set_basic(
            handle,
            unsafe { (b.add(8) as *const u64).read_unaligned() },
            unsafe { (b.add(16) as *const u64).read_unaligned() },
            unsafe { (b.add(32) as *const u32).read_unaligned() },
        )),
        10 => done(unsafe { super::files::rename_from_info(handle, b, length, false) }),
        65 => done(unsafe { super::files::rename_from_info(handle, b, length, true) }),
        13 if length >= 1 => done(super::files::mark_delete(handle, unsafe { *b } != 0)),
        64 if length >= 4 => {
            done(super::files::mark_delete(handle, unsafe { (b as *const u32).read_unaligned() } & 1 != 0))
        }
        14 if length >= 8 => {
            let pos = unsafe { (b as *const i64).read_unaligned() };
            if pos < 0 {
                INVALID_PARAMETER
            } else {
                done((&entry.file).seek(std::io::SeekFrom::Start(pos as u64)).map(|_| ()).map_err(|_| 87))
            }
        }
        19 if length >= 8 => 0,
        20 if length >= 8 => {
            let len = unsafe { (b as *const i64).read_unaligned() };
            if len < 0 {
                INVALID_PARAMETER
            } else {
                done(entry.file.set_len(len as u64).map_err(|_| 112))
            }
        }
        4 | 13 | 14 | 19 | 20 | 64 => LENGTH_MISMATCH,
        _ => {
            eprintln!("NtSetInformationFile: unsupported class {class}");
            0xC0000003 // STATUS_INVALID_INFO_CLASS
        }
    };
    unsafe {
        io.write(IoStatus {
            status: status as usize,
            information: 0,
        });
    }
    status as i32
}
