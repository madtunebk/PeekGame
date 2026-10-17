//! Export declarations for devices. Implementation stays in api/.
use crate::api::system::devices;
use crate::registry::Export;

exports! {
    export!(
        "hid.dll",
        "HidD_GetHidGuid",
        "system",
        "partial",
        "the HID class GUID",
        devices::hid_guid
    ),
    export!(
        "hid.dll",
        "HidD_GetSerialNumberString",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_GetManufacturerString",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_GetProductString",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_FreePreparsedData",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_GetPreparsedData",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_GetAttributes",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_SetOutputReport",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_GetFeature",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_SetFeature",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidD_GetInputReport",
        "system",
        "partial",
        "no HID devices",
        devices::hid_false
    ),
    export!(
        "hid.dll",
        "HidP_GetData",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_MaxDataListLength",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_SetUsages",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_SetUsageValue",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_GetButtonCaps",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_GetValueCaps",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_GetCaps",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_GetUsageValue",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "hid.dll",
        "HidP_GetUsages",
        "system",
        "partial",
        "no HID devices",
        devices::hidp_invalid
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetClassDevsA",
        "system",
        "partial",
        "empty device set",
        devices::get_class_devs
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetClassDevsW",
        "system",
        "partial",
        "empty device set",
        devices::get_class_devs
    ),
    export!(
        "setupapi.dll",
        "SetupDiEnumDeviceInterfaces",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiEnumDeviceInfo",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetDeviceInterfaceDetailW",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetDeviceInterfaceDetailA",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetDeviceRegistryPropertyW",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetDeviceRegistryPropertyA",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiGetDevicePropertyW",
        "system",
        "partial",
        "no devices",
        devices::no_more_items
    ),
    export!(
        "setupapi.dll",
        "SetupDiDestroyDeviceInfoList",
        "system",
        "partial",
        "complete",
        devices::destroy_list
    ),
}
