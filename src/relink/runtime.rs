//! Runtime dependencies kept beside a relinked executable, independent of PeekGame.
use std::{env, fs, path::Path};

pub(super) fn stage_windows_dlls(host: &Path, output: &Path) -> Result<(), String> {
    let source = env::var_os("PEEKGAME_WINDOWS_DLLS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| host.parent().unwrap_or(Path::new(".")).join("windows-dlls"));
    if !source.is_dir() {
        return Ok(());
    }
    let destination = output.join("windows-dlls");
    fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    if fs::canonicalize(&source).ok() == fs::canonicalize(&destination).ok() {
        return Ok(());
    }
    for entry in fs::read_dir(&source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let target = destination.join(entry.file_name());
        fs::copy(&path, &target).map_err(|e| format!("copy {}: {e}", path.display()))?;
    }
    Ok(())
}
