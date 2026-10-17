//! SHLWAPI path manipulation functions: PathCanonicalizeW, PathCombineW, PathRemoveFileSpecW.
use crate::api::text;

/// PathCanonicalizeW: collapses . and .. components, replaces / with \, into pszBuf.
pub unsafe extern "win64" fn canonicalize_w(buf: *mut u16, path: *const u16) -> i32 {
    if buf.is_null() || path.is_null() {
        return 0;
    }
    let Some(raw) = (unsafe { text::string(path) }) else {
        return 0;
    };
    let canonical = canonicalize(&raw);
    let wide = text::wide(&canonical);
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), buf, wide.len());
        *buf.add(wide.len()) = 0;
    }
    1
}

/// PathCombineW: concatenates dir and file, canonicalizes result into pszDest.
pub unsafe extern "win64" fn combine_w(
    dest: *mut u16,
    dir: *const u16,
    file: *const u16,
) -> *mut u16 {
    if dest.is_null() {
        return std::ptr::null_mut();
    }
    let dir_str = if !dir.is_null() {
        unsafe { text::string(dir) }.unwrap_or_default()
    } else {
        String::new()
    };
    let file_str = if !file.is_null() {
        unsafe { text::string(file) }.unwrap_or_default()
    } else {
        String::new()
    };

    let combined = if file_str.starts_with('\\')
        || file_str.starts_with('/')
        || (file_str.len() >= 2 && file_str.as_bytes()[1] == b':')
    {
        file_str
    } else if dir_str.is_empty() {
        file_str
    } else if dir_str.ends_with('\\') || dir_str.ends_with('/') {
        format!("{dir_str}{file_str}")
    } else {
        format!("{dir_str}\\{file_str}")
    };

    let canonical = canonicalize(&combined);
    let wide = text::wide(&canonical);
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dest, wide.len());
        *dest.add(wide.len()) = 0;
    }
    dest
}

/// PathRemoveFileSpecW: removes the trailing file name and separator from a path.
pub unsafe extern "win64" fn remove_file_spec_w(path: *mut u16) -> i32 {
    if path.is_null() {
        return 0;
    }
    let mut len = 0;
    while unsafe { *path.add(len) } != 0 {
        len += 1;
    }
    if len == 0 {
        return 0;
    }
    let mut i = len;
    while i > 0 {
        i -= 1;
        let c = unsafe { *path.add(i) };
        if c == b'\\' as u16 || c == b'/' as u16 {
            if i == 2 && unsafe { *path.add(1) } == b':' as u16 {
                unsafe { *path.add(i + 1) = 0 };
            } else {
                unsafe { *path.add(i) = 0 };
            }
            return 1;
        }
    }
    unsafe { *path = 0 };
    1
}

fn canonicalize(path: &str) -> String {
    let normalized = path.replace('/', "\\");
    let (prefix, rest) = if let Some(idx) = normalized.find(':') {
        let (p, r) = normalized.split_at(idx + 1);
        (p.to_string(), r)
    } else if normalized.starts_with(r"\\") {
        (r"\\".to_string(), &normalized[2..])
    } else if normalized.starts_with('\\') {
        (r"\".to_string(), &normalized[1..])
    } else {
        (String::new(), normalized.as_str())
    };

    let mut parts: Vec<&str> = Vec::new();
    for part in rest.split('\\') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            parts.pop();
        } else {
            parts.push(part);
        }
    }

    if prefix.ends_with('\\') {
        format!("{}{}", prefix, parts.join("\\"))
    } else if !prefix.is_empty() {
        format!("{}\\{}", prefix, parts.join("\\"))
    } else {
        parts.join("\\")
    }
}
