//! Synchronous NT directory enumeration used by .NET FileSystemEnumerator.
use super::files;
use std::{
    fs::{Metadata, ReadDir},
    os::unix::{fs::MetadataExt, io::AsRawFd},
    path::PathBuf,
};
pub(super) struct Scan {
    base: PathBuf,
    entries: ReadDir,
    dots: u8,
    pending: Option<String>,
    pattern: String,
}
#[repr(C)]
pub struct IoStatus {
    status: usize,
    information: usize,
}
fn finish(io: &mut IoStatus, status: u32, bytes: usize) -> i32 {
    io.status = status as usize;
    io.information = bytes;
    status as i32
}
fn ticks(seconds: i64, nanos: i64) -> u64 {
    ((seconds + 11644473600) as u64)
        .saturating_mul(10000000)
        .saturating_add(nanos.max(0) as u64 / 100)
}
fn record(name: &str, meta: &Metadata, class: u32, offset: usize) -> Vec<u8> {
    let wide = name.encode_utf16().collect::<Vec<_>>();
    let mut bytes = vec![0u8; offset + wide.len() * 2];
    let put32 = |bytes: &mut [u8], at: usize, value: u32| {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes())
    };
    let put64 = |bytes: &mut [u8], at: usize, value: u64| {
        bytes[at..at + 8].copy_from_slice(&value.to_le_bytes())
    };
    if class == 12 {
        put32(&mut bytes, 8, (wide.len() * 2) as u32);
    } else {
        for (index, time) in [
            ticks(meta.ctime(), meta.ctime_nsec()),
            ticks(meta.atime(), meta.atime_nsec()),
            ticks(meta.mtime(), meta.mtime_nsec()),
            ticks(meta.ctime(), meta.ctime_nsec()),
        ]
        .iter()
        .enumerate()
        {
            put64(&mut bytes, 8 + index * 8, *time);
        }
        put64(&mut bytes, 40, if meta.is_dir() { 0 } else { meta.len() });
        put64(
            &mut bytes,
            48,
            if meta.is_dir() {
                0
            } else {
                meta.blocks() * 512
            },
        );
        let mut attr = if meta.is_dir() { 16 } else { 0 };
        if meta.mode() & 0o200 == 0 {
            attr |= 1;
        }
        if name.starts_with('.') && name != "." && name != ".." {
            attr |= 2;
        }
        if meta.file_type().is_symlink() {
            attr |= 0x400;
        }
        if attr == 0 {
            attr = 128;
        }
        put32(&mut bytes, 56, attr);
        put32(&mut bytes, 60, (wide.len() * 2) as u32);
        if class == 37 {
            put64(&mut bytes, 96, meta.ino());
        }
        if class == 38 {
            put64(&mut bytes, 72, meta.ino());
        }
    }
    for (index, unit) in wide.iter().enumerate() {
        bytes[offset + index * 2..offset + index * 2 + 2].copy_from_slice(&unit.to_le_bytes());
    }
    bytes
}
pub unsafe extern "win64" fn query(
    handle: usize,
    event: usize,
    apc: usize,
    context: usize,
    io: *mut IoStatus,
    data: *mut u8,
    length: u32,
    class: u32,
    single: u8,
    mask: *const u8,
    restart: u8,
) -> i32 {
    if io.is_null() {
        return 0xc000000du32 as i32;
    }
    let io = unsafe { &mut *io };
    io.information = 0;
    if event | apc | context != 0 {
        return finish(io, 0xc00000bb, 0);
    }
    let offset = match class {
        1 => 64,
        2 => 68,
        3 => 94,
        12 => 12,
        37 => 104,
        38 => 80,
        _ => return finish(io, 0xc0000003, 0),
    };
    if data.is_null() || (length as usize) < offset {
        return finish(io, 0xc0000004, 0);
    }
    let Some(entry) = files::get(handle) else {
        return finish(io, 0xc0000008, 0);
    };
    if !entry.file.metadata().is_ok_and(|meta| meta.is_dir()) {
        return finish(io, 0xc0000103, 0);
    }
    let mut cursor = entry.directory.lock().unwrap();
    let first = cursor.is_none();
    if first {
        let pattern = if mask.is_null() {
            "*".to_owned()
        } else {
            let bytes = unsafe { mask.cast::<u16>().read_unaligned() } as usize;
            let text = unsafe { mask.add(8).cast::<*const u16>().read_unaligned() };
            if bytes % 2 != 0 || text.is_null() {
                return finish(io, 0xc000000d, 0);
            }
            match String::from_utf16(unsafe { std::slice::from_raw_parts(text, bytes / 2) }) {
                Ok(text) => text,
                Err(_) => return finish(io, 0xc000000d, 0),
            }
        };
        // DOS wildcard forms need their own matching rules; fail explicitly.
        if pattern.contains(['<', '>', '"']) {
            return finish(io, 0xc00000bb, 0);
        }
        let base = PathBuf::from(format!("/proc/self/fd/{}", entry.file.as_raw_fd()));
        let entries = match std::fs::read_dir(&base) {
            Ok(entries) => entries,
            Err(_) => return finish(io, 0xc0000022, 0),
        };
        *cursor = Some(Scan {
            base,
            entries,
            dots: 0,
            pending: None,
            pattern,
        });
    }
    let scan = cursor.as_mut().unwrap();
    if restart != 0 && !first {
        scan.entries = match std::fs::read_dir(&scan.base) {
            Ok(entries) => entries,
            Err(_) => return finish(io, 0xc0000022, 0),
        };
        scan.dots = 0;
        scan.pending = None;
    }
    let mut used = 0usize;
    let mut previous = None;
    loop {
        let name = if let Some(name) = scan.pending.take() {
            name
        } else if scan.dots < 2 {
            scan.dots += 1;
            if scan.dots == 1 {
                ".".into()
            } else {
                "..".into()
            }
        } else {
            match scan.entries.next() {
                Some(Ok(entry)) => entry.file_name().to_string_lossy().into_owned(),
                Some(Err(_)) => return finish(io, 0xc0000185, used),
                None => break,
            }
        };
        if !files::matches(&scan.pattern, &name) {
            continue;
        }
        let path = scan.base.join(&name);
        let meta = match std::fs::metadata(&path).or_else(|_| std::fs::symlink_metadata(&path)) {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        let bytes = record(&name, &meta, class, offset);
        if used + bytes.len() > length as usize {
            scan.pending = Some(name);
            if used == 0 {
                if first {
                    let n = (length as usize).min(bytes.len());
                    unsafe {
                        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, n);
                    }
                    return finish(io, 0x80000005, n);
                }
                return finish(io, 0, 0);
            }
            break;
        }
        if let Some(at) = previous {
            unsafe {
                data.add(at)
                    .cast::<u32>()
                    .write_unaligned((used - at) as u32);
            }
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.add(used), bytes.len());
        }
        previous = Some(used);
        used += bytes.len();
        let aligned = (used + 7) & !7;
        if aligned <= length as usize {
            unsafe {
                data.add(used).write_bytes(0, aligned - used);
            }
            used = aligned;
        }
        if single != 0 {
            break;
        }
    }
    if previous.is_none() {
        finish(io, if first { 0xc000000f } else { 0x80000006 }, 0)
    } else {
        finish(io, 0, used)
    }
}
pub extern "win64" fn status_to_dos(status: u32) -> u32 {
    match status {
        0 => 0,
        0x103 => 997,
        0xc0000008 => 6,
        0xc000000d => 87,
        0xc0000003 => 124,
        0xc0000004 => 24,
        0xc00000bb => 50,
        0xc0000103 => 267,
        0xc000000f | 0xc0000034 => 2,
        0x80000006 => 18,
        0x80000005 => 234,
        0xc0000022 => 5,
        0xc0000185 => 1117,
        0xc0000120 => 995,
        _ => 317,
    }
}
