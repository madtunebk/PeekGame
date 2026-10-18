//! Export declarations for diagnostics. Implementation stays in api/.
use crate::api::system::debug;
use crate::api::system::etw;
use crate::api::system::message;
use crate::registry::Export;

exports! {
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
    export!("advapi32.dll", "RegisterTraceGuidsW", "diagnostics", "partial", "no ETW: ERROR_NOT_SUPPORTED, null trace handle", etw::register_trace_guids_w),
    export!("advapi32.dll", "UnregisterTraceGuids", "diagnostics", "partial", "no ETW registration: ERROR_INVALID_HANDLE", etw::unregister_trace_guids),
    export!("advapi32.dll", "GetTraceLoggerHandle", "diagnostics", "partial", "no ETW logger: invalid handle", etw::get_trace_logger_handle),
    export!("advapi32.dll", "GetTraceEnableLevel", "diagnostics", "partial", "no ETW logger: level 0 and ERROR_INVALID_HANDLE", etw::get_trace_enable_level),
    export!("advapi32.dll", "GetTraceEnableFlags", "diagnostics", "partial", "no ETW logger: flags 0 and ERROR_INVALID_HANDLE", etw::get_trace_enable_flags),
    export!("advapi32.dll", "TraceMessage", "diagnostics", "partial", "no ETW logger: ERROR_INVALID_HANDLE", etw::trace_message),
    implemented!(
        "AreFileApisANSI",
        "system",
        "implemented",
        "ANSI is UTF-8",
        etw::file_apis_ansi
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
        "OutputDebugStringA",
        "system",
        "partial",
        "debug text goes to Linux stderr; Windows debugger delivery unavailable",
        debug::output_debug_string_a
    ),
    export!(
        "dbghelp.dll",
        "SymGetOptions",
        "diagnostics",
        "implemented",
        "process symbol option mask",
        debug::sym_get_options
    ),
    export!(
        "dbghelp.dll",
        "SymSetOptions",
        "diagnostics",
        "implemented",
        "process symbol option mask",
        debug::sym_set_options
    ),
    export!(
        "dbghelp.dll",
        "SymInitialize",
        "diagnostics",
        "partial",
        "accepted; symbol loading no-op",
        debug::sym_initialize
    ),
    export!(
        "dbghelp.dll",
        "SymInitializeW",
        "diagnostics",
        "partial",
        "accepted; symbol loading no-op",
        debug::sym_initialize_w
    ),
    export!(
        "dbghelp.dll",
        "SymCleanup",
        "diagnostics",
        "partial",
        "accepted",
        debug::sym_cleanup
    ),
    export!(
        "dbghelp.dll",
        "SymRefreshModuleList",
        "diagnostics",
        "partial",
        "accepted; module refresh no-op",
        debug::sym_refresh_module_list
    ),
    export!(
        "dbghelp.dll",
        "StackWalk64",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::stack_walk64
    ),
    export!(
        "dbghelp.dll",
        "SymFunctionTableAccess64",
        "diagnostics",
        "partial",
        "stub; returns 0",
        debug::sym_function_table_access64
    ),
    export!(
        "dbghelp.dll",
        "SymGetModuleBase64",
        "diagnostics",
        "partial",
        "stub; returns 0",
        debug::sym_get_module_base64
    ),
    export!(
        "dbghelp.dll",
        "SymFromAddr",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::sym_from_addr
    ),
    export!(
        "dbghelp.dll",
        "SymFromAddrW",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::sym_from_addr_w
    ),
    export!(
        "dbghelp.dll",
        "SymGetLineFromAddr64",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::sym_get_line_from_addr64
    ),
    export!(
        "dbghelp.dll",
        "SymGetLineFromAddrW64",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::sym_get_line_from_addr_w64
    ),
    export!(
        "dbghelp.dll",
        "SymGetModuleInfo64",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::sym_get_module_info64
    ),
    export!(
        "dbghelp.dll",
        "SymGetModuleInfoW64",
        "diagnostics",
        "partial",
        "stub; returns FALSE",
        debug::sym_get_module_info_w64
    ),
    export!(
        "dbghelp.dll",
        "SymLoadModuleEx",
        "diagnostics",
        "partial",
        "stub; returns 0",
        debug::sym_load_module_ex
    ),
    export!(
        "dbghelp.dll",
        "SymLoadModuleExW",
        "diagnostics",
        "partial",
        "stub; returns 0",
        debug::sym_load_module_ex_w
    ),
    export!(
        "dbghelp.dll",
        "SymSetHomeDirectory",
        "diagnostics",
        "partial",
        "stub",
        debug::sym_set_home_directory
    ),
    export!(
        "dbghelp.dll",
        "SymSetHomeDirectoryW",
        "diagnostics",
        "partial",
        "stub",
        debug::sym_set_home_directory_w
    ),
    export!(
        "dbghelp.dll",
        "MiniDumpWriteDump",
        "diagnostics",
        "partial",
        "minidump stub",
        debug::minidump_write_dump
    ),
    export!(
        "dbghelp.dll",
        "ImageNtHeader",
        "diagnostics",
        "partial",
        "returns PE NT header",
        debug::image_nt_header
    ),
}

