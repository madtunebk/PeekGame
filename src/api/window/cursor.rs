use super::state::{handle, STATE};
use crate::api::system::error;
pub(crate) extern "win64" fn load_cursor_w(instance: usize, name: *const u16) -> usize {
    if instance != 0 || name as usize != 32512
    /* IDC_ARROW */
    {
        error::set(50);
        return 0;
    }
    let result = STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some((handle, _)) = state.arrow {
            return Ok(handle);
        }
        let native = state.connection()?.arrow()?;
        let handle = handle();
        state.arrow = Some((handle, native));
        Ok(handle)
    });
    match result {
        Ok(handle) => handle,
        Err(code) => {
            error::set(code);
            0
        }
    }
}
