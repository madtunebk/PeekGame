//! hid.dll and setupapi.dll: device enumeration finds no HID devices (gamepads
//! and other HID input are not bridged yet); keyboard and mouse come through
//! window messages and raw input.
use super::error;
/// HidD_GetHidGuid: {4D1E55B2-F16F-11CF-88CB-001111000030}.
pub unsafe extern "win64" fn hid_guid(out: *mut u8) {
    const GUID: [u8; 16] = [
        0xb2, 0x55, 0x1e, 0x4d, 0x6f, 0xf1, 0xcf, 0x11, 0x88, 0xcb, 0x00, 0x11, 0x11, 0x00, 0x00,
        0x30,
    ];
    if !out.is_null() {
        unsafe { std::ptr::copy_nonoverlapping(GUID.as_ptr(), out, 16) };
    }
}
/// HidD_* queries on a device: no HID devices are opened here.
pub extern "win64" fn hid_false() -> u8 {
    error::set(6);
    0
}
/// HidP_* parsers: HIDP_STATUS_INVALID_PREPARSED_DATA.
pub extern "win64" fn hidp_invalid() -> i32 {
    0xC011_0001u32 as i32
}
/// SetupDiGetClassDevs: an empty device information set.
pub extern "win64" fn get_class_devs() -> usize {
    0x000e_0010
}
pub extern "win64" fn get_class_devs_ex(
    _class_guid: usize,
    _enumerator: usize,
    _hwnd_parent: usize,
    _flags: u32,
    _device_info_set: usize,
    _machine_name: usize,
    _reserved: usize,
) -> usize {
    0x000e_0010
}
/// SetupDiEnum*/SetupDiGetDeviceInterfaceDetail: ERROR_NO_MORE_ITEMS.
pub extern "win64" fn no_more_items() -> i32 {
    error::set(259);
    0
}
pub extern "win64" fn destroy_list(_set: usize) -> i32 {
    1
}
