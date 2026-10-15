use std::ffi::{c_int, c_long, c_ulong, c_void};
pub type Display = c_void;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Any {
    pub kind: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub window: c_ulong,
}
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
pub union Event {
    pub kind: c_int,
    pub any: Any,
    pub key: Key,
    pub configure: Configure,
    pub client: Client,
    pub pad: [c_long; 24],
}
const _: () = assert!(std::mem::size_of::<Event>() == 192);
