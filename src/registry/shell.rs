use super::*;
use crate::api::shell::folders;
use crate::api::{memory::local, shell::arguments};
exports! {
    export!(
        "shell32.dll",
        "SHGetKnownFolderPath",
        "shell",
        "partial",
        "AppData and user folders under APPDATA (default ~/.local/share/peekgame)",
        folders::known_folder_path
    ),
    export!(
        "shell32.dll",
        "SHGetFolderPathW",
        "shell",
        "partial",
        "AppData and user folders under APPDATA (default ~/.local/share/peekgame)",
        folders::folder_path_w
    ),
    export!(
        "shell32.dll",
        "CommandLineToArgvW",
        "shell",
        "partial",
        "UTF-16 argument parsing; LocalFree owns returned block",
        arguments::command_line_to_argv
    ),
    implemented!(
        "LocalAlloc",
        "memory",
        "partial",
        "fixed allocation with optional zero-fill; movable unsupported",
        local::allocate
    ),
    implemented!(
        "LocalFree",
        "memory",
        "partial",
        "registered LocalAlloc blocks",
        local::release
    ),
    implemented!(
        "LocalSize",
        "memory",
        "partial",
        "registered LocalAlloc sizes",
        local::size
    ),
}
