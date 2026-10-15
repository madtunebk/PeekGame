use std::ffi::c_void;
pub type WndProc = unsafe extern "win64" fn(usize, u32, usize, isize) -> isize;
#[repr(C)]
pub struct WindowClass {
    pub size: u32,
    pub style: u32,
    pub procedure: Option<WndProc>,
    pub class_extra: i32,
    pub window_extra: i32,
    pub instance: usize,
    pub icon: usize,
    pub cursor: usize,
    pub background: usize,
    pub menu: *const u16,
    pub name: *const u16,
    pub small_icon: usize,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Message {
    pub window: usize,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub point: Point,
    pub private: u32,
}
#[repr(C)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
#[repr(C)]
pub struct CreateStruct {
    pub parameter: *mut c_void,
    pub instance: usize,
    pub menu: usize,
    pub parent: usize,
    pub height: i32,
    pub width: i32,
    pub y: i32,
    pub x: i32,
    pub style: i32,
    pub title: *const u16,
    pub class: *const u16,
    pub ex_style: u32,
}
const _: () = assert!(std::mem::size_of::<WindowClass>() == 80);
const _: () = assert!(std::mem::size_of::<Message>() == 48);
const _: () = assert!(std::mem::size_of::<CreateStruct>() == 80);
pub const WM_CREATE: u32 = 1;
pub const WM_DESTROY: u32 = 2;
pub const WM_SIZE: u32 = 5;
pub const WM_CLOSE: u32 = 0x10;
pub const WM_QUIT: u32 = 0x12;
pub const WM_NCCREATE: u32 = 0x81;
pub const WM_NCDESTROY: u32 = 0x82;
pub const WM_KEYDOWN: u32 = 0x100;
pub const WM_KEYUP: u32 = 0x101;
pub const WM_CHAR: u32 = 0x102;
pub const ERROR_WINDOW: u32 = 1400;
