//! Export declarations for version. Implementation stays in api/.
use crate::api::system::version;
use crate::registry::Export;

exports! {
    implemented!(
        "GetVersion",
        "system",
        "partial",
        "Windows 10.0.19045",
        version::get_version
    ),
    implemented!(
        "GetProductInfo",
        "system",
        "partial",
        "Windows 10 Pro product type for supported target versions",
        version::get_product_info
    ),
    implemented!(
        "GetVersionExW",
        "system",
        "partial",
        "Windows 10.0.19045 workstation",
        version::get_version_ex_w
    ),
    implemented!(
        "GetVersionExA",
        "system",
        "partial",
        "Windows 10.0.19045 workstation",
        version::get_version_ex_a
    ),
    implemented!(
        "VerSetConditionMask",
        "system",
        "partial",
        "complete",
        version::ver_set_condition_mask
    ),
    implemented!(
        "VerifyVersionInfoW",
        "system",
        "partial",
        "against Windows 10.0.19045 workstation",
        version::verify_version_info_w
    ),
    implemented!(
        "VerifyVersionInfoA",
        "system",
        "partial",
        "against Windows 10.0.19045 workstation",
        version::verify_version_info_a
    ),
    export!(
        "ntdll.dll",
        "RtlGetVersion",
        "system",
        "partial",
        "Windows 10.0.19045 workstation",
        version::rtl_get_version
    ),
    export!(
        "ntdll.dll",
        "RtlVerifyVersionInfo",
        "system",
        "partial",
        "against Windows 10.0.19045 workstation",
        version::rtl_verify_version_info
    ),
    export!(
        "ntdll.dll",
        "VerSetConditionMask",
        "system",
        "partial",
        "complete",
        version::ver_set_condition_mask
    ),
    export!(
        "version.dll",
        "GetFileVersionInfoSizeW",
        "system",
        "partial",
        "fixed 512-byte block",
        version::get_file_version_info_size_w
    ),
    export!(
        "version.dll",
        "GetFileVersionInfoSizeA",
        "system",
        "partial",
        "fixed 512-byte block",
        version::get_file_version_info_size_a
    ),
    export!(
        "version.dll",
        "GetFileVersionInfoW",
        "system",
        "partial",
        "standard Windows 10 VS_FIXEDFILEINFO",
        version::get_file_version_info_w
    ),
    export!(
        "version.dll",
        "GetFileVersionInfoA",
        "system",
        "partial",
        "standard Windows 10 VS_FIXEDFILEINFO",
        version::get_file_version_info_a
    ),
    export!(
        "version.dll",
        "VerQueryValueW",
        "system",
        "partial",
        "root and version string queries",
        version::ver_query_value_w
    ),
    export!(
        "version.dll",
        "VerQueryValueA",
        "system",
        "partial",
        "root and version string queries",
        version::ver_query_value_a
    ),
}
