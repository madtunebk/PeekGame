use std::{
    ffi::{c_char, CStr},
    path::{Path, PathBuf},
};
pub unsafe fn narrow(p: *const c_char) -> Result<String, u32> {
    if p.is_null() {
        return Err(87);
    }
    unsafe { CStr::from_ptr(p) }
        .to_str()
        .map(str::to_owned)
        .map_err(|_| 1113)
}
pub unsafe fn wide(p: *const u16) -> Result<String, u32> {
    if p.is_null() {
        return Err(87);
    }
    let mut len = 0;
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(p, len) }).map_err(|_| 1113)
}
pub fn host(text: &str) -> Result<PathBuf, u32> {
    let mut text = text.replace('\\', "/");
    if text.starts_with("//?/") {
        text = text[4..].to_owned();
    }
    if text.as_bytes().get(1) == Some(&b':') {
        if !text[..1].eq_ignore_ascii_case("z") {
            return Err(3);
        }
        text = text[2..].to_owned();
    }
    if text.is_empty() {
        return Err(3);
    }
    Ok(PathBuf::from(text))
}
pub fn windows(path: &Path) -> String {
    let text = path.to_string_lossy().replace('/', "\\");
    if path.is_absolute() {
        format!("Z:{text}")
    } else {
        text
    }
}
pub fn error(e: &std::io::Error) -> u32 {
    match e.raw_os_error() {
        Some(2) => 2,
        Some(20) => 3,
        Some(13 | 1) => 5,
        Some(17) => 183,
        Some(39) => 145,
        Some(28) => 112,
        _ => 50,
    }
}
pub fn full(path: &str) -> Result<PathBuf, u32> {
    let path = host(path)?;
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map_err(|e| error(&e))?.join(path)
    };
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            value => result.push(value.as_os_str()),
        }
    }
    Ok(result)
}
