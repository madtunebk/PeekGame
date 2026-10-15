use super::{
    strings,
    types::{WindowClass, WndProc},
};
use crate::api::system::error;
use std::sync::{Mutex, OnceLock};
#[derive(Clone)]
pub(super) struct Class {
    pub atom: u16,
    pub name: String,
    pub procedure: WndProc,
    pub instance: usize,
    pub cursor: usize,
}
fn registry() -> &'static Mutex<Vec<Class>> {
    static CLASSES: OnceLock<Mutex<Vec<Class>>> = OnceLock::new();
    CLASSES.get_or_init(|| Mutex::new(Vec::new()))
}
pub(super) unsafe fn find(name: *const u16, instance: usize) -> Result<Class, u32> {
    let name_text = if (name as usize) > 65535 {
        Some(unsafe { strings::wide(name) }?)
    } else {
        None
    };
    let classes = registry().lock().map_err(|_| 8u32)?;
    classes
        .iter()
        .find(|c| {
            c.instance == instance
                && match &name_text {
                    Some(name) => c.name.eq_ignore_ascii_case(name),
                    None => c.atom as usize == name as usize,
                }
        })
        .cloned()
        .ok_or(1407)
}
pub(crate) unsafe extern "win64" fn register_class_ex_w(pointer: *const WindowClass) -> u16 {
    let result = (|| {
        if pointer.is_null() {
            return Err(87);
        }
        let class = unsafe { &*pointer };
        if class.size != std::mem::size_of::<WindowClass>() as u32 {
            return Err(87);
        }
        let procedure = class.procedure.ok_or(87u32)?;
        if class.style != 0
            || class.class_extra != 0
            || class.window_extra != 0
            || class.icon != 0
            || class.background != 0
            || !class.menu.is_null()
            || class.small_icon != 0
        {
            return Err(50);
        }
        let name = unsafe { strings::wide(class.name) }?;
        if name.is_empty() || name.encode_utf16().count() > 255 {
            return Err(87);
        }
        let mut classes = registry().lock().map_err(|_| 8u32)?;
        if classes
            .iter()
            .any(|c| c.instance == class.instance && c.name.eq_ignore_ascii_case(&name))
        {
            return Err(1410);
        }
        let atom = u16::try_from(classes.len() + 1).map_err(|_| 8u32)?;
        classes.push(Class {
            atom,
            name,
            procedure,
            instance: class.instance,
            cursor: class.cursor,
        });
        Ok(atom)
    })();
    match result {
        Ok(atom) => atom,
        Err(code) => {
            error::set(code);
            0
        }
    }
}
