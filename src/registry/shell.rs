use super::*;
use crate::api::shell::{folders, paths};
use crate::api::{memory::local, shell::arguments};
exports! {
 export!("shell32.dll","DragQueryFileW","shell","partial","UTF-16 CF_HDROP count, length and file names",crate::api::window::drop_data::query_file),
 export!("shell32.dll","DragFinish","shell","implemented","release HDROP global allocation",crate::api::window::drop_data::finish),
    export!("shell32.dll", "SHGetPropertyStoreForWindow", "shell", "partial", "per-window shell property store unavailable: NULL output and E_NOTIMPL", crate::api::shell::identity::window_property_store),

    export!("shell32.dll", "DragAcceptFiles", "window", "partial", "updates WS_EX_ACCEPTFILES; native WM_DROPFILES delivery unsupported and logged", crate::api::window::drops::accept),

    export!("shell32.dll", "SetCurrentProcessExplicitAppUserModelID", "shell", "partial", "process application ID becomes WM_CLASS for subsequent top-level X11 windows", crate::api::shell::identity::set),
    export!("shell32.dll", "ShellExecuteW", "shell", "partial", "open/explore URI and files through host GIO applications; no Windows process execution", crate::api::shell::execute::execute_w),
    export!("shell32.dll", "ShellExecuteA", "shell", "partial", "open/explore URI and files through host GIO applications; no Windows process execution", crate::api::shell::execute::execute_a),
    export!("shell32.dll", "ShellExecuteExW", "shell", "partial", "open/explore URI and files through host GIO applications", crate::api::shell::execute::execute_ex_w),
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
        "LocalReAlloc",
        "memory",
        "partial",
        "registered LocalAlloc block resize with optional zero-fill",
        local::reallocate
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
    export!(
        "shlwapi.dll",
        "PathCanonicalizeW",
        "shell",
        "implemented",
        "canonicalizes path, collapses . and .. components",
        paths::canonicalize_w
    ),
    export!(
        "shlwapi.dll",
        "PathCombineW",
        "shell",
        "implemented",
        "concatenates dir and file and canonicalizes",
        paths::combine_w
    ),
    export!(
        "shlwapi.dll",
        "PathRemoveFileSpecW",
        "shell",
        "implemented",
        "removes trailing file name component",
        paths::remove_file_spec_w
    ),
}
