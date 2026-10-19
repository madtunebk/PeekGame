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
pub fn peekgame_root() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("peekgame")
}

pub fn host(text: &str) -> Result<PathBuf, u32> {
    let mut text = text.replace('\\', "/");
    if text.starts_with("//?/") {
        text = text[4..].to_owned();
    }
    if text.as_bytes().get(1) == Some(&b':') {
        let drive = text.as_bytes()[0];
        if drive.eq_ignore_ascii_case(&b'c') {
            let relative = text[2..].trim_start_matches('/');
            let lower = relative.to_ascii_lowercase();
            if lower.contains("steamclient64.dll") {
                if let Ok(home) = std::env::var("HOME") {
                    let candidates = [
                        PathBuf::from(&home).join(".steam/debian-installation/steamclient64.dll"),
                        PathBuf::from(&home).join(".steam/debian-installation/legacycompat/steamclient64.dll"),
                        PathBuf::from(&home).join(".steam/steam/steamclient64.dll"),
                        PathBuf::from(&home).join(".steam/root/steamclient64.dll"),
                    ];
                    for cand in &candidates {
                        if cand.is_file() {
                            return Ok(cand.clone());
                        }
                    }
                }
            }
            if lower.contains("gameoverlayrenderer64.dll") {
                if let Ok(home) = std::env::var("HOME") {
                    let candidates = [
                        PathBuf::from(&home).join(".steam/debian-installation/GameOverlayRenderer64.dll"),
                        PathBuf::from(&home).join(".steam/debian-installation/legacycompat/GameOverlayRenderer64.dll"),
                        PathBuf::from(&home).join(".steam/steam/GameOverlayRenderer64.dll"),
                        PathBuf::from(&home).join(".steam/root/GameOverlayRenderer64.dll"),
                    ];
                    for cand in &candidates {
                        if cand.is_file() {
                            return Ok(cand.clone());
                        }
                    }
                }
            }
            let root = peekgame_root();
            let norm = relative.replace('\\', "/");
            let lower = norm.to_ascii_lowercase();

            if let Some(rest) = lower.strip_prefix("users/steamuser/appdata/local/") {
                return Ok(resolve_case(root.join("Local").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "users/steamuser/appdata/local" {
                return Ok(resolve_case(root.join("Local")));
            } else if let Some(rest) = lower.strip_prefix("users/steamuser/appdata/locallow/") {
                return Ok(resolve_case(root.join("LocalLow").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "users/steamuser/appdata/locallow" {
                return Ok(resolve_case(root.join("LocalLow")));
            } else if let Some(rest) = lower.strip_prefix("users/steamuser/saved games/") {
                return Ok(resolve_case(root.join("Saved Games").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "users/steamuser/saved games" {
                return Ok(resolve_case(root.join("Saved Games")));
            } else if let Some(rest) = lower.strip_prefix("users/steamuser/documents/") {
                return Ok(resolve_case(root.join("Documents").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "users/steamuser/documents" {
                return Ok(resolve_case(root.join("Documents")));
            } else if let Some(rest) = lower.strip_prefix("users/steamuser/desktop/") {
                return Ok(resolve_case(root.join("Desktop").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "users/steamuser/desktop" {
                return Ok(resolve_case(root.join("Desktop")));
            } else if let Some(rest) = lower.strip_prefix("users/steamuser/downloads/") {
                return Ok(resolve_case(root.join("Downloads").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "users/steamuser/downloads" {
                return Ok(resolve_case(root.join("Downloads")));
            } else if let Some(rest) = lower.strip_prefix("programdata/") {
                return Ok(resolve_case(root.join("ProgramData").join(&norm[norm.len() - rest.len()..])));
            } else if lower == "programdata" {
                return Ok(resolve_case(root.join("ProgramData")));
            }

            let drive_c = root.join("drive_c");
            return Ok(resolve_case(drive_c.join(relative)));
        } else if !drive.eq_ignore_ascii_case(&b'z') {
            return Err(3);
        }
        text = text[2..].to_owned();
    }
    if text.is_empty() {
        return Err(3);
    }
    if let Some(device) = device(&text) {
        return device;
    }
    Ok(resolve_case(PathBuf::from(text)))
}
/// Windows device names in the last component, also with an extension ("nul.txt"):
/// NUL is /dev/null; the other devices do not exist here (ERROR_FILE_NOT_FOUND).
fn device(text: &str) -> Option<Result<PathBuf, u32>> {
    let last = text.rsplit('/').next().unwrap_or(text);
    let stem = last.split('.').next().unwrap_or(last).trim_end_matches(' ').to_ascii_uppercase();
    let numbered = |prefix: &str| {
        stem.strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && matches!(n.as_bytes()[0], b'1'..=b'9'))
    };
    match stem.as_str() {
        "NUL" => Some(Ok(PathBuf::from("/dev/null"))),
        "CON" | "PRN" | "AUX" => Some(Err(2)),
        _ if numbered("COM") || numbered("LPT") => Some(Err(2)),
        _ => None,
    }
}
/// Windows paths ignore case. The exact path is used when it exists (no cost on the
/// usual path); otherwise each component that is missing exactly is matched against
/// its directory ignoring case (an exact match wins over a case variant, ties go to
/// the first name in sorted order). Components with no match stay as written, so
/// new files and directories are created under the resolved parent.
pub fn resolve_case(path: PathBuf) -> PathBuf {
    use std::path::Component;
    if path.symlink_metadata().is_ok() {
        return path;
    }
    let mut resolved = PathBuf::new();
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            resolved.push(component.as_os_str());
            continue;
        };
        let exact = resolved.join(name);
        if exact.symlink_metadata().is_ok() {
            resolved = exact;
            continue;
        }
        let directory = if resolved.as_os_str().is_empty() { Path::new(".") } else { resolved.as_path() };
        let wanted = name.to_string_lossy().to_lowercase();
        let mut matches: Vec<std::ffi::OsString> = std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name())
            .filter(|candidate| candidate.to_string_lossy().to_lowercase() == wanted)
            .collect();
        matches.sort();
        match matches.into_iter().next() {
            Some(found) => resolved.push(found),
            None => {
                // Nothing to match from here on: keep the rest as written.
                resolved.push(name);
                for rest in components.by_ref() {
                    resolved.push(rest.as_os_str());
                }
                return resolved;
            }
        }
    }
    resolved
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
