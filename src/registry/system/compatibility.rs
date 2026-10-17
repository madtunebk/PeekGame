//! Export declarations for compatibility. Implementation stays in api/.
use crate::api::system::compat;
use crate::registry::Export;

exports! {
    implemented!("SetHandleCount", "system", "implemented", "Windows NT no-op; returns requested count", compat::set_handle_count),
    implemented!(
        "lstrlenW",
        "system",
        "partial",
        "complete",
        compat::lstrlen_w
    ),
    implemented!(
        "lstrlenA",
        "system",
        "partial",
        "complete",
        compat::lstrlen_a
    ),
    implemented!(
        "lstrcmpW",
        "system",
        "partial",
        "ordinal comparison",
        compat::lstrcmp_w
    ),
    implemented!(
        "lstrcmpiW",
        "system",
        "partial",
        "case-insensitive",
        compat::lstrcmpi_w
    ),
    implemented!(
        "lstrcmpA",
        "system",
        "partial",
        "byte comparison",
        compat::lstrcmp_a
    ),
    implemented!(
        "lstrcmpiA",
        "system",
        "partial",
        "ASCII case-insensitive",
        compat::lstrcmpi_a
    ),
    implemented!(
        "lstrcpyW",
        "system",
        "partial",
        "complete",
        compat::lstrcpy_w
    ),
    implemented!(
        "lstrcpynW",
        "system",
        "partial",
        "complete",
        compat::lstrcpyn_w
    ),
    implemented!(
        "lstrcatW",
        "system",
        "partial",
        "complete",
        compat::lstrcat_w
    ),
    implemented!(
        "lstrcpyA",
        "system",
        "partial",
        "complete",
        compat::lstrcpy_a
    ),
    implemented!(
        "CreateSemaphoreA",
        "threading",
        "partial",
        "as CreateSemaphoreW",
        compat::create_semaphore_a
    ),
    implemented!(
        "GetTempPathA",
        "filesystem",
        "partial",
        "as GetTempPathW",
        compat::temp_path_a
    ),
    implemented!(
        "GetTempFileNameA",
        "filesystem",
        "partial",
        "as GetTempFileNameW",
        compat::temp_file_name_a
    ),
    implemented!(
        "GetProcessAffinityMask",
        "system",
        "partial",
        "every host CPU",
        compat::process_affinity_mask
    ),
    implemented!(
        "SetProcessAffinityMask",
        "system",
        "partial",
        "accepted",
        compat::set_process_affinity_mask
    ),
    implemented!(
        "GetHandleInformation",
        "system",
        "partial",
        "no flags",
        compat::handle_information
    ),
    implemented!(
        "SetHandleInformation",
        "system",
        "partial",
        "accepted",
        compat::set_handle_information
    ),
    implemented!(
        "OpenProcess",
        "system",
        "partial",
        "this process only",
        compat::open_process
    ),
    implemented!(
        "DeviceIoControl",
        "system",
        "partial",
        "no device drivers",
        compat::device_io_control
    ),
    export!(
        "advapi32.dll",
        "RegNotifyChangeKeyValue",
        "system",
        "partial",
        "accepted; no notifications",
        compat::reg_notify_change_key_value
    ),
    export!(
        "gdi32.dll",
        "CreateCompatibleDC",
        "window",
        "partial",
        "token DC",
        compat::create_compatible_dc
    ),
    export!(
        "gdi32.dll",
        "DeleteDC",
        "window",
        "partial",
        "token DC",
        compat::delete_dc
    ),
    export!(
        "user32.dll",
        "ChangeDisplaySettingsExW",
        "window",
        "partial",
        "no mode switching: restore and the device's current size succeed, other sizes DISP_CHANGE_BADMODE",
        compat::change_display_settings_ex_w
    ),
    export!(
        "user32.dll",
        "ChangeDisplaySettingsExA",
        "window",
        "partial",
        "no mode switching: restore and the device's current size succeed, other sizes DISP_CHANGE_BADMODE",
        compat::change_display_settings_ex_a
    ),
    implemented!(
        "GetProfileIntA",
        "system",
        "partial",
        "no win.ini; returns caller default",
        compat::get_profile_int_a
    ),
    implemented!(
        "MulDiv",
        "system",
        "implemented",
        "64-bit intermediate multiply-divide with rounding",
        compat::mul_div
    ),
}
