//! Windows and classes, process-wide (any thread may ask about a window; only its
//! owner thread runs its window procedure for messages it retrieves).
use super::types::WndProc;
use crate::host::{graphics::surface::Surface, x11::Connection};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};
pub(super) struct Window {
    /// The X window; 0 for a message-only window.
    pub native: u64,
    pub surface: Option<Arc<Surface>>,
    pub procedure: WndProc,
    pub class: u16,
    pub style: u32,
    pub ex_style: u32,
    pub parent: usize,
    pub owner: u32,
    pub instance: usize,
    pub id: usize,
    pub user_data: isize,
    pub extra: Vec<u8>,
    pub title: String,
    pub props: HashMap<String, usize>,
    /// Client size; position of the client origin (screen, or parent client for
    /// children) as last known.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen_monitor:Option<u64>,
    pub destroying: bool,
    pub paint: bool,
    pub icons: [usize; 2],
    pub track_leave: bool,
    /// Size set by PeekGame itself, whose X confirmation is not news.
    pub expected: Option<(i32, i32)>,
    /// The size before that request: a ConfigureNotify with it is stale and dropped.
    /// Any other size while `expected` is pending is the window manager's decision.
    pub expected_from: Option<(i32, i32)>,
    /// After a top-level window is mapped: the size the program gave it and until when
    /// a different size from the window manager is answered by requesting it again.
    pub map_guard: Option<(i32, i32, std::time::Instant)>,
    /// Unmaps PeekGame asked for, whose UnmapNotify is not a minimize.
    pub own_unmaps: u32,
}
#[derive(Clone)]
pub(super) struct Class {
    pub atom: u16,
    pub name: String,
    pub procedure: WndProc,
    pub instance: usize,
    pub cursor: usize,
    pub style: u32,
    pub window_extra: usize,
}
pub(super) static WINDOWS: Mutex<BTreeMap<usize, Window>> = Mutex::new(BTreeMap::new());
pub(super) static CLASSES: Mutex<Vec<Class>> = Mutex::new(Vec::new());
pub(super) fn windows() -> MutexGuard<'static, BTreeMap<usize, Window>> {
    WINDOWS.lock().unwrap()
}
/// HWND values: small even numbers, never X ids or pointers.
pub(super) fn handle() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0x10020);
    NEXT.fetch_add(2, Ordering::Relaxed)
}
pub(super) fn connection() -> Result<Arc<Connection>, u32> {
    Connection::shared()
}
pub(super) fn current_thread() -> u32 {
    crate::api::system::startup::thread_id()
}
/// Runs `f` on a window, or fails with ERROR_INVALID_WINDOW_HANDLE.
pub(super) fn with_window<T>(hwnd: usize, f: impl FnOnce(&mut Window) -> T) -> Result<T, u32> {
    windows()
        .get_mut(&hwnd)
        .map(f)
        .ok_or(super::types::ERROR_WINDOW)
}
pub(super) fn exists(hwnd: usize) -> bool {
    windows().contains_key(&hwnd)
}
/// The window whose X window is `native`.
pub(super) fn by_native(native: u64) -> Option<usize> {
    windows()
        .iter()
        .find(|(_, w)| w.native == native && native != 0)
        .map(|(&h, _)| h)
}
/// Screen position of a window's client origin.
pub(super) fn client_origin(hwnd: usize) -> Option<(i32, i32)> {
    let native = windows().get(&hwnd).map(|w| w.native)?;
    if native == 0 {
        return Some((0, 0));
    }
    Some(connection().ok()?.origin(native))
}
