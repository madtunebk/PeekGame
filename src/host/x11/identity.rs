//! Application identity used for desktop window grouping (X11 WM_CLASS).
use std::sync::Mutex;
static APP_ID: Mutex<Option<String>> = Mutex::new(None);
pub(crate) fn set(id: String) {
    *APP_ID.lock().unwrap() = Some(id);
}
pub(super) fn property() -> Option<Vec<u8>> {
    APP_ID.lock().unwrap().as_ref().map(|id| {
        let mut bytes = Vec::with_capacity(id.len() * 2 + 2);
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(0);
        bytes
    })
}
