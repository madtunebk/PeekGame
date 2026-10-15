use super::*;
use crate::api::memory::{heap, virtual_memory};
use crate::api::system::console;
use crate::api::system::cpu;
use crate::api::system::dynamic;
use crate::api::system::encoding;
use crate::api::system::etw;
use crate::api::system::message;
use crate::api::system::psapi;
use crate::api::system::random;
use crate::api::system::security;
use crate::api::system::startup;
use crate::api::system::toolhelp;
use crate::api::system::{environment, locale, pointers, time};
use crate::api::{memory::global, system::info};
use crate::seh;
exports! {
    implemented!(
        "RaiseException",
        "system",
        "partial",
        "vectored handlers, frame handlers through the unwind tables, then the unhandled-exception filter",
        seh::raise_exception_entry
    ),
    implemented!(
        "RtlUnwindEx",
        "system",
        "partial",
        "termination handlers up to the target frame, then continues there",
        seh::unwind_ex_entry
    ),
    implemented!(
        "RtlUnwind",
        "system",
        "partial",
        "as RtlUnwindEx",
        seh::unwind_entry
    ),
    implemented!(
        "RtlCaptureContext",
        "system",
        "partial",
        "integer, flags, segment and FPU/SSE state",
        seh::capture_context_entry
    ),
    implemented!(
        "RtlRestoreContext",
        "system",
        "partial",
        "integer, flags and FPU/SSE state",
        seh::restore_context_entry
    ),
    implemented!(
        "RtlVirtualUnwind",
        "system",
        "partial",
        "UNWIND_INFO v1/v2, chained info and epilogs",
        seh::unwind::virtual_unwind
    ),
    implemented!(
        "RtlLookupFunctionEntry",
        "system",
        "partial",
        "registered tables, then the image's .pdata",
        seh::tables::lookup
    ),
    implemented!(
        "RtlAddFunctionTable",
        "system",
        "partial",
        "caller-owned sorted array",
        seh::tables::add
    ),
    implemented!(
        "RtlDeleteFunctionTable",
        "system",
        "partial",
        "arrays and callback tables",
        seh::tables::delete
    ),
    implemented!(
        "RtlInstallFunctionTableCallback",
        "system",
        "partial",
        "callback asked per pc in its range",
        seh::tables::install_callback
    ),
    implemented!(
        "RtlAddGrowableFunctionTable",
        "system",
        "partial",
        "sorted table grown in place",
        seh::tables::add_growable
    ),
    implemented!(
        "RtlGrowFunctionTable",
        "system",
        "partial",
        "extends the entry count",
        seh::tables::grow
    ),
    implemented!(
        "RtlDeleteGrowableFunctionTable",
        "system",
        "partial",
        "removes the table",
        seh::tables::delete_growable
    ),
    implemented!(
        "RtlPcToFileHeader",
        "system",
        "partial",
        "loaded PE images",
        seh::tables::pc_to_file_header
    ),
    implemented!(
        "AddVectoredExceptionHandler",
        "system",
        "partial",
        "first and last lists",
        seh::add_vectored_handler
    ),
    implemented!(
        "RemoveVectoredExceptionHandler",
        "system",
        "partial",
        "by registration handle",
        seh::remove_vectored_handler
    ),
    implemented!(
        "__C_specific_handler",
        "system",
        "partial",
        "__try/__except/__finally scope tables",
        seh::c_specific_handler
    ),
    export!(
        "ntdll.dll",
        "RtlUnwindEx",
        "system",
        "partial",
        "termination handlers up to the target frame, then continues there",
        seh::unwind_ex_entry
    ),
    export!(
        "ntdll.dll",
        "RtlUnwind",
        "system",
        "partial",
        "as RtlUnwindEx",
        seh::unwind_entry
    ),
    export!(
        "ntdll.dll",
        "RtlCaptureContext",
        "system",
        "partial",
        "integer, flags, segment and FPU/SSE state",
        seh::capture_context_entry
    ),
    export!(
        "ntdll.dll",
        "RtlRestoreContext",
        "system",
        "partial",
        "integer, flags and FPU/SSE state",
        seh::restore_context_entry
    ),
    export!(
        "ntdll.dll",
        "RtlVirtualUnwind",
        "system",
        "partial",
        "UNWIND_INFO v1/v2, chained info and epilogs",
        seh::unwind::virtual_unwind
    ),
    export!(
        "ntdll.dll",
        "RtlLookupFunctionEntry",
        "system",
        "partial",
        "registered tables, then the image's .pdata",
        seh::tables::lookup
    ),
    export!(
        "ntdll.dll",
        "RtlAddFunctionTable",
        "system",
        "partial",
        "caller-owned sorted array",
        seh::tables::add
    ),
    export!(
        "ntdll.dll",
        "RtlDeleteFunctionTable",
        "system",
        "partial",
        "arrays and callback tables",
        seh::tables::delete
    ),
    export!(
        "ntdll.dll",
        "RtlInstallFunctionTableCallback",
        "system",
        "partial",
        "callback asked per pc in its range",
        seh::tables::install_callback
    ),
    export!(
        "ntdll.dll",
        "RtlAddGrowableFunctionTable",
        "system",
        "partial",
        "sorted table grown in place",
        seh::tables::add_growable
    ),
    export!(
        "ntdll.dll",
        "RtlGrowFunctionTable",
        "system",
        "partial",
        "extends the entry count",
        seh::tables::grow
    ),
    export!(
        "ntdll.dll",
        "RtlDeleteGrowableFunctionTable",
        "system",
        "partial",
        "removes the table",
        seh::tables::delete_growable
    ),
    export!(
        "ntdll.dll",
        "RtlPcToFileHeader",
        "system",
        "partial",
        "loaded PE images",
        seh::tables::pc_to_file_header
    ),
    export!(
        "ntdll.dll",
        "RtlAddVectoredExceptionHandler",
        "system",
        "partial",
        "first and last lists",
        seh::add_vectored_handler
    ),
    export!(
        "ntdll.dll",
        "RtlRemoveVectoredExceptionHandler",
        "system",
        "partial",
        "by registration handle",
        seh::remove_vectored_handler
    ),
    export!(
        "ntdll.dll",
        "__C_specific_handler",
        "system",
        "partial",
        "__try/__except/__finally scope tables",
        seh::c_specific_handler
    ),
    implemented!(
        "UnhandledExceptionFilter",
        "system",
        "partial",
        "the top-level filter, else execute handler",
        seh::unhandled_exception_filter
    ),
    export!(
        "msvcrt.dll",
        "__C_specific_handler",
        "crt",
        "partial",
        "__try/__except/__finally scope tables",
        seh::c_specific_handler
    ),
    export!(
        "advapi32.dll",
        "EventRegister",
        "diagnostics",
        "implemented",
        "no ETW: ERROR_NOT_SUPPORTED, null handle",
        etw::register
    ),
    export!(
        "advapi32.dll",
        "EventUnregister",
        "diagnostics",
        "implemented",
        "no ETW",
        etw::unregister
    ),
    export!(
        "advapi32.dll",
        "EventWrite",
        "diagnostics",
        "implemented",
        "no ETW: invalid handle",
        etw::write
    ),
    export!(
        "advapi32.dll",
        "EventWriteTransfer",
        "diagnostics",
        "implemented",
        "no ETW: invalid handle",
        etw::write_transfer
    ),
    export!(
        "advapi32.dll",
        "EventSetInformation",
        "diagnostics",
        "implemented",
        "no ETW: invalid handle",
        etw::set_information
    ),
    export!(
        "advapi32.dll",
        "EventEnabled",
        "diagnostics",
        "implemented",
        "never enabled",
        etw::enabled
    ),
    export!(
        "advapi32.dll",
        "EventProviderEnabled",
        "diagnostics",
        "implemented",
        "never enabled",
        etw::provider_enabled
    ),
    implemented!(
        "AreFileApisANSI",
        "system",
        "implemented",
        "ANSI is UTF-8",
        etw::file_apis_ansi
    ),
    implemented!(
        "GetTempPath2W",
        "filesystem",
        "implemented",
        "same as GetTempPathW",
        info::temp_path_w
    ),
    implemented!(
        "FormatMessageW",
        "system",
        "partial",
        "system texts for common errors; plain FROM_STRING",
        message::format_w
    ),
    implemented!(
        "FormatMessageA",
        "system",
        "partial",
        "system texts for common errors; plain FROM_STRING",
        message::format_a
    ),
    implemented!(
        "K32EnumProcessModules",
        "system",
        "implemented",
        "loaded PE modules",
        psapi::enum_process_modules
    ),
    implemented!(
        "K32EnumProcessModulesEx",
        "system",
        "implemented",
        "loaded PE modules",
        psapi::enum_process_modules_ex
    ),
    implemented!(
        "K32GetModuleInformation",
        "system",
        "implemented",
        "base, size, entry",
        psapi::module_information
    ),
    implemented!(
        "K32GetModuleFileNameExW",
        "system",
        "implemented",
        "current process",
        psapi::module_file_name_ex_w
    ),
    implemented!(
        "K32GetModuleFileNameExA",
        "system",
        "implemented",
        "current process",
        psapi::module_file_name_ex_a
    ),
    implemented!(
        "K32GetModuleBaseNameW",
        "system",
        "implemented",
        "current process",
        psapi::module_base_name_w
    ),
    implemented!(
        "K32GetModuleBaseNameA",
        "system",
        "implemented",
        "current process",
        psapi::module_base_name_a
    ),
    implemented!(
        "K32GetProcessMemoryInfo",
        "system",
        "partial",
        "/proc/self/status figures",
        psapi::process_memory_info
    ),
    implemented!(
        "K32GetPerformanceInfo",
        "system",
        "partial",
        "/proc/meminfo figures",
        psapi::performance_info
    ),
    export!(
        "psapi.dll",
        "EnumProcessModules",
        "system",
        "implemented",
        "loaded PE modules",
        psapi::enum_process_modules
    ),
    export!(
        "psapi.dll",
        "EnumProcessModulesEx",
        "system",
        "implemented",
        "loaded PE modules",
        psapi::enum_process_modules_ex
    ),
    export!(
        "psapi.dll",
        "GetModuleInformation",
        "system",
        "implemented",
        "base, size, entry",
        psapi::module_information
    ),
    export!(
        "psapi.dll",
        "GetModuleFileNameExW",
        "system",
        "implemented",
        "current process",
        psapi::module_file_name_ex_w
    ),
    export!(
        "psapi.dll",
        "GetModuleBaseNameW",
        "system",
        "implemented",
        "current process",
        psapi::module_base_name_w
    ),
    export!(
        "psapi.dll",
        "GetProcessMemoryInfo",
        "system",
        "partial",
        "/proc/self/status figures",
        psapi::process_memory_info
    ),
    export!(
        "advapi32.dll",
        "OpenProcessToken",
        "security",
        "partial",
        "current process, query access",
        security::open_process_token
    ),
    export!(
        "advapi32.dll",
        "OpenThreadToken",
        "security",
        "implemented",
        "no impersonation: ERROR_NO_TOKEN",
        security::open_thread_token
    ),
    export!(
        "advapi32.dll",
        "GetTokenInformation",
        "security",
        "partial",
        "elevation and integrity level (root is elevated)",
        security::token_information
    ),
    export!(
        "advapi32.dll",
        "GetSidSubAuthority",
        "security",
        "implemented",
        "SID layout",
        security::sid_sub_authority
    ),
    export!(
        "advapi32.dll",
        "GetSidSubAuthorityCount",
        "security",
        "implemented",
        "SID layout",
        security::sid_sub_authority_count
    ),
    export!(
        "advapi32.dll",
        "AllocateLocallyUniqueId",
        "security",
        "implemented",
        "unique for the process",
        security::allocate_luid
    ),
    export!(
        "advapi32.dll",
        "GetUserNameA",
        "security",
        "implemented",
        "effective Linux account",
        security::user_name_a
    ),
    implemented!(
        "CreateToolhelp32Snapshot",
        "system",
        "partial",
        "process (/proc) and own-thread snapshots",
        toolhelp::create_snapshot
    ),
    implemented!(
        "Process32First",
        "system",
        "implemented",
        "snapshot processes",
        toolhelp::process32_first
    ),
    implemented!(
        "Process32Next",
        "system",
        "implemented",
        "snapshot processes",
        toolhelp::process32_next
    ),
    implemented!(
        "Process32FirstW",
        "system",
        "implemented",
        "snapshot processes",
        toolhelp::process32_first_w
    ),
    implemented!(
        "Process32NextW",
        "system",
        "implemented",
        "snapshot processes",
        toolhelp::process32_next_w
    ),
    implemented!(
        "Thread32First",
        "system",
        "implemented",
        "threads of this process",
        toolhelp::thread32_first
    ),
    implemented!(
        "Thread32Next",
        "system",
        "implemented",
        "threads of this process",
        toolhelp::thread32_next
    ),
    export!(
        "bcrypt.dll",
        "BCryptGenRandom",
        "crypto",
        "implemented",
        "host CSPRNG (getrandom)",
        random::bcrypt_gen_random
    ),
    export!(
        "bcryptprimitives.dll",
        "ProcessPrng",
        "crypto",
        "implemented",
        "host CSPRNG (getrandom)",
        random::process_prng
    ),
    implemented!(
        "SetErrorMode",
        "system",
        "implemented",
        "process error mode bits",
        info::set_error_mode
    ),
    implemented!(
        "GetErrorMode",
        "system",
        "implemented",
        "process error mode bits",
        info::get_error_mode
    ),
    implemented!(
        "GetSystemInfo",
        "system",
        "implemented",
        "real CPU set, page size, 64 KiB granularity, cpuid",
        info::system_info
    ),
    implemented!(
        "GetNativeSystemInfo",
        "system",
        "implemented",
        "same as GetSystemInfo (x64 process)",
        info::system_info
    ),
    implemented!(
        "GlobalMemoryStatusEx",
        "system",
        "implemented",
        "/proc/meminfo and process size",
        info::global_memory_status_ex
    ),
    implemented!(
        "GetLogicalProcessorInformationEx",
        "system",
        "partial",
        "cores, packages, single NUMA node, one group (64 CPUs)",
        info::logical_processor_information_ex
    ),
    implemented!(
        "SetThreadPriority",
        "threading",
        "partial",
        "accepted; host scheduling unchanged",
        info::set_thread_priority
    ),
    implemented!(
        "SetThreadAffinityMask",
        "threading",
        "partial",
        "returns the usable CPU mask; host scheduling unchanged",
        info::set_thread_affinity_mask
    ),
    implemented!(
        "SwitchToThread",
        "threading",
        "implemented",
        "sched_yield",
        info::switch_to_thread
    ),
    implemented!(
        "SleepEx",
        "threading",
        "partial",
        "no APC delivery",
        info::sleep_ex
    ),
    implemented!(
        "SetThreadDescription",
        "threading",
        "partial",
        "calling thread's Linux name",
        info::set_thread_description
    ),
    implemented!(
        "GetTempPathW",
        "filesystem",
        "implemented",
        "TMP, TEMP, TMPDIR or /tmp on Z:",
        info::temp_path_w
    ),
    implemented!(
        "GetComputerNameW",
        "system",
        "implemented",
        "host name first label, uppercased",
        info::computer_name_w
    ),
    implemented!(
        "GetSystemDirectoryW",
        "filesystem",
        "partial",
        "conventional path; nothing exists there",
        info::system_directory_w
    ),
    implemented!(
        "GetSystemDirectoryA",
        "filesystem",
        "partial",
        "conventional path; nothing exists there",
        info::system_directory_a
    ),
    implemented!(
        "GetWindowsDirectoryW",
        "filesystem",
        "partial",
        "conventional path; nothing exists there",
        info::windows_directory_w
    ),
    implemented!(
        "GetCurrentDirectoryA",
        "filesystem",
        "implemented",
        "host current directory on Z:",
        info::current_directory_a
    ),
    implemented!(
        "OutputDebugStringW",
        "system",
        "partial",
        "text goes to stderr",
        info::output_debug_string_w
    ),
    implemented!(
        "GetModuleFileNameA",
        "filesystem",
        "partial",
        "EXE and loaded PE modules, Z: paths",
        info::module_file_name_a
    ),
    implemented!(
        "GlobalAlloc",
        "memory",
        "partial",
        "fixed and moveable blocks",
        global::alloc
    ),
    implemented!(
        "GlobalLock",
        "memory",
        "implemented",
        "lock count",
        global::lock
    ),
    implemented!(
        "GlobalUnlock",
        "memory",
        "implemented",
        "lock count",
        global::unlock
    ),
    implemented!(
        "GlobalFree",
        "memory",
        "implemented",
        "unlocked blocks",
        global::free
    ),
    implemented!(
        "GlobalSize",
        "memory",
        "implemented",
        "block size",
        global::size
    ),
    implemented!(
        "VirtualAlloc",
        "memory",
        "partial",
        "reserve/commit as anonymous mappings on 64 KiB boundaries",
        virtual_memory::alloc
    ),
    implemented!(
        "VirtualFree",
        "memory",
        "partial",
        "release whole reservations, decommit pages",
        virtual_memory::free
    ),
    implemented!(
        "VirtualProtect",
        "memory",
        "implemented",
        "mprotect with the previous protection from /proc/self/maps",
        virtual_memory::protect
    ),
    implemented!(
        "VirtualQuery",
        "memory",
        "partial",
        "regions from /proc/self/maps; image and reservation bases",
        virtual_memory::query
    ),
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
        "called for exceptions no vectored or frame handler takes",
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
