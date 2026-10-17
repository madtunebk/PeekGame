//! Export declarations for loader. Implementation stays in api/.
use crate::api::system::dynamic;
use crate::api::system::modules;
use crate::registry::Export;

exports! {
    implemented!(
        "AddDllDirectory",
        "system",
        "partial",
        "searched after the DLL directory",
        dynamic::add_dll_directory
    ),
    implemented!(
        "RemoveDllDirectory",
        "system",
        "partial",
        "by cookie",
        dynamic::remove_dll_directory
    ),
    implemented!(
        "SetDefaultDllDirectories",
        "system",
        "partial",
        "flags accepted; search order unchanged",
        dynamic::set_default_dll_directories
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
        "GetModuleHandleW",
        "system",
        "partial",
        "main image, PeekGame DLLs or loaded PE modules; never loads",
        modules::get_module_handle_w
    ),
}
