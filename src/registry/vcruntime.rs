//! Explicit VCRUNTIME exports; never redirect its entire namespace to MSVCRT.
use super::Export;
exports! {
    export!("vcruntime140.dll", "memcpy", "crt", "partial", "Win64 adapter; C SEH supported, C++ exception handlers not implemented", crate::api::crt::simple::call_4),
    export!("vcruntime140.dll", "memmove", "crt", "partial", "Win64 adapter; C SEH supported, C++ exception handlers not implemented", crate::api::crt::simple::call_5),
    export!("vcruntime140.dll", "memset", "crt", "partial", "Win64 adapter; C SEH supported, C++ exception handlers not implemented", crate::api::crt::simple::call_6),
    export!("vcruntime140.dll", "memcmp", "crt", "partial", "Win64 adapter; C SEH supported, C++ exception handlers not implemented", crate::api::crt::simple::call_7),
    export!("vcruntime140.dll", "__C_specific_handler", "crt", "partial", "Win64 adapter; C SEH supported, C++ exception handlers not implemented", crate::seh::c_specific_handler),
    export!("vcruntime140.dll", "_CxxThrowException", "crt", "partial", "MSVC C++ exception raise adapter", crate::seh::cxx::cxx_throw_exception),
    export!("vcruntime140.dll", "__CxxFrameHandler3", "crt", "partial", "MSVC C++ frame personality routine", crate::seh::cxx::cxx_frame_handler_3),
    export!("vcruntime140.dll", "__current_exception", "crt", "partial", "thread-local current exception record pointer", crate::seh::cxx::current_exception),
    export!("vcruntime140.dll", "__current_exception_context", "crt", "partial", "thread-local current exception context pointer", crate::seh::cxx::current_exception_context)
}
