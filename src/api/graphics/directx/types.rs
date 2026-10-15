use std::ffi::c_void;
pub const E_FAIL: i32 = 0x80004005u32 as i32;
pub const E_POINTER: i32 = 0x80004003u32 as i32;
pub const E_NOINTERFACE: i32 = 0x80004002u32 as i32;
pub const E_INVALIDARG: i32 = 0x80070057u32 as i32;
pub const DXGI_UNSUPPORTED: i32 = 0x887a0004u32 as i32;
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Guid {
    pub a: u32,
    pub b: u16,
    pub c: u16,
    pub d: [u8; 8],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SwapDesc {
    pub width: u32,
    pub height: u32,
    pub refresh: [u32; 2],
    pub format: u32,
    pub scanline: u32,
    pub scaling: u32,
    pub sample: [u32; 2],
    pub usage: u32,
    pub count: u32,
    pub window: usize,
    pub windowed: i32,
    pub effect: u32,
    pub flags: u32,
}
#[repr(C)]
pub struct Mapped {
    pub data: *mut c_void,
    pub row_pitch: u32,
    pub depth_pitch: u32,
}
const _: () = assert!(std::mem::size_of::<SwapDesc>() == 72);
const _: () = assert!(std::mem::size_of::<Mapped>() == 16);
