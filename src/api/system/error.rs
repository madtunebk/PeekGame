use std::cell::Cell;
thread_local! { static LAST_ERROR: Cell<u32> = const { Cell::new(0) }; }
pub(crate) fn set(code: u32) {
    LAST_ERROR.set(code);
    crate::host::teb::set_error(code);
}
pub(crate) extern "win64" fn get_last_error() -> u32 {
    crate::host::teb::error().unwrap_or_else(|| LAST_ERROR.get())
}
