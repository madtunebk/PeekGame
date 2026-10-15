//! Window classes: process-wide, matched by atom or case-insensitive name (the
//! registering module's classes first).
use super::{
    state::{Class, CLASSES},
    strings,
    types::{WindowClass, WindowClassPlain, WndProc},
};
use crate::api::system::error;
pub(super) unsafe fn find(name: *const u16, instance: usize) -> Result<Class, u32> {
    let atom = (name as usize) < 65536;
    let text = if atom {
        None
    } else {
        Some(unsafe { strings::wide(name) }?)
    };
    let classes = CLASSES.lock().unwrap();
    let matching = |c: &&Class| match &text {
        Some(n) => c.name.eq_ignore_ascii_case(n),
        None => c.atom as usize == name as usize,
    };
    classes
        .iter()
        .filter(matching)
        .find(|c| c.instance == instance)
        .or_else(|| classes.iter().find(matching))
        .cloned()
        .ok_or(1407) // ERROR_CANNOT_FIND_WND_CLASS
}
pub(super) fn by_atom(atom: u16) -> Option<Class> {
    CLASSES
        .lock()
        .unwrap()
        .iter()
        .find(|c| c.atom == atom)
        .cloned()
}
#[allow(clippy::too_many_arguments)]
unsafe fn register(
    procedure: Option<WndProc>,
    name: *const u16,
    instance: usize,
    style: u32,
    cursor: usize,
    window_extra: i32,
) -> u16 {
    let result = (|| {
        let procedure = procedure.ok_or(87u32)?;
        if !(0..=40960).contains(&window_extra) {
            return Err(87);
        }
        let name = unsafe { strings::wide(name) }?;
        if name.is_empty() || name.encode_utf16().count() > 255 {
            return Err(87);
        }
        let mut classes = CLASSES.lock().unwrap();
        if classes
            .iter()
            .any(|c| c.instance == instance && c.name.eq_ignore_ascii_case(&name))
        {
            return Err(1410); // ERROR_CLASS_ALREADY_EXISTS
        }
        let atom = 0xc000
            + classes
                .iter()
                .map(|c| c.atom - 0xc000 + 1)
                .max()
                .unwrap_or(0);
        classes.push(Class {
            atom,
            name,
            procedure,
            instance,
            cursor,
            style,
            window_extra: window_extra as usize,
        });
        Ok(atom)
    })();
    result.unwrap_or_else(|code| {
        error::set(code);
        0
    })
}
/// RegisterClassExW. Icons, brushes and menus are accepted and not drawn: the
/// compositor and the program's renderer own the window's pixels.
pub(crate) unsafe extern "win64" fn register_class_ex_w(class: *const WindowClass) -> u16 {
    if class.is_null() || unsafe { (*class).size } != std::mem::size_of::<WindowClass>() as u32 {
        error::set(87);
        return 0;
    }
    let c = unsafe { &*class };
    unsafe {
        register(
            c.procedure,
            c.name,
            c.instance,
            c.style,
            c.cursor,
            c.window_extra,
        )
    }
}
pub(crate) unsafe extern "win64" fn register_class_w(class: *const WindowClassPlain) -> u16 {
    if class.is_null() {
        error::set(87);
        return 0;
    }
    let c = unsafe { &*class };
    unsafe {
        register(
            c.procedure,
            c.name,
            c.instance,
            c.style,
            c.cursor,
            c.window_extra,
        )
    }
}
pub(crate) unsafe extern "win64" fn unregister_class_w(name: *const u16, instance: usize) -> i32 {
    let Ok(class) = (unsafe { find(name, instance) }) else {
        error::set(1411); // ERROR_CLASS_DOES_NOT_EXIST
        return 0;
    };
    if super::state::windows()
        .values()
        .any(|w| w.class == class.atom)
    {
        error::set(1412); // ERROR_CLASS_HAS_WINDOWS
        return 0;
    }
    CLASSES.lock().unwrap().retain(|c| c.atom != class.atom);
    1
}
/// GetClassNameW: length copied, without the NUL.
pub(crate) unsafe extern "win64" fn get_class_name_w(
    hwnd: usize,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    let Some(atom) = super::state::windows().get(&hwnd).map(|w| w.class) else {
        error::set(super::types::ERROR_WINDOW);
        return 0;
    };
    let name: Vec<u16> = by_atom(atom)
        .map(|c| c.name)
        .unwrap_or_default()
        .encode_utf16()
        .collect();
    if out.is_null() || capacity <= 0 {
        error::set(122);
        return 0;
    }
    let n = name.len().min(capacity as usize - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(name.as_ptr(), out, n);
        *out.add(n) = 0;
    }
    n as i32
}
