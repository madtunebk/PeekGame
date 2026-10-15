use super::*;
use crate::api::{memory::local, shell::arguments};
exports! {
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
