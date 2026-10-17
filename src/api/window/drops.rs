//! Legacy WM_DROPFILES opt-in; OLE delivery uses the separate native Xdnd backend.
use super::state;
use crate::api::system::error;
use std::sync::atomic::{AtomicBool, Ordering};
static WARNED: AtomicBool = AtomicBool::new(false);
pub extern "win64" fn accept(hwnd: usize, enabled: i32) {
    match state::with_window(hwnd, |w| {
        if enabled != 0 {
            w.ex_style |= 0x10;
        } else {
            w.ex_style &= !0x10;
        }
    }) {
        Err(code) => error::set(code),
        Ok(()) if enabled != 0 => {
            error::set(50);
            if !WARNED.swap(true, Ordering::Relaxed) {
                eprintln!("DragAcceptFiles: legacy WM_DROPFILES delivery is unsupported; OLE drops use RegisterDragDrop");
            }
        }
        Ok(()) => {}
    }
}
