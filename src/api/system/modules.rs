use crate::{
    abi::{Handle, ERROR_INVALID_STATE},
    api::system::{
        dynamic::{builtin_handle, narrow, wide},
        error,
    },
    host::process::PROCESS,
    images, registry,
};
use std::ffi::c_char;
fn main_image() -> Handle {
    match PROCESS.get() {
        Some(p) => p.image_base,
        None => {
            error::set(ERROR_INVALID_STATE);
            0
        }
    }
}
/// A loaded module by name: PeekGame's own DLLs (always loaded) or a mapped PE
/// image. GetModuleHandle never loads anything.
fn named(name: &str) -> Option<(Handle, Option<std::sync::Arc<images::Module>>)> {
    if crate::api::steam::recognizes(name) && crate::api::steam::loaded() {
        return Some((crate::api::steam::base_address(), None));
    }
    if registry::provides(name) {
        return Some((builtin_handle(name), None));
    }
    images::by_name(name).map(|m| (m.base, Some(m)))
}
fn handle(name: Option<String>, null: bool) -> Handle {
    if null {
        return main_image();
    }
    match name.as_deref().and_then(named) {
        Some((handle, _)) => handle,
        None => {
            error::set(if name.is_some() { 126 } else { 87 });
            0
        }
    }
}
pub unsafe extern "win64" fn get_module_handle_a(name: *const c_char) -> Handle {
    handle(unsafe { narrow(name) }, name.is_null())
}
pub unsafe extern "win64" fn get_module_handle_w(name: *const u16) -> Handle {
    handle(unsafe { wide(name) }, name.is_null())
}
// GetModuleHandleEx flags.
const PIN: u32 = 1;
const UNCHANGED_REFCOUNT: u32 = 2;
const FROM_ADDRESS: u32 = 4;
fn module_handle_ex(flags: u32, name: Option<String>, address: usize, out: *mut Handle) -> i32 {
    if out.is_null() {
        error::set(87);
        return 0;
    }
    unsafe { *out = 0 };
    if flags & !(PIN | UNCHANGED_REFCOUNT | FROM_ADDRESS) != 0
        || flags & (PIN | UNCHANGED_REFCOUNT) == PIN | UNCHANGED_REFCOUNT
    {
        error::set(87);
        return 0;
    }
    let found = if flags & FROM_ADDRESS != 0 {
        if let Some(m) = images::by_address(address) {
            Some((m.base, Some(m)))
        } else if crate::api::steam::recognizes_address(address) {
            Some((crate::api::steam::base_address(), None))
        } else if let Some(dll) = registry::by_address(address) {
            Some((builtin_handle(dll), None))
        } else {
            None
        }
    } else if address == 0 {
        let base = main_image();
        (base != 0).then(|| (base, images::by_handle(base)))
    } else {
        name.as_deref().and_then(named)
    };
    eprintln!("[modules] module_handle_ex flags={flags:#x} addr={address:#x} name={name:?} -> found={}", found.is_some());
    let Some((handle, module)) = found else {
        error::set(126);
        return 0;
    };
    if let Some(module) = module {
        images::handle_reference(&module, flags);
    }
    unsafe { *out = handle };
    1
}
pub unsafe extern "win64" fn get_module_handle_ex_w(
    flags: u32,
    name: *const u16,
    out: *mut Handle,
) -> i32 {
    let text = if flags & FROM_ADDRESS != 0 {
        None
    } else {
        unsafe { wide(name) }
    };
    module_handle_ex(flags, text, name as usize, out)
}
pub unsafe extern "win64" fn get_module_handle_ex_a(
    flags: u32,
    name: *const c_char,
    out: *mut Handle,
) -> i32 {
    let text = if flags & FROM_ADDRESS != 0 {
        None
    } else {
        unsafe { narrow(name) }
    };
    module_handle_ex(flags, text, name as usize, out)
}
