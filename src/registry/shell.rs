use super::*;
use crate::api::{memory::local, shell::arguments};
pub(super) static EXPORTS: &[Export] = &[
    Export {
        dll: "shell32.dll",
        name: "CommandLineToArgvW",
        category: "shell",
        status: "partial",
        scope: "UTF-16 argument parsing; LocalFree owns returned block",
        address: Some(|| arguments::command_line_to_argv as *const std::ffi::c_void),
    },
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
];
