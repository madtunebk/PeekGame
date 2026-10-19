//! Export declarations for process. Implementation stays in api/.
use crate::api::system::command_line;
use crate::api::system::environment;
use crate::api::system::error;
use crate::api::system::process;
use crate::api::system::standard_handles;
use crate::api::system::startup;
use crate::registry::Export;

exports! {
    implemented!("GetProcessId", "system", "partial", "current-process handle only; others ERROR_INVALID_HANDLE", process::get_process_id),
    implemented!("FlushInstructionCache", "system", "implemented", "coherent hardware no-op", process::flush_instruction_cache),

    export!("ntdll.dll", "RtlDllShutdownInProgress", "system", "implemented", "actual process ExitProcess shutdown state", process::dll_shutdown_in_progress),
    export!("ktmw32.dll", "CreateTransaction", "system", "partial", "unsupported transactions: INVALID_HANDLE_VALUE and ERROR_NOT_SUPPORTED", crate::api::system::transactions::create),
    export!("ktmw32.dll", "CommitTransaction", "system", "partial", "no valid transaction handles: ERROR_INVALID_HANDLE", crate::api::system::transactions::commit),
    export!("advapi32.dll", "RegCreateKeyTransactedW", "system", "partial", "unsupported transactions: ERROR_NOT_SUPPORTED", crate::api::system::transactions::create_key),
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
    export!(
        "userenv.dll",
        "RegisterGPNotification",
        "system",
        "implemented",
        "group policy change notification stub",
        environment::register_gp_notification
    ),
    export!(
        "userenv.dll",
        "UnregisterGPNotification",
        "system",
        "implemented",
        "group policy change notification unregister stub",
        environment::unregister_gp_notification
    ),
    export!(
        "userenv.dll",
        "GetUserProfileDirectoryW",
        "system",
        "implemented",
        "user profile directory",
        environment::get_user_profile_directory_w
    ),
    export!(
        "userenv.dll",
        "GetUserProfileDirectoryA",
        "system",
        "implemented",
        "user profile directory",
        environment::get_user_profile_directory_a
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
        "GetLastError",
        "system",
        "implemented",
        "per-thread Win32 error state; TEB not provided yet; PEEKGAME_TRACE_LAST_ERROR logs nonzero reads",
        error::get_last_error_traced
    ),
    implemented!(
        "GetCommandLineA",
        "system",
        "implemented",
        "loader-supplied command line; requires process configuration",
        command_line::get_command_line_a
    ),
    implemented!(
        "GetStdHandle",
        "system",
        "partial",
        "inherited standard streams; requires process configuration",
        standard_handles::get_std_handle
    ),
    implemented!(
        "ExitProcess",
        "system",
        "partial",
        "terminates host process; PE DLL detach callbacks not implemented",
        process::exit_process
    ),
    implemented!("SetStdHandle", "system", "implemented", "thread-safe process standard handle replacement", standard_handles::set_std_handle),
    implemented!("InitializeProcThreadAttributeList", "system", "implemented", "size query and initialization of an attribute list", process::initialize_proc_thread_attribute_list),
    implemented!("UpdateProcThreadAttribute", "system", "implemented", "records attributes for process creation", process::update_proc_thread_attribute),
    implemented!("DeleteProcThreadAttributeList", "system", "implemented", "nothing to release", process::delete_proc_thread_attribute_list),
    implemented!("CreateProcessW", "system", "partial", "child processes unsupported: ERROR_NOT_SUPPORTED, command line logged", process::create_process_w),
    implemented!("CreateProcessA", "system", "partial", "child processes unsupported: ERROR_NOT_SUPPORTED, command line logged", process::create_process_a),
    implemented!("TerminateProcess", "system", "partial", "current process only; immediate termination without cleanup", process::terminate_process),
    implemented!("PowerCreateRequest", "system", "partial", "owned power-request handles; simple UTF-16 reason contexts", crate::api::system::power::create),
    implemented!("PowerSetRequest", "system", "partial", "validates request handles; host inhibition unavailable: ERROR_NOT_SUPPORTED", crate::api::system::power::set),
    implemented!("PowerClearRequest", "system", "partial", "validates request handles; host inhibition unavailable: ERROR_NOT_SUPPORTED", crate::api::system::power::clear),
    implemented!("GetSystemPowerStatus", "system", "implemented", "AC online, no battery", crate::api::system::power::get_system_power_status),
    implemented!("SetPriorityClass", "system", "partial", "current-process thread nice mapping; real host permission errors; realtime unsupported", crate::api::system::priority::set),
    implemented!("GetPriorityClass", "system", "partial", "current-process priority class from leader host nice value", crate::api::system::priority::get),
    implemented!("GetProcessTimes", "system", "partial", "current process user and kernel rusage times", process::get_process_times),
    implemented!("GetProcessIoCounters", "system", "partial", "current process I/O counters from /proc/self/io", process::get_process_io_counters),
    implemented!("GetExitCodeProcess", "system", "partial", "STILL_ACTIVE for current process handle", process::get_exit_code_process),
    implemented!("IsWow64Process", "system", "partial", "FALSE for exposed x64 process handles", process::is_wow64_process),
    implemented!("IsWow64Process2", "system", "partial", "not WOW64; native AMD64", process::is_wow64_process2),
    implemented!("GetProcessMitigationPolicy", "system", "partial", "all mitigation flags clear", process::get_process_mitigation_policy),
    implemented!("SetProcessMitigationPolicy", "system", "partial", "accepted, not enforced", process::set_process_mitigation_policy),
    implemented!("CreateJobObjectA", "system", "partial", "owned job object handle", process::create_job_object_a),
    implemented!("CreateJobObjectW", "system", "partial", "owned job object handle; name ignored", process::create_job_object_w),
    implemented!("SetInformationJobObject", "system", "partial", "accepts job limits/configuration", process::set_information_job_object),
    implemented!("AssignProcessToJobObject", "system", "partial", "accepts process assignment to job", process::assign_process_to_job_object),
    implemented!("GetEnvironmentStrings", "system", "implemented", "ANSI/UTF-8 snapshot block of the host environment", environment::strings_a),
    implemented!("GetEnvironmentStringsA", "system", "implemented", "ANSI/UTF-8 snapshot block of the host environment", environment::strings_a),
    implemented!("FreeEnvironmentStringsA", "system", "implemented", "blocks from GetEnvironmentStrings/GetEnvironmentStringsA", environment::free_strings_a),
    export!("ntdll.dll", "NtQueryInformationProcess", "system", "partial", "basic process info and debug port for current process", crate::api::system::nt_info::query_information_process),
    export!("ntdll.dll", "NtSetInformationThread", "system", "partial", "thread information and debugger hiding", crate::api::system::nt_info::set_information_thread),
    export!("ntdll.dll", "ZwSetInformationThread", "system", "partial", "thread information and debugger hiding", crate::api::system::nt_info::set_information_thread),
    export!("ntdll.dll", "NtQueryInformationThread", "system", "partial", "thread information queries", crate::api::system::nt_info::query_information_thread),
    export!("ntdll.dll", "ZwQueryInformationThread", "system", "partial", "thread information queries", crate::api::system::nt_info::query_information_thread),
}
