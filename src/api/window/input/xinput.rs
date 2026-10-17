//! XInput without a controller backend: every user slot (0-3) is reported as not
//! connected, as Windows does with no pads attached. Indexes past 3 are
//! ERROR_BAD_ARGUMENTS. Nothing is written to the caller's structures.
const ERROR_BAD_ARGUMENTS: u32 = 160;
const ERROR_DEVICE_NOT_CONNECTED: u32 = 1167;
const XUSER_INDEX_ANY: u32 = 0xff;

fn slot(user: u32, out: usize) -> u32 {
    if user > 3 || out == 0 {
        ERROR_BAD_ARGUMENTS
    } else {
        ERROR_DEVICE_NOT_CONNECTED
    }
}
pub(crate) extern "win64" fn get_state(user: u32, state: usize) -> u32 {
    slot(user, state)
}
pub(crate) extern "win64" fn set_state(user: u32, vibration: usize) -> u32 {
    slot(user, vibration)
}
pub(crate) extern "win64" fn get_capabilities(user: u32, flags: u32, capabilities: usize) -> u32 {
    if flags & !1 != 0 {
        return ERROR_BAD_ARGUMENTS; // only XINPUT_FLAG_GAMEPAD
    }
    slot(user, capabilities)
}
/// XInputEnable only gates reporting; with no controllers there is nothing to gate.
pub(crate) extern "win64" fn enable(_enable: i32) {}
pub(crate) extern "win64" fn get_battery_information(user: u32, device: u32, info: usize) -> u32 {
    if device > 1 {
        return ERROR_BAD_ARGUMENTS; // BATTERY_DEVTYPE_GAMEPAD or _HEADSET
    }
    slot(user, info)
}
pub(crate) extern "win64" fn get_keystroke(user: u32, _reserved: u32, keystroke: usize) -> u32 {
    if user == XUSER_INDEX_ANY && keystroke != 0 {
        return ERROR_DEVICE_NOT_CONNECTED;
    }
    slot(user, keystroke)
}
pub(crate) extern "win64" fn get_dsound_audio_device_guids(user: u32, render: usize, capture: usize) -> u32 {
    slot(user, if render == 0 || capture == 0 { 0 } else { 1 })
}
pub(crate) extern "win64" fn get_audio_device_ids(
    user: u32,
    render: usize,
    _render_count: usize,
    _capture: usize,
    _capture_count: usize,
) -> u32 {
    slot(user, render.max(1))
}
