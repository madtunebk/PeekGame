//! Export declarations for registry keys. Implementation stays in api/.
use crate::api::system::registry_keys;
use crate::registry::Export;

exports! {
    export!(
        "advapi32.dll",
        "RegOpenKeyExW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::open_key_ex_w
    ),
    export!(
        "advapi32.dll",
        "RegOpenKeyExA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::open_key_ex_a
    ),
    export!(
        "advapi32.dll",
        "RegOpenKeyW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::open_key_w
    ),
    export!(
        "advapi32.dll",
        "RegOpenKeyA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::open_key_a
    ),
    export!(
        "advapi32.dll",
        "RegCreateKeyExW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::create_key_ex_w
    ),
    export!(
        "advapi32.dll",
        "RegCreateKeyExA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::create_key_ex_a
    ),
    export!(
        "advapi32.dll",
        "RegCreateKeyW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::create_key_w
    ),
    export!(
        "advapi32.dll",
        "RegCreateKeyA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::create_key_a
    ),
    export!(
        "advapi32.dll",
        "RegCloseKey",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::close_key
    ),
    export!(
        "advapi32.dll",
        "RegFlushKey",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::flush_key
    ),
    export!(
        "advapi32.dll",
        "RegQueryValueExW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::query_value_ex_w
    ),
    export!(
        "advapi32.dll",
        "RegQueryValueExA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::query_value_ex_a
    ),
    export!(
        "advapi32.dll",
        "RegSetValueExW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::set_value_ex_w
    ),
    export!(
        "advapi32.dll",
        "RegSetValueExA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::set_value_ex_a
    ),
    export!(
        "advapi32.dll",
        "RegSetKeyValueW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::set_key_value_w
    ),
    export!(
        "advapi32.dll",
        "RegDeleteValueW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::delete_value_w
    ),
    export!(
        "advapi32.dll",
        "RegDeleteValueA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::delete_value_a
    ),
    export!(
        "advapi32.dll",
        "RegDeleteKeyW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::delete_key_w
    ),
    export!(
        "advapi32.dll",
        "RegDeleteKeyA",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::delete_key_a
    ),
    export!(
        "advapi32.dll",
        "RegDeleteKeyExW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::delete_key_ex_w
    ),
    export!(
        "advapi32.dll",
        "RegDeleteTreeW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::delete_tree_w
    ),
    export!(
        "advapi32.dll",
        "RegEnumKeyExW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::enum_key_ex_w
    ),
    export!(
        "advapi32.dll",
        "RegEnumKeyW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::enum_key_w
    ),
    export!(
        "advapi32.dll",
        "RegEnumValueW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::enum_value_w
    ),
    export!(
        "advapi32.dll",
        "RegQueryInfoKeyW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::query_info_key_w
    ),
    export!(
        "advapi32.dll",
        "RegGetValueW",
        "system",
        "partial",
        "HKCU persisted under APPDATA/registry; other roots empty and read-only",
        registry_keys::get_value_w
    ),
    export!(
        "shlwapi.dll",
        "SHDeleteKeyW",
        "system",
        "partial",
        "recursive delete in HKCU",
        registry_keys::delete_tree_w
    ),
}
