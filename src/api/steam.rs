use std::{
    ffi::{c_char, c_void, CString},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

unsafe extern "C" {
    fn steam_bridge_open(path: *const c_char) -> i32;
    fn steam_bridge_export(name: *const c_char) -> *const c_void;
    fn steam_bridge_steam_pid() -> u32;
    fn steam_bridge_contains_address(addr: *const c_void) -> i32;
}

pub(crate) fn steam_pid() -> u32 {
    unsafe { steam_bridge_steam_pid() }
}

static LOADED: AtomicBool = AtomicBool::new(false);
static STEAMCLIENT_MAPPING: OnceLock<(usize, usize)> = OnceLock::new();

pub(crate) fn recognizes(name: &str) -> bool {
    let clean = name.trim_end_matches('.');
    let file = clean.rsplit(['/', '\\']).next().unwrap_or(clean);
    file.eq_ignore_ascii_case("steamclient64.dll") || file.eq_ignore_ascii_case("steamclient64")
}

pub(crate) fn base_address() -> usize {
    STEAMCLIENT_MAPPING.get().map(|(b, _)| *b).unwrap_or(0x138000000)
}

pub(crate) fn recognizes_address(address: usize) -> bool {
    if !loaded() {
        return false;
    }
    if let Some(&(base, size)) = STEAMCLIENT_MAPPING.get() {
        if address >= base && address - base < size {
            return true;
        }
    }
    unsafe { steam_bridge_contains_address(address as *const c_void) != 0 }
}

pub(crate) fn loaded() -> bool {
    LOADED.load(Ordering::Acquire)
}

pub(crate) fn open() -> bool {
    if std::env::var_os("SteamAppId").is_none() {
        let mut candidates = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("steam_appid.txt"));
            if let Some(parent) = cwd.parent() {
                candidates.push(parent.join("steam_appid.txt"));
            }
        }
        if let Some(exe) = crate::images::exe() {
            if let Some(parent) = exe.path.parent() {
                candidates.push(parent.join("steam_appid.txt"));
                if let Some(grandparent) = parent.parent() {
                    candidates.push(grandparent.join("steam_appid.txt"));
                }
            }
        }
        for cand in candidates {
            if let Ok(text) = std::fs::read_to_string(&cand) {
                let trimmed = text.trim();
                if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                    std::env::set_var("SteamAppId", trimmed);
                    std::env::set_var("SteamGameId", trimmed);
                    break;
                }
            }
        }
    }
    // Failed attempts can be retried. C serializes initialization and dlopen.
    if unsafe { steam_bridge_open(std::ptr::null()) } != 0 {
        return false;
    }
    LOADED.store(true, Ordering::Release);
    STEAMCLIENT_MAPPING.get_or_init(|| {
        if let Ok(home) = std::env::var("HOME") {
            let candidates = [
                PathBuf::from(&home).join(".steam/debian-installation/steamclient64.dll"),
                PathBuf::from(&home).join(".steam/debian-installation/legacycompat/steamclient64.dll"),
                PathBuf::from(&home).join(".steam/steam/steamclient64.dll"),
                PathBuf::from(&home).join(".steam/root/steamclient64.dll"),
            ];
            for cand in &candidates {
                if cand.is_file() {
                    if let Ok(bytes) = std::fs::read(cand) {
                        if let Ok(image) = crate::pe::parse(&bytes) {
                            if let Ok(mapping) = crate::images::map::Mapping::new(&bytes, &image) {
                                let _ = mapping.protect(&image);
                                let base = mapping.base();
                                let size = mapping.size();
                                std::mem::forget(mapping);
                                eprintln!("steam_bridge: mapped PE steamclient64.dll at {base:#x} ({size:#x} bytes)");
                                return (base, size);
                            }
                        }
                    }
                }
            }
        }
        (0x138000000, 0x019b3000)
    });
    true
}

pub(crate) fn export(name: &str) -> Option<usize> {
    if !loaded() {
        return None;
    }
    let name = CString::new(name).ok()?;
    let p = unsafe { steam_bridge_export(name.as_ptr()) };
    (!p.is_null()).then_some(p as usize)
}
