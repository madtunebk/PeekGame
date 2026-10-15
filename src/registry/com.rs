use super::Export;
use crate::api::com::{apartment, bstr, winrt};
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
    export!(
        "oleaut32.dll",
        "SysAllocString",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::alloc_string
    ),
    export!(
        "oleaut32.dll",
        "SysAllocStringLen",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::alloc_string_len
    ),
    export!(
        "oleaut32.dll",
        "SysAllocStringByteLen",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::alloc_string_byte_len
    ),
    export!(
        "oleaut32.dll",
        "SysFreeString",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::free_string
    ),
    export!(
        "oleaut32.dll",
        "SysStringLen",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::string_len
    ),
    export!(
        "oleaut32.dll",
        "SysStringByteLen",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::string_byte_len
    ),
    export!(
        "oleaut32.dll",
        "SysReAllocString",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::realloc_string
    ),
    export!(
        "oleaut32.dll",
        "SysReAllocStringLen",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::realloc_string_len
    ),
    export!(
        "oleaut32.dll",
        "VariantInit",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::variant_init
    ),
    export!(
        "oleaut32.dll",
        "VariantClear",
        "com",
        "partial",
        "BSTR layout as on Windows",
        bstr::variant_clear
    ),
    export!(
        "ole32.dll",
        "RoInitialize",
        "com",
        "partial",
        "COM apartment for the thread; no Windows Runtime",
        winrt::ro_initialize
    ),
    export!(
        "ole32.dll",
        "RoUninitialize",
        "com",
        "partial",
        "leaves the COM apartment",
        apartment::uninitialize
    ),
    export!(
        "ole32.dll",
        "RoGetActivationFactory",
        "com",
        "partial",
        "no runtime classes: REGDB_E_CLASSNOTREG",
        winrt::ro_get_activation_factory
    ),
    export!(
        "ole32.dll",
        "RoActivateInstance",
        "com",
        "partial",
        "no runtime classes: REGDB_E_CLASSNOTREG",
        winrt::ro_activate_instance
    ),
    export!(
        "ole32.dll",
        "WindowsCreateString",
        "com",
        "implemented",
        "owned HSTRING with reference count",
        winrt::create_string
    ),
    export!(
        "ole32.dll",
        "WindowsCreateStringReference",
        "com",
        "implemented",
        "HSTRING in the caller's header",
        winrt::create_string_reference
    ),
    export!(
        "ole32.dll",
        "WindowsDeleteString",
        "com",
        "implemented",
        "owned and reference HSTRINGs",
        winrt::delete_string
    ),
    export!(
        "ole32.dll",
        "WindowsDuplicateString",
        "com",
        "implemented",
        "shares owned, copies reference HSTRINGs",
        winrt::duplicate_string
    ),
    export!(
        "ole32.dll",
        "WindowsGetStringRawBuffer",
        "com",
        "implemented",
        "text and length",
        winrt::get_string_raw_buffer
    ),
    export!(
        "ole32.dll",
        "WindowsGetStringLen",
        "com",
        "implemented",
        "length in UTF-16 units",
        winrt::get_string_len
    ),
    export!(
        "ole32.dll",
        "WindowsIsStringEmpty",
        "com",
        "implemented",
        "NULL or zero length",
        winrt::is_string_empty
    ),
    export!(
        "ole32.dll",
        "WindowsCompareStringOrdinal",
        "com",
        "implemented",
        "UTF-16 code unit order",
        winrt::compare_string_ordinal
    ),
    export!(
        "ole32.dll",
        "CoInitialize",
        "com",
        "partial",
        "apartment-threaded COM apartment",
        winrt::co_initialize
    ),
    export!(
        "ole32.dll",
        "StringFromGUID2",
        "com",
        "implemented",
        "braced uppercase text",
        winrt::string_from_guid2
    ),
    export!(
        "ole32.dll",
        "CoCreateGuid",
        "com",
        "implemented",
        "random version 4 GUID",
        winrt::co_create_guid
    ),
    export!(
        "uiautomationcore.dll",
        "UiaHostProviderFromHwnd",
        "accessibility",
        "partial",
        "no automation clients: E_NOTIMPL",
        winrt::uia_host_provider_from_hwnd
    ),
    export!(
        "uiautomationcore.dll",
        "UiaRaiseAutomationEvent",
        "accessibility",
        "partial",
        "no listeners: no-op",
        winrt::uia_raise_event
    ),
    export!(
        "uiautomationcore.dll",
        "UiaRaiseAutomationPropertyChangedEvent",
        "accessibility",
        "partial",
        "no listeners: no-op",
        winrt::uia_raise_property_changed
    ),
    export!(
        "uiautomationcore.dll",
        "UiaRaiseStructureChangedEvent",
        "accessibility",
        "partial",
        "no listeners: no-op",
        winrt::uia_raise_structure_changed
    ),
    export!(
        "uiautomationcore.dll",
        "UiaRaiseNotificationEvent",
        "accessibility",
        "partial",
        "no listeners: no-op",
        winrt::uia_raise_notification
    ),
    export!(
        "uiautomationcore.dll",
        "UiaClientsAreListening",
        "accessibility",
        "implemented",
        "no automation clients",
        winrt::uia_clients_are_listening
    ),
    export!(
        "uiautomationcore.dll",
        "UiaReturnRawElementProvider",
        "accessibility",
        "partial",
        "no automation clients: 0",
        winrt::uia_return_raw_element_provider
    ),
    com!("CoInitializeEx", apartment::initialize),
    com!("CoUninitialize", apartment::uninitialize),
    com!("OleInitialize", apartment::ole_initialize),
    com!("OleUninitialize", apartment::uninitialize),
    com!("CoCreateInstance", apartment::create_instance),
    com!("CoTaskMemAlloc", apartment::allocate),
    com!("CoTaskMemRealloc", apartment::reallocate),
    com!("CoTaskMemFree", apartment::release),
}
