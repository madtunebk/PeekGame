//! Where LoadLibrary looks for a program DLL. A bare name is searched in the EXE's
//! directory, then the SetDllDirectory directory, then the current directory (not
//! while a DLL directory is set, even ""); `first_dir` comes before all. A name with
//! a path is looked up there only. As in LoadLibrary, a name without an extension
//! means NAME.dll and a trailing '.' means no extension. Matching ignores case.
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
static EXE_DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);
/// `None`: not set. `Some(None)`: set to "" (only the EXE directory is searched).
static DLL_DIRECTORY: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

pub fn set_exe_directory(dir: Option<&Path>) {
    *EXE_DIRECTORY.lock().unwrap() = dir.map(Path::to_owned);
}
pub fn set_dll_directory(path: Option<&str>) -> bool {
    let value = match path {
        None => None,
        Some("") => Some(None),
        Some(p) => match host_path(p) {
            Some(dir) => Some(Some(dir)),
            None => return false,
        },
    };
    *DLL_DIRECTORY.lock().unwrap() = value;
    true
}

/// A Windows path as a host path: `\` becomes `/`; drive Z: is the host root and
/// other drives do not exist.
pub fn host_path(name: &str) -> Option<PathBuf> {
    let mut path = name.replace('\\', "/");
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        if !path[..1].eq_ignore_ascii_case("z") {
            return None;
        }
        path.replace_range(..2, "");
    }
    Some(PathBuf::from(path))
}

/// The file name LoadLibrary means by `name`, with the default `.dll` extension.
pub fn normalize(name: &str) -> Option<String> {
    let path = host_path(name)?;
    let mut file = path.file_name()?.to_string_lossy().into_owned();
    if let Some(stripped) = file.strip_suffix('.') {
        file = stripped.to_owned();
    } else if !file.contains('.') {
        file.push_str(".dll");
    }
    Some(file)
}

fn find_in(dir: &Path, file: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        (entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(file)
            && path.is_file())
        .then_some(path)
    })
}

pub fn find(name: &str, first_dir: Option<&Path>) -> Option<PathBuf> {
    let file = normalize(name)?;
    let path = host_path(name)?;
    if name.contains(['/', '\\']) {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("/"));
        return find_in(dir, &file);
    }
    if let Some(found) = first_dir.and_then(|d| find_in(d, &file)) {
        return Some(found);
    }
    if let Some(found) = EXE_DIRECTORY
        .lock()
        .unwrap()
        .as_deref()
        .and_then(|d| find_in(d, &file))
    {
        return Some(found);
    }
    match &*DLL_DIRECTORY.lock().unwrap() {
        Some(Some(dir)) => find_in(dir, &file),
        Some(None) => None,
        None => find_in(Path::new("."), &file),
    }
}
