use std::{env, fs, path::Path};
pub fn run(root: &Path) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    for name in ["cache", "logs"] {
        fs::create_dir_all(root.join(name)).map_err(|e| e.to_string())?;
    }
    let log = fs::File::create(root.join("logs/run.log")).map_err(|e| e.to_string())?;
    // Redirect before loading the runtime so loader/driver diagnostics share the log.
    unsafe extern "C" {
        fn dup2(old: i32, new: i32) -> i32;
    }
    use std::os::fd::AsRawFd;
    for stream in [1, 2] {
        if unsafe { dup2(log.as_raw_fd(), stream) } < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    env::set_current_dir(&root).map_err(|e| e.to_string())?;
    env::set_var("PEEKGAME_DXVK_DIR", root.join("runtime"));
    env::set_var("DXVK_WSI_DRIVER", "PeekX11");
    env::set_var("DXVK_LOG_PATH", root.join("logs"));
    env::set_var("XDG_CACHE_HOME", root.join("cache"));
    let backend = crate::loader::backend::Backend::open(&root.join("game_imports.so"))?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    crate::loader::run_with(
        &root.join("game.exe"),
        &args,
        &root.join("logs"),
        Some(&backend),
    )
}
