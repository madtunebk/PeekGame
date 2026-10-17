//! COM context-token query. A successful token must be a real IObjContext.
use super::apartment;
const E_POINTER: i32 = 0x80004003u32 as i32;
const CO_E_NOTINITIALIZED: i32 = 0x800401f0u32 as i32;
const E_NOTIMPL: i32 = 0x80004001u32 as i32;
/// No AddRef or fake success: initialized apartments have no context object yet.
pub unsafe extern "win64" fn get_token(out: *mut usize) -> i32 {
    if out.is_null() {
        return E_POINTER;
    }
    unsafe { out.write(0) };
    if !apartment::initialized() {
        return CO_E_NOTINITIALIZED;
    }
    E_NOTIMPL
}
