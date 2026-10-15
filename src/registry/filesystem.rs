use super::*;
use crate::api::filesystem::paths;
pub(super) static EXPORTS: &[Export] = &[
    implemented!(
        "GetLongPathNameW",
        "filesystem",
        "partial",
        "existing paths without synthetic 8.3 aliases",
        paths::long_w
    ),
    implemented!(
        "GetCurrentDirectoryW",
        "filesystem",
        "partial",
        "host current directory through Z: root mapping",
        paths::current_w
    ),
    implemented!(
        "SetCurrentDirectoryW",
        "filesystem",
        "partial",
        "host current directory through Z: root mapping",
        paths::set_current_w
    ),
    implemented!(
        "GetFullPathNameW",
        "filesystem",
        "partial",
        "lexical absolute path through Z: mapping",
        paths::full_w
    ),
    implemented!(
        "GetModuleFileNameW",
        "filesystem",
        "partial",
        "configured main executable; runtime module files unavailable",
        paths::module_w
    ),
    implemented!(
        "GetFileAttributesW",
        "filesystem",
        "partial",
        "host file/directory/read-only attributes",
        paths::attributes_w
    ),
    implemented!(
        "GetFileAttributesA",
        "filesystem",
        "partial",
        "host file/directory/read-only attributes; UTF-8",
        paths::attributes_a
    ),
];
