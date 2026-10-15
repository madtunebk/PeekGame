//! Known folders (ported from peek's shell32.inc). Roaming AppData is the APPDATA
//! directory: the APPDATA environment variable when set, otherwise
//! `$XDG_DATA_HOME/peekgame` (default `~/.local/share/peekgame`), so game data
//! persists between runs. The user profile is the same directory; the other folders
//! are created inside it on first use, as they always exist on Windows. Folders with
//! no meaning here (Windows, System, Program Files, ...) are not found.
use crate::{
    api::{com::apartment, text},
    host::paths,
};
use std::path::PathBuf;
const E_INVALIDARG: i32 = 0x80070057u32 as i32;
const E_FAIL: i32 = 0x80004005u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007000Eu32 as i32;
const NOT_FOUND: i32 = 0x80070002u32 as i32;

/// (GUID, CSIDL, subdirectory of AppData).
const FOLDERS: &[(&str, i32, &str)] = &[
    ("3EB685DB-65F9-4CF6-A03A-E3EF65729F3D", 0x1a, ""), // RoamingAppData
    ("5E6C858F-0E22-4760-9AFE-EA3317B67173", 0x28, ""), // Profile
    ("F1B32785-6FBA-4FCF-9D55-7B8E7F157091", 0x1c, "Local"),
    ("A520A1A4-1780-4FF6-BD18-167343C5AF16", -1, "LocalLow"),
    ("FDD39AD0-238F-46AF-ADB4-6C85480369C7", 0x05, "Documents"),
    ("4C5C32FF-BB9D-43B0-B5B4-2D72E54EAAA4", -1, "Saved Games"),
    ("B4BFCC3A-DB2C-424C-B029-7FE99A87C641", 0x10, "Desktop"),
    ("33E28130-4E1E-4676-835A-98395C3BC3BB", 0x27, "Pictures"),
    ("4BD8D571-6D19-48D3-BE97-422220080E43", 0x0d, "Music"),
    ("18989B1D-99B5-455B-841C-AB7C74E4DDFC", 0x0e, "Videos"),
    ("374DE290-123F-4565-9164-39C4925E467B", -1, "Downloads"),
];
/// The AppData root on the host (created) and its Windows path.
fn appdata() -> Option<PathBuf> {
    let root = match std::env::var("APPDATA") {
        Ok(windows) if !windows.is_empty() => paths::host(&windows).ok()?,
        _ => {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
                })?;
            let root = data.join("peekgame");
            // Programs read APPDATA directly too.
            unsafe { std::env::set_var("APPDATA", paths::windows(&root)) };
            root
        }
    };
    std::fs::create_dir_all(&root).ok()?;
    Some(root)
}
/// The folder's Windows path, created on the host when missing.
fn folder(subdirectory: &str) -> Option<String> {
    let mut path = appdata()?;
    if !subdirectory.is_empty() {
        path = path.join(subdirectory);
        std::fs::create_dir_all(&path).ok()?;
    }
    Some(paths::windows(&path))
}
pub unsafe extern "win64" fn known_folder_path(
    id: *const u8,
    _flags: u32,
    token: usize,
    out: *mut *mut u16,
) -> i32 {
    if out.is_null() {
        return E_INVALIDARG;
    }
    unsafe { *out = std::ptr::null_mut() };
    if id.is_null() || token != 0 {
        return E_INVALIDARG;
    }
    let g = unsafe { std::slice::from_raw_parts(id, 16) };
    let guid = format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{}",
        u32::from_le_bytes(g[0..4].try_into().unwrap()),
        u16::from_le_bytes(g[4..6].try_into().unwrap()),
        u16::from_le_bytes(g[6..8].try_into().unwrap()),
        g[8],
        g[9],
        g[10..]
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>()
    );
    let Some(&(_, _, subdirectory)) = FOLDERS.iter().find(|(g, _, _)| *g == guid) else {
        eprintln!("SHGetKnownFolderPath({{{guid}}}): folder not available");
        return NOT_FOUND;
    };
    let Some(path) = folder(subdirectory) else {
        return E_FAIL;
    };
    let mut wide = text::wide(&path);
    wide.push(0);
    let block = apartment::allocate(wide.len() * 2).cast::<u16>();
    if block.is_null() {
        return E_OUTOFMEMORY;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), block, wide.len());
        *out = block;
    }
    0
}
/// SHGetFolderPathW: CSIDL_FLAG_CREATE (0x8000) and the other high flags are accepted.
pub unsafe extern "win64" fn folder_path_w(
    _hwnd: usize,
    csidl: i32,
    token: usize,
    _flags: u32,
    out: *mut u16,
) -> i32 {
    if out.is_null() || token != 0 {
        return E_INVALIDARG;
    }
    unsafe { *out = 0 };
    let Some(&(_, _, subdirectory)) = FOLDERS.iter().find(|(_, c, _)| *c == csidl & 0xff) else {
        eprintln!("SHGetFolderPathW(CSIDL {csidl:#x}): folder not available");
        return NOT_FOUND;
    };
    match folder(subdirectory) {
        Some(path) if path.encode_utf16().count() < 260 => {
            unsafe { text::copy_out(&text::wide(&path), out, 260) };
            0
        }
        Some(_) => 0x800700CEu32 as i32,
        None => E_FAIL,
    }
}
