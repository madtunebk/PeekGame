//! Export declarations for console. Implementation stays in api/.
use crate::api::system::console;
use crate::registry::Export;

exports! {
    implemented!(
        "SetConsoleCP",
        "system",
        "partial",
        "UTF-8 console policy only",
        console::output_cp
    ),
    implemented!(
        "AttachConsole",
        "system",
        "partial",
        "inherited terminal-backed streams only",
        console::attach
    ),
    implemented!(
        "AllocConsole",
        "system",
        "partial",
        "inherited terminal-backed streams only",
        console::allocate
    ),
    implemented!(
        "SetConsoleOutputCP",
        "system",
        "partial",
        "inherited terminal-backed streams only",
        console::output_cp
    ),
    implemented!(
        "GetConsoleMode",
        "system",
        "partial",
        "inherited terminal-backed streams only",
        console::mode
    ),
    implemented!(
        "GetFileType",
        "system",
        "partial",
        "inherited terminal-backed streams only",
        console::file_type
    ),
    implemented!("WriteConsoleW", "system", "partial", "UTF-16 output to inherited host terminals; redirected handles fail", crate::api::system::console::write_w),
    implemented!("ReadConsoleW", "system", "partial", "UTF-16 input from inherited host terminal stdin", crate::api::system::console::read_w),
    implemented!("SetConsoleMode", "system", "partial", "inherited terminal output: processed, wrap and VT; input/screen-buffer modes unsupported", crate::api::system::console::set_mode),
    implemented!("GetConsoleScreenBufferInfo", "system", "partial", "terminal/handle validation; no emulated Win32 screen buffer: ERROR_NOT_SUPPORTED", crate::api::system::console::screen_buffer_info),
    implemented!("SetConsoleTextAttribute", "system", "partial", "terminal ANSI foreground/background and intensity; COMMON_LVB flags unsupported", crate::api::system::console::text_attribute),
    implemented!("SetConsoleCtrlHandler", "system", "partial", "register or remove process console control handler", crate::api::system::console::set_ctrl_handler),
    implemented!("GetConsoleWindow", "system", "partial", "returns NULL window handle", console::get_console_window),
    implemented!("GetConsoleTitleW", "system", "implemented", "retrieves console title as UTF-16", console::get_title_w),
    implemented!("GetConsoleTitleA", "system", "implemented", "retrieves console title as ANSI", console::get_title_a),
    implemented!("SetConsoleTitleW", "system", "implemented", "sets console title from UTF-16", console::set_title_w),
    implemented!("SetConsoleTitleA", "system", "implemented", "sets console title from ANSI", console::set_title_a),
}

