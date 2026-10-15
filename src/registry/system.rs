use super::*;
use crate::api::memory::heap;
use crate::api::system::console;
use crate::api::system::cpu;
use crate::api::system::dynamic;
use crate::api::system::encoding;
use crate::api::system::startup;
use crate::api::system::{environment, locale, pointers, time};
exports! {
    implemented!(
        "GetProcessHeap",
        "memory",
        "partial",
        "the single process heap",
        heap::get_process_heap
    ),
    implemented!(
        "HeapAlloc",
        "memory",
        "partial",
        "process heap; zero-fill; other heaps stop",
        heap::alloc
    ),
    implemented!(
        "HeapFree",
        "memory",
        "partial",
        "process heap blocks; foreign pointers fail",
        heap::release
    ),
    implemented!(
        "HeapSize",
        "memory",
        "partial",
        "requested size of process heap blocks",
        heap::size
    ),
    implemented!(
        "HeapReAlloc",
        "memory",
        "partial",
        "zero-fill and in-place-only; exceptions unavailable",
        heap::reallocate
    ),
    implemented!(
        "HeapQueryInformation",
        "memory",
        "partial",
        "HeapCompatibilityInformation (LFH)",
        heap::query_information
    ),
    implemented!(
        "EncodePointer",
        "system",
        "implemented",
        "per-process random cookie and rotation",
        pointers::encode
    ),
    implemented!(
        "DecodePointer",
        "system",
        "implemented",
        "inverse of EncodePointer",
        pointers::decode
    ),
    implemented!(
        "EncodeSystemPointer",
        "system",
        "implemented",
        "same as EncodePointer",
        pointers::encode
    ),
    implemented!(
        "DecodeSystemPointer",
        "system",
        "implemented",
        "same as DecodePointer",
        pointers::decode
    ),
    implemented!(
        "GetTickCount",
        "system",
        "implemented",
        "milliseconds since boot (CLOCK_BOOTTIME)",
        time::tick_count
    ),
    implemented!(
        "GetTickCount64",
        "system",
        "implemented",
        "milliseconds since boot (CLOCK_BOOTTIME)",
        time::tick_count64
    ),
    implemented!(
        "GetSystemTimePreciseAsFileTime",
        "system",
        "implemented",
        "host UTC with 100 ns resolution",
        time::precise_file_time
    ),
    implemented!(
        "GetSystemTime",
        "system",
        "implemented",
        "host UTC as SYSTEMTIME",
        time::system_time
    ),
    implemented!(
        "GetLocalTime",
        "system",
        "implemented",
        "host local time as SYSTEMTIME",
        time::local_time
    ),
    implemented!(
        "FileTimeToSystemTime",
        "system",
        "implemented",
        "UTC conversion with Windows validation",
        time::file_time_to_system_time
    ),
    implemented!(
        "SystemTimeToFileTime",
        "system",
        "implemented",
        "UTC conversion with Windows validation",
        time::system_time_to_file_time
    ),
    implemented!(
        "GetEnvironmentVariableA",
        "system",
        "implemented",
        "host environment, case-insensitive names",
        environment::get_a
    ),
    implemented!(
        "GetEnvironmentVariableW",
        "system",
        "implemented",
        "host environment, case-insensitive names",
        environment::get_w
    ),
    implemented!(
        "SetEnvironmentVariableA",
        "system",
        "implemented",
        "process environment",
        environment::set_a
    ),
    implemented!(
        "SetEnvironmentVariableW",
        "system",
        "implemented",
        "process environment",
        environment::set_w
    ),
    implemented!(
        "GetEnvironmentStringsW",
        "system",
        "implemented",
        "snapshot block of the host environment",
        environment::strings_w
    ),
    implemented!(
        "FreeEnvironmentStringsW",
        "system",
        "implemented",
        "blocks from GetEnvironmentStringsW",
        environment::free_strings_w
    ),
    implemented!(
        "ExpandEnvironmentStringsW",
        "system",
        "implemented",
        "%NAME% from the process environment",
        environment::expand_w
    ),
    implemented!(
        "GetLocaleInfoW",
        "system",
        "partial",
        "user locale only; name, language, country, separators, measure",
        locale::info_w
    ),
    implemented!(
        "GetLocaleInfoEx",
        "system",
        "partial",
        "user locale only; name, language, country, separators, measure",
        locale::info_ex
    ),
    implemented!(
        "GetUserDefaultLCID",
        "system",
        "implemented",
        "LANGID of the host locale",
        locale::user_default_lcid
    ),
    implemented!(
        "GetUserDefaultLocaleName",
        "system",
        "implemented",
        "host locale as a Windows name",
        locale::user_default_locale_name
    ),
    implemented!(
        "GetSystemDefaultLocaleName",
        "system",
        "implemented",
        "host locale as a Windows name",
        locale::user_default_locale_name
    ),
    implemented!(
        "IsValidLocale",
        "system",
        "partial",
        "the user locale and en-US",
        locale::is_valid_locale
    ),
    implemented!(
        "EnumSystemLocalesW",
        "system",
        "partial",
        "the user locale only",
        locale::enum_system_locales_w
    ),
    implemented!(
        "IsValidCodePage",
        "system",
        "partial",
        "UTF-8 and 1252",
        locale::is_valid_code_page
    ),
    implemented!(
        "GetCPInfo",
        "system",
        "partial",
        "UTF-8 and 1252",
        locale::cp_info
    ),
    implemented!(
        "GetConsoleOutputCP",
        "system",
        "implemented",
        "UTF-8 terminal",
        locale::console_output_cp
    ),
    implemented!(
        "GetStringTypeW",
        "system",
        "partial",
        "CT_CTYPE1 from Unicode properties",
        locale::string_type_w
    ),
    implemented!(
        "LCMapStringW",
        "system",
        "partial",
        "upper/lower case mapping only",
        locale::lcmap_w
    ),
    implemented!(
        "LCMapStringEx",
        "system",
        "partial",
        "upper/lower case mapping only",
        locale::lcmap_ex
    ),
    implemented!(
        "CompareStringW",
        "system",
        "partial",
        "ordinal UTF-16 order, optional case folding",
        locale::compare_w
    ),
    implemented!(
        "CompareStringEx",
        "system",
        "partial",
        "ordinal UTF-16 order, optional case folding",
        locale::compare_ex
    ),
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
        "main image, PeekGame DLLs or loaded PE modules; never loads",
        modules::get_module_handle_a
    ),
    implemented!(
        "GetModuleHandleExW",
        "system",
        "partial",
        "by name, NULL or address inside a loaded PE module; pin and reference rules",
        modules::get_module_handle_ex_w
    ),
    implemented!(
        "GetModuleHandleExA",
        "system",
        "partial",
        "by name, NULL or address inside a loaded PE module; pin and reference rules",
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
        "PeekGame DLLs, then program PE DLLs (EXE dir, SetDllDirectory, current dir) with dependencies, TLS and DllMain",
        dynamic::load_library_a
    ),
    implemented!(
        "LoadLibraryW",
        "system",
        "partial",
        "PeekGame DLLs, then program PE DLLs (EXE dir, SetDllDirectory, current dir) with dependencies, TLS and DllMain",
        dynamic::load_library_w
    ),
    implemented!(
        "LoadLibraryExA",
        "system",
        "partial",
        "as LoadLibraryA; SEARCH_SYSTEM32 means PeekGame DLLs only; data-file loads unsupported",
        dynamic::load_library_ex_a
    ),
    implemented!(
        "LoadLibraryExW",
        "system",
        "partial",
        "as LoadLibraryW; SEARCH_SYSTEM32 means PeekGame DLLs only; data-file loads unsupported",
        dynamic::load_library_ex_w
    ),
    implemented!(
        "FreeLibrary",
        "system",
        "implemented",
        "PE module reference counts, DLL_PROCESS_DETACH and unmapping; PeekGame DLLs stay loaded",
        dynamic::free_library
    ),
    implemented!(
        "DisableThreadLibraryCalls",
        "system",
        "implemented",
        "stops DLL_THREAD_ATTACH/DETACH calls to the module's DllMain",
        dynamic::disable_thread_library_calls
    ),
    implemented!(
        "SetDllDirectoryA",
        "system",
        "implemented",
        "directory searched after the EXE's; replaces the current directory",
        dynamic::set_dll_directory_a
    ),
    implemented!(
        "SetDllDirectoryW",
        "system",
        "implemented",
        "directory searched after the EXE's; replaces the current directory",
        dynamic::set_dll_directory_w
    ),
    implemented!(
        "GetProcAddress",
        "system",
        "partial",
        "PeekGame DLLs by name; PE modules by name, ordinal and forwarder; logs requests",
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
        "main image, PeekGame DLLs or loaded PE modules; never loads",
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
}
