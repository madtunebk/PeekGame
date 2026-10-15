use super::Export;
use crate::api::com::apartment;
macro_rules! com {
    (@link $name:literal,$function:path $(,)?) => {
        link_symbol!("ole32.dll", $name, $function);
    };
    ($name:literal,$function:path $(,)?) => {
        Export {
            dll: "ole32.dll",
            name: $name,
            category: "com",
            status: "partial",
            scope: "logical COM apartments and task allocator; no registered COM classes",
            address: Some(|| $function as *const std::ffi::c_void),
        }
    };
}
exports! {
    com!("CoInitializeEx", apartment::initialize),
    com!("CoUninitialize", apartment::uninitialize),
    com!("OleInitialize", apartment::ole_initialize),
    com!("OleUninitialize", apartment::uninitialize),
    com!("CoCreateInstance", apartment::create_instance),
    com!("CoTaskMemAlloc", apartment::allocate),
    com!("CoTaskMemRealloc", apartment::reallocate),
    com!("CoTaskMemFree", apartment::release),
}
