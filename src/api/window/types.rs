use std::ffi::c_void;
pub type WndProc = unsafe extern "win64" fn(usize, u32, usize, isize) -> isize;
/// WNDCLASSEXW (x64).
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
/// WNDCLASSW (x64): WNDCLASSEXW without cbSize and hIconSm.
#[repr(C)]
pub struct WindowClassPlain {
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
}
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
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
#[derive(Clone, Copy, Default, Debug, PartialEq)]
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
/// WINDOWPOS (x64).
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WindowPos {
    pub window: usize,
    pub after: usize,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub flags: u32,
}
/// WINDOWINFO (Win32).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WindowInfo {
    pub size: u32,
    pub window: Rect,
    pub client: Rect,
    pub style: u32,
    pub ex_style: u32,
    pub window_status: u32,
    pub border_x: u32,
    pub border_y: u32,
    pub window_type: u16,
    pub creator_version: u16,
}
const _: () = assert!(std::mem::size_of::<WindowClass>() == 80);
const _: () = assert!(std::mem::size_of::<WindowClassPlain>() == 72);
const _: () = assert!(std::mem::size_of::<Message>() == 48);
const _: () = assert!(std::mem::size_of::<CreateStruct>() == 80);
const _: () = assert!(std::mem::size_of::<WindowPos>() == 40);
const _: () = assert!(std::mem::size_of::<WindowInfo>() == 60);
pub const WM_CREATE: u32 = 1;
pub const WM_DESTROY: u32 = 2;
pub const WM_MOVE: u32 = 3;
pub const WM_SIZE: u32 = 5;
pub const WM_ACTIVATE: u32 = 6;
pub const WM_SETFOCUS: u32 = 7;
pub const WM_KILLFOCUS: u32 = 8;
pub const WM_SETTEXT: u32 = 0x0c;
pub const WM_PAINT: u32 = 0x0f;
pub const WM_CLOSE: u32 = 0x10;
pub const WM_QUIT: u32 = 0x12;
pub const WM_ERASEBKGND: u32 = 0x14;
pub const WM_SHOWWINDOW: u32 = 0x18;
pub const WM_ACTIVATEAPP: u32 = 0x1c;
pub const WM_SETCURSOR: u32 = 0x20;
pub const WM_WINDOWPOSCHANGING: u32 = 0x46;
pub const WM_WINDOWPOSCHANGED: u32 = 0x47;
pub const WM_NCCREATE: u32 = 0x81;
pub const WM_NCDESTROY: u32 = 0x82;
pub const WM_NCHITTEST: u32 = 0x84;
pub const WM_NCACTIVATE: u32 = 0x86;
pub const WM_INPUT: u32 = 0xff;
pub const WM_KEYDOWN: u32 = 0x100;
pub const WM_KEYUP: u32 = 0x101;
pub const WM_CHAR: u32 = 0x102;
pub const WM_SYSKEYDOWN: u32 = 0x104;
pub const WM_SYSKEYUP: u32 = 0x105;
pub const WM_SYSCHAR: u32 = 0x106;
pub const WM_SYSCOMMAND: u32 = 0x112;
pub const WM_TIMER: u32 = 0x113;
pub const WM_MOUSEMOVE: u32 = 0x200;
pub const WM_MOUSEWHEEL: u32 = 0x20a;
pub const WM_XBUTTONDOWN: u32 = 0x20b;
pub const WM_MOUSEHWHEEL: u32 = 0x20e;
pub const WM_MOUSELEAVE: u32 = 0x2a3;
pub const ERROR_WINDOW: u32 = 1400;
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_MINIMIZE: u32 = 0x2000_0000;
pub const WS_MAXIMIZE: u32 = 0x0100_0000;
pub const WS_CAPTION: u32 = 0x00c0_0000;
pub const WS_THICKFRAME: u32 = 0x0004_0000;
pub const HWND_MESSAGE: usize = usize::MAX - 2;
pub const DESKTOP: usize = 0x0001_0010;
pub(crate) fn pack(low: i32, high: i32) -> isize {
    ((high as u32 & 0xffff) << 16 | (low as u32 & 0xffff)) as i32 as isize
}
