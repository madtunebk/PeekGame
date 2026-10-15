//! Windows sizes and documented constants; these are not observed game values.
pub type Handle = usize;
pub type WinBool = i32;
pub const INVALID_HANDLE: Handle = usize::MAX;
pub const ERROR_INVALID_HANDLE: u32 = 6;
pub const ERROR_NOT_SUPPORTED: u32 = 50;
pub const ERROR_INVALID_PARAMETER: u32 = 87;
pub const ERROR_ALREADY_EXISTS: u32 = 183;
pub const ERROR_INVALID_STATE: u32 = 5023;
