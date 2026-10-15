use crate::host::x11::Connection;
use std::{
    ffi::c_void,
    sync::{atomic::AtomicU32, Arc},
};
#[repr(C)]
pub(crate) struct RawSurface {
    pub display: *mut c_void,
    pub window: u64,
    pub alive: AtomicU32,
    pub width: AtomicU32,
    pub height: AtomicU32,
    pub visible: AtomicU32,
}
const _: () = assert!(std::mem::size_of::<RawSurface>() == 32);
pub(crate) struct Surface {
    pub raw: RawSurface,
    _connection: Arc<Connection>,
}
impl Surface {
    pub fn new(connection: Arc<Connection>, window: u64, width: u32, height: u32) -> Self {
        Self {
            raw: RawSurface {
                display: connection.display,
                window,
                alive: AtomicU32::new(1),
                width: AtomicU32::new(width),
                height: AtomicU32::new(height),
                visible: AtomicU32::new(0),
            },
            _connection: connection,
        }
    }
}
// Display operations are protected by Xlib after XInitThreads; fields shared with
// DXVK worker threads are atomic. The Arc keeps XCloseDisplay after graphics teardown.
unsafe impl Send for Surface {}
unsafe impl Sync for Surface {}
