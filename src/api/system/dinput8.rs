use crate::api::window::input::key_down;
use crate::host::x11::keyboard::scan_of_vk;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

pub const GUID_SYSKEYBOARD: [u8; 16] = [
    0x61, 0x2b, 0x1d, 0x6f, 0xa0, 0xd5, 0xcf, 0x11,
    0xbf, 0xc7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00,
];

pub const GUID_SYSMOUSE: [u8; 16] = [
    0x60, 0x2b, 0x1d, 0x6f, 0xa0, 0xd5, 0xcf, 0x11,
    0xbf, 0xc7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00,
];

const DI_OK: i32 = 0;
const DIENUM_CONTINUE: i32 = 1;

#[repr(C)]
#[allow(clippy::upper_case_acronyms)] // mirrors the Win32 DIDEVICEINSTANCEW name
struct DIDEVICEINSTANCEW {
    dw_size: u32,
    guid_instance: [u8; 16],
    guid_product: [u8; 16],
    dw_dev_type: u32,
    tsz_instance_name: [u16; 260],
    tsz_product_name: [u16; 260],
    guid_ff_driver: [u8; 16],
    w_usage_page: u16,
    w_usage: u16,
}

#[repr(C)]
pub struct DirectInputDevice8Object {
    pub vtable: *const usize,
    pub refs: AtomicU32,
    pub is_mouse: bool,
}
unsafe impl Sync for DirectInputDevice8Object {}
unsafe impl Send for DirectInputDevice8Object {}

unsafe extern "win64" fn didevice8_query_interface(
    this: *mut DirectInputDevice8Object,
    _iid: *const u8,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe {
        *out = this.cast();
        didevice8_add_ref(this);
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_add_ref(this: *mut DirectInputDevice8Object) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

unsafe extern "win64" fn didevice8_release(this: *mut DirectInputDevice8Object) -> u32 {
    let old = unsafe { (*this).refs.load(Ordering::Relaxed) };
    if old > 1 {
        unsafe { (*this).refs.fetch_sub(1, Ordering::Relaxed) - 1 }
    } else {
        1
    }
}

unsafe extern "win64" fn didevice8_get_capabilities(
    this: *mut DirectInputDevice8Object,
    caps: *mut u32,
) -> i32 {
    if caps.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe {
        let size = *caps as usize;
        if size >= 8 {
            *caps.add(1) = 0x00000001; // DIDC_ATTACHED
            let dev_type = if (*this).is_mouse { 0x12 } else { 0x13 };
            *caps.add(2) = dev_type;
        }
        if size >= 24 {
            if (*this).is_mouse {
                *caps.add(3) = 3; // 3 axes: X, Y, Z
                *caps.add(4) = 5; // 5 buttons
                *caps.add(5) = 0; // 0 POVs
            } else {
                *caps.add(3) = 0; // 0 axes
                *caps.add(4) = 104; // 104 keys
                *caps.add(5) = 0; // 0 POVs
            }
        }
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_enum_objects(
    _this: *mut DirectInputDevice8Object,
    _cb: usize,
    _ref: usize,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_get_property(
    _this: *mut DirectInputDevice8Object,
    _prop: *const u8,
    _hdr: *mut u8,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_set_property(
    _this: *mut DirectInputDevice8Object,
    _prop: *const u8,
    _hdr: *const u8,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_acquire(_this: *mut DirectInputDevice8Object) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_unacquire(_this: *mut DirectInputDevice8Object) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_get_device_state(
    this: *mut DirectInputDevice8Object,
    cb_data: u32,
    lpv_data: *mut u8,
) -> i32 {
    if lpv_data.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe {
        std::ptr::write_bytes(lpv_data, 0, cb_data as usize);
        if (*this).is_mouse {
            // DIMOUSESTATE: lX(4), lY(4), lZ(4), rgbButtons[4]
            // DIMOUSESTATE2: lX(4), lY(4), lZ(4), rgbButtons[8]
            if cb_data >= 16 {
                if key_down(1) {
                    *lpv_data.add(12) = 0x80;
                }
                if key_down(2) {
                    *lpv_data.add(13) = 0x80;
                }
                if key_down(4) {
                    *lpv_data.add(14) = 0x80;
                }
            }
        } else {
            // Keyboard 256-byte scancode buffer
            for vk in 0..=255u8 {
                if key_down(vk) {
                    let dik = (scan_of_vk(vk) & 0xff) as usize;
                    if dik < cb_data as usize {
                        *lpv_data.add(dik) = 0x80;
                    }
                }
            }
        }
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_get_device_data(
    _this: *mut DirectInputDevice8Object,
    _cb_data: u32,
    _rgdod: *mut u8,
    pdw_in_out: *mut u32,
    _flags: u32,
) -> i32 {
    if !pdw_in_out.is_null() {
        unsafe { *pdw_in_out = 0 };
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_set_data_format(
    _this: *mut DirectInputDevice8Object,
    _format: *const u8,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_set_event_notification(
    _this: *mut DirectInputDevice8Object,
    _event: usize,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_set_cooperative_level(
    _this: *mut DirectInputDevice8Object,
    _hwnd: usize,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_get_object_info(
    _this: *mut DirectInputDevice8Object,
    _doi: *mut u8,
    _obj: u32,
    _how: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_get_device_info(
    this: *mut DirectInputDevice8Object,
    didi: *mut u8,
) -> i32 {
    if didi.is_null() {
        return 0x80004003u32 as i32;
    }
    let is_mouse = unsafe { (*this).is_mouse };
    unsafe {
        let inst = didi as *mut DIDEVICEINSTANCEW;
        populate_instance(inst, is_mouse);
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_run_control_panel(
    _this: *mut DirectInputDevice8Object,
    _hwnd: usize,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_initialize(
    _this: *mut DirectInputDevice8Object,
    _instance: usize,
    _version: u32,
    _guid: *const u8,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_create_effect(
    _this: *mut DirectInputDevice8Object,
    _guid: *const u8,
    _params: *const u8,
    out: *mut *mut c_void,
    _outer: *mut c_void,
) -> i32 {
    if !out.is_null() {
        unsafe { *out = std::ptr::null_mut() };
    }
    0x80004001u32 as i32
}

unsafe extern "win64" fn didevice8_enum_effects(
    _this: *mut DirectInputDevice8Object,
    _cb: usize,
    _ref: usize,
    _type: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_get_effect_info(
    _this: *mut DirectInputDevice8Object,
    _dei: *mut u8,
    _guid: *const u8,
) -> i32 {
    0x80004001u32 as i32
}

unsafe extern "win64" fn didevice8_get_force_feedback_state(
    _this: *mut DirectInputDevice8Object,
    flags: *mut u32,
) -> i32 {
    if !flags.is_null() {
        unsafe { *flags = 0 };
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_send_force_feedback_command(
    _this: *mut DirectInputDevice8Object,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_enum_created_effect_objects(
    _this: *mut DirectInputDevice8Object,
    _cb: usize,
    _ref: usize,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_escape(
    _this: *mut DirectInputDevice8Object,
    _escape: *mut u8,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_poll(_this: *mut DirectInputDevice8Object) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_send_device_data(
    _this: *mut DirectInputDevice8Object,
    _cb_data: u32,
    _rgdod: *const u8,
    pdw_in_out: *mut u32,
    _flags: u32,
) -> i32 {
    if !pdw_in_out.is_null() {
        unsafe { *pdw_in_out = 0 };
    }
    DI_OK
}

unsafe extern "win64" fn didevice8_enum_effects_in_file(
    _this: *mut DirectInputDevice8Object,
    _file: *const u16,
    _cb: usize,
    _ref: usize,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_write_effect_to_file(
    _this: *mut DirectInputDevice8Object,
    _file: *const u16,
    _entries: u32,
    _rgdif: *const u8,
    _flags: u32,
) -> i32 {
    0x80004001u32 as i32
}

unsafe extern "win64" fn didevice8_build_action_map(
    _this: *mut DirectInputDevice8Object,
    _lpdiaf: *mut u8,
    _user: *const u16,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_set_action_map(
    _this: *mut DirectInputDevice8Object,
    _lpdiaf: *const u8,
    _user: *const u16,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn didevice8_get_image_info(
    _this: *mut DirectInputDevice8Object,
    _image_info: *mut u8,
) -> i32 {
    0x80004001u32 as i32
}

fn device_vtable() -> &'static [usize; 32] {
    static VTABLE: OnceLock<[usize; 32]> = OnceLock::new();
    VTABLE.get_or_init(|| [
        didevice8_query_interface as *const () as usize,
        didevice8_add_ref as *const () as usize,
        didevice8_release as *const () as usize,
        didevice8_get_capabilities as *const () as usize,
        didevice8_enum_objects as *const () as usize,
        didevice8_get_property as *const () as usize,
        didevice8_set_property as *const () as usize,
        didevice8_acquire as *const () as usize,
        didevice8_unacquire as *const () as usize,
        didevice8_get_device_state as *const () as usize,
        didevice8_get_device_data as *const () as usize,
        didevice8_set_data_format as *const () as usize,
        didevice8_set_event_notification as *const () as usize,
        didevice8_set_cooperative_level as *const () as usize,
        didevice8_get_object_info as *const () as usize,
        didevice8_get_device_info as *const () as usize,
        didevice8_run_control_panel as *const () as usize,
        didevice8_initialize as *const () as usize,
        didevice8_create_effect as *const () as usize,
        didevice8_enum_effects as *const () as usize,
        didevice8_get_effect_info as *const () as usize,
        didevice8_get_force_feedback_state as *const () as usize,
        didevice8_send_force_feedback_command as *const () as usize,
        didevice8_enum_created_effect_objects as *const () as usize,
        didevice8_escape as *const () as usize,
        didevice8_poll as *const () as usize,
        didevice8_send_device_data as *const () as usize,
        didevice8_enum_effects_in_file as *const () as usize,
        didevice8_write_effect_to_file as *const () as usize,
        didevice8_build_action_map as *const () as usize,
        didevice8_set_action_map as *const () as usize,
        didevice8_get_image_info as *const () as usize,
    ])
}

fn create_device_instance(is_mouse: bool) -> &'static DirectInputDevice8Object {
    static KEYBOARD: OnceLock<DirectInputDevice8Object> = OnceLock::new();
    static MOUSE: OnceLock<DirectInputDevice8Object> = OnceLock::new();
    if is_mouse {
        MOUSE.get_or_init(|| DirectInputDevice8Object {
            vtable: device_vtable().as_ptr(),
            refs: AtomicU32::new(1),
            is_mouse: true,
        })
    } else {
        KEYBOARD.get_or_init(|| DirectInputDevice8Object {
            vtable: device_vtable().as_ptr(),
            refs: AtomicU32::new(1),
            is_mouse: false,
        })
    }
}

unsafe fn populate_instance(inst: *mut DIDEVICEINSTANCEW, is_mouse: bool) {
    if inst.is_null() {
        return;
    }
    let guid = if is_mouse { GUID_SYSMOUSE } else { GUID_SYSKEYBOARD };
    let name_str = if is_mouse { "Mouse" } else { "Keyboard" };
    let mut name_wide = [0u16; 260];
    for (i, c) in name_str.encode_utf16().enumerate() {
        name_wide[i] = c;
    }
    unsafe {
        (*inst).dw_size = std::mem::size_of::<DIDEVICEINSTANCEW>() as u32;
        (*inst).guid_instance = guid;
        (*inst).guid_product = guid;
        (*inst).dw_dev_type = if is_mouse { 0x12 } else { 0x13 };
        (*inst).tsz_instance_name = name_wide;
        (*inst).tsz_product_name = name_wide;
        (*inst).guid_ff_driver = [0u8; 16];
        (*inst).w_usage_page = 1;
        (*inst).w_usage = if is_mouse { 2 } else { 6 };
    }
}

// ------------------------------------------------------------------------------------------------
// DirectInput8 Object
// ------------------------------------------------------------------------------------------------

#[repr(C)]
pub struct DirectInput8Object {
    pub vtable: *const usize,
    pub refs: AtomicU32,
}
unsafe impl Sync for DirectInput8Object {}
unsafe impl Send for DirectInput8Object {}

unsafe extern "win64" fn di8_query(
    this: *mut DirectInput8Object,
    _iid: *const u8,
    out: *mut *mut c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32;
    }
    unsafe {
        *out = this.cast();
        di8_add_ref(this);
    }
    DI_OK
}

unsafe extern "win64" fn di8_add_ref(this: *mut DirectInput8Object) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::Relaxed) + 1 }
}

unsafe extern "win64" fn di8_release(this: *mut DirectInput8Object) -> u32 {
    let old = unsafe { (*this).refs.load(Ordering::Relaxed) };
    if old > 1 {
        unsafe { (*this).refs.fetch_sub(1, Ordering::Relaxed) - 1 }
    } else {
        1
    }
}

unsafe extern "win64" fn di8_create_device(
    _this: *mut DirectInput8Object,
    guid: *const u8,
    out: *mut *mut c_void,
    _outer: *mut c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32;
    }
    if guid.is_null() {
        unsafe { *out = std::ptr::null_mut() };
        return 0x80070057u32 as i32; // E_INVALIDARG
    }
    let g = unsafe { std::slice::from_raw_parts(guid, 16) };
    let is_mouse = g == GUID_SYSMOUSE;
    let is_kb = g == GUID_SYSKEYBOARD;
    if !is_mouse && !is_kb {
        unsafe { *out = std::ptr::null_mut() };
        return 0x8007007eu32 as i32; // DIERR_DEVICENOTREG
    }
    let dev = create_device_instance(is_mouse);
    dev.refs.store(1, Ordering::Relaxed);
    unsafe { *out = (dev as *const DirectInputDevice8Object).cast_mut().cast() };
    DI_OK
}

type EnumDevicesCallbackW = unsafe extern "win64" fn(*const DIDEVICEINSTANCEW, usize) -> i32;

unsafe extern "win64" fn di8_enum_devices(
    _this: *mut DirectInput8Object,
    class: u32,
    callback: usize,
    context: usize,
    _flags: u32,
) -> i32 {
    if callback == 0 {
        return 0x80004003u32 as i32;
    }
    let cb: EnumDevicesCallbackW = unsafe { std::mem::transmute(callback) };
    let include_mouse = class == 0 || class == 1 || class == 2;
    let include_kb = class == 0 || class == 1 || class == 3;

    if include_kb {
        let mut inst = unsafe { std::mem::zeroed::<DIDEVICEINSTANCEW>() };
        unsafe { populate_instance(&mut inst, false) };
        let cont = unsafe { cb(&inst, context) };
        if cont != DIENUM_CONTINUE {
            return DI_OK;
        }
    }
    if include_mouse {
        let mut inst = unsafe { std::mem::zeroed::<DIDEVICEINSTANCEW>() };
        unsafe { populate_instance(&mut inst, true) };
        let _ = unsafe { cb(&inst, context) };
    }
    DI_OK
}

unsafe extern "win64" fn di8_device_status(_this: *mut DirectInput8Object, _guid: *const u8) -> i32 {
    DI_OK
}

unsafe extern "win64" fn di8_control_panel(_this: *mut DirectInput8Object, _owner: usize, _flags: u32) -> i32 {
    DI_OK
}

unsafe extern "win64" fn di8_initialize(_this: *mut DirectInput8Object, _instance: usize, _version: u32) -> i32 {
    DI_OK
}

unsafe extern "win64" fn di8_find_device(
    _this: *mut DirectInput8Object,
    _guid: *const u8,
    _name: *const u16,
    out: *mut u8,
) -> i32 {
    if !out.is_null() {
        unsafe { std::ptr::write_bytes(out, 0, 16) };
    }
    0x8007007eu32 as i32
}

unsafe extern "win64" fn di8_enum_semantics(
    _this: *mut DirectInput8Object,
    _user: *const u16,
    _action: *const u8,
    _callback: usize,
    _context: usize,
    _flags: u32,
) -> i32 {
    DI_OK
}

unsafe extern "win64" fn di8_configure(
    _this: *mut DirectInput8Object,
    _params: *const u8,
    _callback: usize,
    _context: usize,
    _flags: u32,
) -> i32 {
    0x80004001u32 as i32
}

fn direct_input8_object() -> &'static DirectInput8Object {
    static VTABLE: OnceLock<[usize; 11]> = OnceLock::new();
    static OBJECT: OnceLock<DirectInput8Object> = OnceLock::new();
    let vtable = VTABLE.get_or_init(|| [
        di8_query as *const () as usize,
        di8_add_ref as *const () as usize,
        di8_release as *const () as usize,
        di8_create_device as *const () as usize,
        di8_enum_devices as *const () as usize,
        di8_device_status as *const () as usize,
        di8_control_panel as *const () as usize,
        di8_initialize as *const () as usize,
        di8_find_device as *const () as usize,
        di8_enum_semantics as *const () as usize,
        di8_configure as *const () as usize,
    ]);
    OBJECT.get_or_init(|| DirectInput8Object {
        vtable: vtable.as_ptr(),
        refs: AtomicU32::new(1),
    })
}

pub unsafe extern "win64" fn direct_input8_create(
    _instance: usize,
    _version: u32,
    _iid: *const u8,
    out: *mut *mut c_void,
    outer: *mut c_void,
) -> i32 {
    if out.is_null() {
        return 0x80004003u32 as i32;
    }
    if !outer.is_null() {
        unsafe { *out = std::ptr::null_mut() };
        return 0x80040110u32 as i32;
    }
    let object = direct_input8_object();
    object.refs.store(1, Ordering::Relaxed);
    unsafe { *out = (object as *const DirectInput8Object).cast_mut().cast() };
    DI_OK
}
