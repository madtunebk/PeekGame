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
            scope: "logical COM apartments, task allocator and WASAPI render endpoint",
            address: Some(|| $function as *const std::ffi::c_void),
        }
    };
}
exports! {
    export!("ole32.dll", "CoGetContextToken", "com", "partial", "E_POINTER/null, CO_E_NOTINITIALIZED before COM; initialized context objects unsupported: E_NOTIMPL", crate::api::com::context::get_token),
 export!("ole32.dll","ReleaseStgMedium","com","partial","TYMED_HGLOBAL or pUnkForRelease ownership",crate::api::window::drop_data::release_medium),
    export!("ole32.dll", "RegisterDragDrop", "com", "partial", "owner-thread OLE registration with native Xdnd local-file delivery", crate::api::window::ole_drop::register),
    export!("ole32.dll", "RevokeDragDrop", "com", "partial", "revoke Xdnd awareness and release owned drop target", crate::api::window::ole_drop::revoke),
    export!("ole32.dll", "CoInitializeSecurity", "com", "partial", "apartment security initialization stub", apartment::initialize_security),
    export!("ole32.dll", "CoSetProxyBlanket", "com", "partial", "validates proxy interface pointer", apartment::set_proxy_blanket),

    export!("uiautomationcore.dll", "UiaLookupId", "accessibility", "partial", "UI Automation identifier registry unavailable: lookup returns 0", winrt::uia_lookup_id),
    export!("uiautomationcore.dll", "UiaGetReservedNotSupportedValue", "accessibility", "partial", "returns static sentinel pointer for unsupported value", winrt::uia_get_reserved_not_supported_value),
    export!("uiautomationcore.dll", "UiaGetReservedMixedAttributeValue", "accessibility", "partial", "returns static sentinel pointer for mixed attribute value", winrt::uia_get_reserved_not_supported_value),

    export!("oleaut32.dll", "SafeArrayCreateVector", "com", "partial", "1D safe array contiguous vector allocation", crate::api::com::safearray::create_vector),
    export!("oleaut32.dll", "SafeArrayCreate", "com", "partial", "1D safe array contiguous vector allocation", crate::api::com::safearray::create),
    export!("oleaut32.dll", "SafeArrayDestroy", "com", "partial", "deallocates safe array vector and data", crate::api::com::safearray::destroy_vector),
    export!("oleaut32.dll", "SafeArrayPutElement", "com", "partial", "bounds-checked element assignment for 1D safe array", crate::api::com::safearray::put_element),
    export!("oleaut32.dll", "SafeArrayGetElement", "com", "partial", "bounds-checked element retrieval for 1D safe array", crate::api::com::safearray::get_element),
    export!("oleaut32.dll", "SafeArrayAccessData", "com", "partial", "locks array and retrieves data pointer", crate::api::com::safearray::access_data),
    export!("oleaut32.dll", "SafeArrayUnaccessData", "com", "partial", "unlocks array", crate::api::com::safearray::unaccess_data),
    export!("oleaut32.dll", "SafeArrayGetDim", "com", "partial", "retrieves array dimensions", crate::api::com::safearray::get_dim),
    export!("oleaut32.dll", "SafeArrayGetElemsize", "com", "partial", "retrieves element size", crate::api::com::safearray::get_elemsize),
    export!("oleaut32.dll", "SafeArrayGetUBound", "com", "partial", "retrieves upper bound", crate::api::com::safearray::get_ubound),
    export!("oleaut32.dll", "SafeArrayGetLBound", "com", "partial", "retrieves lower bound", crate::api::com::safearray::get_lbound),
    export!("oleaut32.dll", "SafeArrayLock", "com", "partial", "increments lock count", crate::api::com::safearray::lock),
    export!("oleaut32.dll", "SafeArrayUnlock", "com", "partial", "decrements lock count", crate::api::com::safearray::unlock),
    export!("oleaut32.dll", "SafeArrayCopy", "com", "partial", "1D safe array deep copy", crate::api::com::safearray::copy),
    export!("oleaut32.dll", "SafeArrayGetVartype", "com", "partial", "retrieves safe array element vartype", crate::api::com::safearray::get_vartype),
    export!("oleaut32.dll", "SetErrorInfo", "com", "partial", "thread-local IErrorInfo assignment", crate::api::com::error_info::set_error_info),
    export!("oleaut32.dll", "GetErrorInfo", "com", "partial", "thread-local IErrorInfo retrieval and clear", crate::api::com::error_info::get_error_info),
    export!("oleaut32.dll", "CreateErrorInfo", "com", "partial", "returns E_NOTIMPL", crate::api::com::error_info::create_error_info),
    export!("ole32.dll", "SetErrorInfo", "com", "partial", "thread-local IErrorInfo assignment", crate::api::com::error_info::set_error_info),
    export!("ole32.dll", "GetErrorInfo", "com", "partial", "thread-local IErrorInfo retrieval and clear", crate::api::com::error_info::get_error_info),
    export!("ole32.dll", "CreateErrorInfo", "com", "partial", "returns E_NOTIMPL", crate::api::com::error_info::create_error_info),
    export!("oleaut32.dll", "VariantCopy", "com", "partial", "copies VARIANT and duplicates BSTR", bstr::variant_copy),
    export!("oleaut32.dll", "VariantCopyInd", "com", "partial", "copies VARIANT and duplicates BSTR", bstr::variant_copy_ind),
    export!("oleaut32.dll", "VariantChangeType", "com", "partial", "basic VARIANT type change", bstr::variant_change_type),
    export!("oleaut32.dll", "SystemTimeToVariantTime", "com", "partial", "converts SYSTEMTIME to VARIANT time", bstr::system_time_to_variant_time),
    export!("oleaut32.dll", "VariantTimeToSystemTime", "com", "partial", "converts VARIANT time to SYSTEMTIME", bstr::variant_time_to_system_time),
    export!("oleaut32.dll", "VarBstrCmp", "com", "implemented", "compares two BSTRs", bstr::var_bstr_cmp),
    export!("oleaut32.dll", "VarCmp", "com", "partial", "compares two VARIANTs", bstr::var_cmp),

    export!("ole32.dll", "PropVariantClear", "com", "partial", "owned scalar/string/blob/interface variants", crate::api::com::variants::clear),
    export!("ole32.dll", "PropVariantCopy", "com", "partial", "copies PROPVARIANT", crate::api::com::variants::copy),
    export!("ole32.dll", "CoCreateFreeThreadedMarshaler", "com", "partial", "returns E_NOTIMPL", crate::api::com::variants::create_free_threaded_marshaler),
    export!("propsys.dll", "PropVariantToBSTR", "com", "partial", "converts PROPVARIANT to BSTR", crate::api::com::variants::to_bstr),
    export!("propsys.dll", "InitPropVariantFromCLSID", "com", "implemented", "initializes VT_CLSID PROPVARIANT", crate::api::com::variants::init_from_clsid),
    export!("propsys.dll", "PSGetPropertyKeyFromName", "com", "partial", "returns TYPE_E_ELEMENTNOTFOUND", crate::api::com::variants::get_property_key_from_name),
    export!("propsys.dll", "VariantCompare", "com", "partial", "returns 0 (equal)", crate::api::com::variants::variant_compare),
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
        "QueryPathOfRegTypeLib",
        "com",
        "partial",
        "returns TYPE_E_LIBNOTREGISTERED",
        bstr::query_path_of_reg_type_lib
    ),
    export!(
        "oleaut32.dll",
        "RegisterTypeLib",
        "com",
        "partial",
        "returns S_OK",
        bstr::register_type_lib
    ),
    export!(
        "oleaut32.dll",
        "UnRegisterTypeLib",
        "com",
        "partial",
        "returns S_OK",
        bstr::unregister_type_lib
    ),
    export!(
        "oleaut32.dll",
        "RegisterTypeLibForUser",
        "com",
        "partial",
        "returns S_OK",
        bstr::register_type_lib_for_user
    ),
    export!(
        "oleaut32.dll",
        "UnRegisterTypeLibForUser",
        "com",
        "partial",
        "returns S_OK",
        bstr::unregister_type_lib_for_user
    ),
    export!(
        "oleaut32.dll",
        "LoadRegTypeLib",
        "com",
        "partial",
        "returns TYPE_E_LIBNOTREGISTERED",
        bstr::load_reg_type_lib
    ),
    export!(
        "oleaut32.dll",
        "LoadTypeLib",
        "com",
        "partial",
        "returns TYPE_E_CANTLOADLIBRARY",
        bstr::load_type_lib
    ),
    export!(
        "oleaut32.dll",
        "LoadTypeLibEx",
        "com",
        "partial",
        "returns TYPE_E_CANTLOADLIBRARY",
        bstr::load_type_lib_ex
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
        "WindowsConcatString",
        "com",
        "implemented",
        "new owned HSTRING from two HSTRINGs (NULL is empty)",
        winrt::concat_string
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
    com!("OleUninitialize", apartment::ole_uninitialize),
    com!("CoCreateInstance", apartment::create_instance),
    com!("CoTaskMemAlloc", apartment::allocate),
    com!("CoTaskMemRealloc", apartment::reallocate),
    com!("CoTaskMemFree", apartment::release),
    com!("IIDFromString", apartment::iid_from_string),
    com!("CLSIDFromString", apartment::clsid_from_string),
    com!("StringFromIID", apartment::string_from_iid),
    com!("StringFromCLSID", apartment::string_from_iid),
    export!("mfplat.dll", "MFStartup", "multimedia", "partial", "Media Foundation startup stub", winrt::mf_startup),
    export!("mfplat.dll", "MFShutdown", "multimedia", "partial", "Media Foundation shutdown stub", winrt::mf_shutdown),
    export!("mfplat.dll", "MFFrameRateToAverageTimePerFrame", "multimedia", "implemented", "100ns frame duration from rational rate", winrt::mf_frame_rate_to_avg_time),
    export!("mfplat.dll", "MFCreateAlignedMemoryBuffer", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateAsyncResult", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateAttributes", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateDXGIDeviceManager", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateDXGISurfaceBuffer", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateEventQueue", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateMediaEvent", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateMediaType", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateMemoryBuffer", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreatePresentationDescriptor", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateSample", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFCreateStreamDescriptor", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFLockDXGIDeviceManager", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFUnlockDXGIDeviceManager", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFTEnum", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfplat.dll", "MFTEnumEx", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mf.dll", "MFCreateDeviceSource", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mf.dll", "MFEnumDeviceSources", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mf.dll", "MFGetService", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("mfreadwrite.dll", "MFCreateSourceReaderFromMediaSource", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("dxva2.dll", "DXVA2CreateDirect3DDeviceManager9", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("dxva2.dll", "DXVA2CreateVideoService", "multimedia", "partial", "E_NOTIMPL", winrt::mf_not_impl),
    export!("msmpeg2vdec.dll", "DllGetClassObject", "multimedia", "partial", "REGDB_E_CLASSNOTREG", winrt::mf_class_not_reg),
    export!("msvproc.dll", "DllGetClassObject", "multimedia", "partial", "REGDB_E_CLASSNOTREG", winrt::mf_class_not_reg),
}

