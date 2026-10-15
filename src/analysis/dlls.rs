use crate::model::Result;
use std::{fs, path::PathBuf};
pub(super) fn find_dll(dll: &str, dirs: &[PathBuf]) -> Result<Option<PathBuf>> {
    if dll.contains(['/', '\\', ':']) {
        return Err(format!("unsafe DLL name {dll:?}"));
    }
    for dir in dirs {
        let mut matches = Vec::new();
        for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(dll)
                && entry.path().is_file()
            {
                matches.push(entry.path());
            }
        }
        if matches.len() > 1 {
            return Err(format!("ambiguous DLL {dll} in {}", dir.display()));
        }
        if let Some(path) = matches.pop() {
            return fs::canonicalize(path).map(Some).map_err(|e| e.to_string());
        }
    }
    Ok(None)
}
