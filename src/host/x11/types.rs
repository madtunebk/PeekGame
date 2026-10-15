use std::ffi::{c_char, c_int, c_long, c_ulong};
pub type Display = std::ffi::c_void;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Any {
    pub kind: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: c_ulong,
}
/// XKeyEvent, XButtonEvent and XMotionEvent share this layout (`detail` is the
/// keycode, the button, or is_hint).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Key {
    pub base: Any,
    pub root: c_ulong,
    pub subwindow: c_ulong,
    pub time: c_ulong,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: u32,
    pub keycode: u32,
    pub same_screen: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Crossing {
    pub base: Any,
    pub root: c_ulong,
    pub subwindow: c_ulong,
    pub time: c_ulong,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub mode: c_int,
    pub detail: c_int,
    pub same_screen: c_int,
    pub focus: c_int,
    pub state: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Focus {
    pub base: Any,
    pub mode: c_int,
    pub detail: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Configure {
    pub base: Any,
    pub window: c_ulong,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border: c_int,
    pub above: c_ulong,
    pub override_redirect: c_int,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Client {
    pub base: Any,
    pub message_type: c_ulong,
    pub format: c_int,
    pub data: [c_long; 5],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ErrorEvent {
    pub kind: c_int,
    pub display: *mut Display,
    pub resource: c_ulong,
    pub serial: c_ulong,
    pub error_code: u8,
    pub request_code: u8,
    pub minor_code: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union Event {
    pub kind: c_int,
    pub any: Any,
    pub key: Key,
    pub crossing: Crossing,
    pub focus: Focus,
    pub configure: Configure,
    pub client: Client,
    pub pad: [c_long; 24],
}
const _: () = assert!(std::mem::size_of::<Event>() == 192);
#[repr(C)]
pub struct Color {
    pub pixel: c_ulong,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub flags: c_char,
    pub pad: c_char,
}
#[repr(C)]
#[derive(Default)]
pub struct SizeHints {
    pub flags: c_long,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub min_width: c_int,
    pub min_height: c_int,
    pub max_width: c_int,
    pub max_height: c_int,
    pub width_inc: c_int,
    pub height_inc: c_int,
    pub min_aspect: [c_int; 2],
    pub max_aspect: [c_int; 2],
    pub base_width: c_int,
    pub base_height: c_int,
    pub win_gravity: c_int,
}
