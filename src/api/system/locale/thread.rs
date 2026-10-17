//! Calling-thread LCID state, inherited when the runtime creates a Windows thread.
use crate::api::system::error;
use std::cell::Cell;
thread_local! { static LCID: Cell<Option<u32>> = const { Cell::new(None) }; }
pub extern "win64" fn get() -> u32 {
    LCID.with(|slot| {
        if let Some(id) = slot.get() {
            return id;
        }
        let id = super::user_default_lcid();
        slot.set(Some(id));
        id
    })
}
pub(crate) fn inherit(id: u32) {
    LCID.with(|slot| slot.set(Some(id)));
    crate::host::teb::set_locale(id);
}
pub extern "win64" fn set(id: u32) -> i32 {
    let id = match id {
        0x400 | 0x800 => super::user_default_lcid(),
        0xc00 | 0x1000 | 0x1400 => {
            error::set(50);
            return 0;
        } // custom locales unavailable
        id => id,
    };
    if id != 0x7f
        && !super::TABLE
            .iter()
            .any(|(_, known)| id == u32::from(*known))
    {
        error::set(87);
        return 0;
    }
    inherit(id);
    1
}
