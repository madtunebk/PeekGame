use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong};
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
    pub selection: Selection,
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Selection {
    pub kind: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub requestor: c_ulong,
    pub selection: c_ulong,
    pub target: c_ulong,
    pub property: c_ulong,
    pub time: c_ulong,
}

/// XGenericEventCookie: the envelope for an extension (XInput2) event. After
/// XNextEvent reports kind 35 (GenericEvent), `data` is filled by XGetEventData.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GenericEventCookie {
    pub kind: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub extension: c_int,
    pub evtype: c_int,
    pub cookie: c_uint,
    pub data: *mut std::ffi::c_void,
}
/// XIEventMask: the per-device set of XI2 event types to receive.
#[repr(C)]
pub struct IEventMask {
    pub deviceid: c_int,
    pub mask_len: c_int,
    pub mask: *mut u8,
}
/// XIValuatorState: the set of valuators that changed and their cooked values.
/// Its pointer members give it 8-byte alignment, which lands it at offset 64 in
/// XIRawEvent (4 bytes of padding after `detail`) — a nested struct reproduces
/// that padding, where a flattened one would not.
#[repr(C)]
pub struct IValuatorState {
    pub mask_len: c_int,
    pub mask: *const u8,
    pub values: *const f64,
}
/// The leading fields of XIRawEvent (XInput2.h). We read only the relative
/// valuators, so the trailing fields past `raw_values` are left off.
#[repr(C)]
pub struct IRawEvent {
    pub kind: c_int,
    pub serial: c_ulong,
    pub send_event: c_int,
    pub display: *mut Display,
    pub extension: c_int,
    pub evtype: c_int,
    pub time: c_ulong,
    pub deviceid: c_int,
    pub sourceid: c_int,
    pub detail: c_int,
    pub valuators: IValuatorState,
    /// Device values before the pointer-acceleration transform (what a game's
    /// raw-input camera look wants). Indexed in step with the set mask bits.
    pub raw_values: *const f64,
}

/// Native XWindowAttributes (LP64 Xlib layout).
#[repr(C)]
#[derive(Default)]
pub struct Attributes {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub border: i32,
    pub depth: i32,
    pub visual: usize,
    pub root: u64,
    pub class: i32,
    pub bit_gravity: i32,
    pub win_gravity: i32,
    pub backing_store: i32,
    pub backing_planes: u64,
    pub backing_pixel: u64,
    pub save_under: i32,
    pub colormap: u64,
    pub map_installed: i32,
    pub map_state: i32,
    pub all_events: i64,
    pub your_events: i64,
    pub do_not_propagate: i64,
    pub override_redirect: i32,
    pub screen: usize,
}
const _: () = assert!(std::mem::size_of::<Attributes>() == 136);
