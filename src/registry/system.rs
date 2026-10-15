use super::*;
use crate::api::system::console;
use crate::api::system::cpu;
use crate::api::system::dynamic;
use crate::api::system::encoding;
use crate::api::system::startup;
pub(super) static EXPORTS: &[Export] = &[
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
    implemented!(
        "GetACP",
        "system",
        "partial",
        "UTF-8 ANSI policy",
        encoding::codepage
    ),
    implemented!(
        "GetOEMCP",
        "system",
        "partial",
        "UTF-8 OEM policy",
        encoding::codepage
    ),
    implemented!(
        "WideCharToMultiByte",
        "system",
        "partial",
        "UTF-8/ANSI UTF-16 conversion; validates flags and buffers",
        encoding::wide_to_multi
    ),
    implemented!(
        "MultiByteToWideChar",
        "system",
        "partial",
        "UTF-8/ANSI UTF-16 conversion; validates flags and buffers",
        encoding::multi_to_wide
    ),
    implemented!(
        "GetModuleHandleA",
        "system",
        "partial",
        "main image or registered runtime modules",
        modules::get_module_handle_a
    ),
    implemented!(
        "GetModuleHandleExW",
        "system",
        "partial",
        "main-image address lookup and registered runtime names; lifetime fixed",
        modules::get_module_handle_ex_w
    ),
    implemented!(
        "GetModuleHandleExA",
        "system",
        "partial",
        "main-image address lookup and registered runtime names; lifetime fixed",
        modules::get_module_handle_ex_a
    ),
    implemented!(
        "SetUnhandledExceptionFilter",
        "system",
        "partial",
        "handler registration; dispatch pending",
        startup::set_filter
    ),
    implemented!(
        "GetCurrentProcessId",
        "system",
        "partial",
        "host process ID",
        startup::process_id
    ),
    implemented!(
        "GetCurrentThreadId",
        "system",
        "partial",
        "host thread ID",
        startup::thread_id
    ),
    implemented!(
        "GetCurrentProcess",
        "system",
        "partial",
        "Windows current-process pseudo handle",
        startup::process
    ),
    implemented!(
        "GetCurrentThread",
        "system",
        "partial",
        "Windows current-thread pseudo handle",
        startup::thread
    ),
    implemented!(
        "SetLastError",
        "system",
        "partial",
        "thread-local Win32 error",
        startup::set_error
    ),
    implemented!(
        "GetStartupInfoW",
        "system",
        "partial",
        "inherited streams and default startup fields",
        startup::startup_info
    ),
    implemented!(
        "GetStartupInfoA",
        "system",
        "partial",
        "inherited streams and default startup fields",
        startup::startup_info
    ),
    implemented!(
        "GetSystemTimeAsFileTime",
        "system",
        "partial",
        "host UTC file time",
        startup::file_time
    ),
    implemented!(
        "QueryPerformanceFrequency",
        "system",
        "partial",
        "nanosecond host monotonic clock",
        startup::performance_frequency
    ),
    implemented!(
        "QueryPerformanceCounter",
        "system",
        "partial",
        "host monotonic clock",
        startup::performance_counter
    ),
    implemented!(
        "Sleep",
        "system",
        "partial",
        "host thread sleep",
        startup::sleep
    ),
    implemented!(
        "IsDebuggerPresent",
        "system",
        "partial",
        "no Windows debugger attached",
        startup::is_debugger_present
    ),
    implemented!(
        "GetCommandLineW",
        "system",
        "partial",
        "UTF-16 configured command line",
        startup::get_command_line_w
    ),
    implemented!(
        "IsProcessorFeaturePresent",
        "system",
        "partial",
        "host CPU features; unsupported OS facilities and unmapped features absent",
        cpu::present
    ),
    implemented!(
        "LoadLibraryA",
        "system",
        "partial",
        "registered runtime modules only; external PE DLL loading unsupported",
        dynamic::load_library_a
    ),
    implemented!(
        "LoadLibraryW",
        "system",
        "partial",
        "registered runtime modules only; external PE DLL loading unsupported",
        dynamic::load_library_w
    ),
    implemented!(
        "GetProcAddress",
        "system",
        "partial",
        "named exports from registered runtime modules; logs requests",
        dynamic::get_proc_address
    ),
    implemented!(
        "GetLastError",
        "system",
        "implemented",
        "per-thread Win32 error state; TEB not provided yet",
        error::get_last_error
    ),
    implemented!(
        "GetCommandLineA",
        "system",
        "implemented",
        "loader-supplied command line; requires process configuration",
        command_line::get_command_line_a
    ),
    implemented!(
        "GetModuleHandleW",
        "system",
        "partial",
        "main image or registered runtime modules; external PE DLLs unavailable",
        modules::get_module_handle_w
    ),
    implemented!(
        "GetStdHandle",
        "system",
        "partial",
        "inherited standard streams; requires process configuration",
        standard_handles::get_std_handle
    ),
    implemented!(
        "OutputDebugStringA",
        "system",
        "partial",
        "debug text goes to Linux stderr; Windows debugger delivery unavailable",
        debug::output_debug_string_a
    ),
    implemented!(
        "ExitProcess",
        "system",
        "partial",
        "terminates host process; PE DLL detach callbacks not implemented",
        process::exit_process
    ),
    implemented!(
        "WriteFile",
        "filesystem",
        "partial",
        "synchronous inherited standard streams only; overlapped I/O unsupported",
        write::write_file
    ),
];
