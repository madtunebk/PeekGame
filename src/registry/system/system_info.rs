//! Export declarations for system info. Implementation stays in api/.
use crate::api::system::cpu;
use crate::api::system::info;
use crate::api::system::psapi;
use crate::api::system::toolhelp;
use crate::registry::Export;

exports! {
    implemented!(
        "SetThreadErrorMode",
        "system",
        "partial",
        "recorded per thread",
        info::set_thread_error_mode
    ),
    implemented!(
        "GetThreadErrorMode",
        "system",
        "partial",
        "recorded per thread",
        info::get_thread_error_mode
    ),
    implemented!(
        "SetThreadStackGuarantee",
        "threading",
        "partial",
        "recorded per thread",
        info::set_thread_stack_guarantee
    ),
    implemented!(
        "GetTempPath2W",
        "filesystem",
        "implemented",
        "same as GetTempPathW",
        info::temp_path_w
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
        "GetPerformanceInfo",
        "system",
        "partial",
        "/proc/meminfo figures",
        psapi::performance_info
    ),
    export!(
        "ntdll.dll",
        "NtQuerySystemInformation",
        "system",
        "partial",
        "basic, performance, time of day and processor accounting classes",
        crate::api::system::nt_info::query_system_information
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
        "GlobalMemoryStatus",
        "system",
        "implemented",
        "MEMORYSTATUS from the GlobalMemoryStatusEx values",
        crate::api::system::info::global_memory_status
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
        "SetThreadGroupAffinity",
        "threading",
        "partial",
        "single processor group 0; returns usable GROUP_AFFINITY",
        info::set_thread_group_affinity
    ),
    implemented!(
        "SetThreadPriorityBoost",
        "threading",
        "partial",
        "accepted; host scheduling unchanged",
        info::set_thread_priority_boost
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
        "GetThreadDescription",
        "threading",
        "partial",
        "returns stored UTF-16 description in a LocalFree-owned buffer",
        info::get_thread_description
    ),
    implemented!(
        "SetThreadInformation",
        "threading",
        "partial",
        "validates thread and information class; unsupported Linux policy returns ERROR_NOT_SUPPORTED",
        info::set_thread_information
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
        "GetWindowsDirectoryA",
        "filesystem",
        "partial",
        "conventional path; nothing exists there",
        info::windows_directory_a
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
        "IsProcessorFeaturePresent",
        "system",
        "partial",
        "host CPU features; unsupported OS facilities and unmapped features absent",
        cpu::present
    ),
    implemented!(
        "GetLogicalProcessorInformation",
        "system",
        "partial",
        "processor cores and L1/L2/L3 caches",
        info::logical_processor_information
    ),
}
