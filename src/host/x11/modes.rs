//! Actual per-output XRandR display modes. Enumeration never changes the desktop.
use super::{monitors::Monitor, Connection};
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mode {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
    pub flags: u32,
}
unsafe extern "C" {
    fn peekgame_x11_modes(
        display: *mut std::ffi::c_void,
        monitor: u64,
        out: *mut Mode,
        capacity: u32,
        current: *mut Mode,
    ) -> u32;
}
pub(crate) fn list(monitor: &Monitor) -> (Vec<Mode>, Mode) {
    let mut current = Mode {
        width: monitor.width as u32,
        height: monitor.height as u32,
        hz: 0,
        flags: 0,
    };
    let Ok(c) = Connection::shared() else {
        return (vec![current], current);
    };
    let mut modes = vec![current; 4096];
    let count = unsafe {
        peekgame_x11_modes(
            c.display,
            monitor.id,
            modes.as_mut_ptr(),
            modes.len() as u32,
            &mut current,
        )
    } as usize;
    modes.truncate(count.min(4096));
    modes.sort_by_key(|m| (m.width, m.height, m.hz, m.flags));
    modes.dedup();
    if !modes
        .iter()
        .any(|m| m.width == current.width && m.height == current.height && m.hz == current.hz)
    {
        modes.push(current);
    }
    (modes, current)
}
