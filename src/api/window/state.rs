use super::types::{Message, WndProc};
use crate::host::{graphics::surface::Surface, x11::Connection};
use std::sync::Arc;
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    sync::atomic::{AtomicUsize, Ordering},
};
pub(super) struct Window {
    pub native: u64,
    pub surface: Arc<Surface>,
    pub procedure: WndProc,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
    pub destroying: bool,
}
#[derive(Default)]
pub(super) struct State {
    pub connection: Option<Arc<Connection>>,
    pub windows: BTreeMap<usize, Window>,
    pub queue: VecDeque<Message>,
    pub arrow: Option<(usize, u64)>,
}
thread_local! {pub(super) static STATE:RefCell<State>=RefCell::new(State::default());}
pub(super) fn handle() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0x10000);
    NEXT.fetch_add(1, Ordering::Relaxed)
}
impl State {
    pub fn connection(&mut self) -> Result<&Connection, u32> {
        if self.connection.is_none() {
            self.connection = Some(Arc::new(Connection::open()?));
        }
        Ok(self.connection.as_deref().unwrap())
    }
}
