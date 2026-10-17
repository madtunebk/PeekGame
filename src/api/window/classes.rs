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

#[repr(C)]
pub struct WindowClassExA {
    pub size: u32,
    pub style: u32,
    pub procedure: Option<WndProc>,
    pub class_extra: i32,
    pub window_extra: i32,
    pub instance: usize,
    pub icon: usize,
    pub cursor: usize,
    pub background: usize,
    pub menu: *const std::ffi::c_char,
    pub name: *const std::ffi::c_char,
    pub small_icon: usize,
}

pub(crate) unsafe extern "win64" fn register_class_ex_a(class: *const WindowClassExA) -> u16 {
    if class.is_null() || unsafe { (*class).size } != std::mem::size_of::<WindowClassExA>() as u32 {
        error::set(87);
        return 0;
    }
    let c = unsafe { &*class };
    if (c.name as usize) <= 0xffff {
        let wide = WindowClass {
            size: std::mem::size_of::<WindowClass>() as u32,
            style: c.style,
            procedure: c.procedure,
            class_extra: c.class_extra,
            window_extra: c.window_extra,
            instance: c.instance,
            icon: c.icon,
            cursor: c.cursor,
            background: c.background,
            menu: std::ptr::null(),
            name: c.name as *const u16,
            small_icon: c.small_icon,
        };
        return unsafe { register_class_ex_w(&wide) };
    }
    let name_str = match unsafe { std::ffi::CStr::from_ptr(c.name) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            error::set(87);
            return 0;
        }
    };
    let mut wide_name: Vec<u16> = name_str.encode_utf16().collect();
    wide_name.push(0);
    let wide = WindowClass {
        size: std::mem::size_of::<WindowClass>() as u32,
        style: c.style,
        procedure: c.procedure,
        class_extra: c.class_extra,
        window_extra: c.window_extra,
        instance: c.instance,
        icon: c.icon,
        cursor: c.cursor,
        background: c.background,
        menu: std::ptr::null(),
        name: wide_name.as_ptr(),
        small_icon: c.small_icon,
    };
    unsafe { register_class_ex_w(&wide) }
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
pub(crate) unsafe extern "win64" fn register_class_a(class: *const WindowClassPlain) -> u16 {
    if class.is_null() {
        error::set(87);
        return 0;
    }
    let c = unsafe { &*class };
    if (c.name as usize) <= 0xffff {
        return unsafe {
            register(
                c.procedure,
                c.name,
                c.instance,
                c.style,
                c.cursor,
                c.window_extra,
            )
        };
    }
    let Ok(name_str) = (unsafe { strings::narrow(c.name.cast()) }) else {
        error::set(87);
        return 0;
    };
    let mut wide_name: Vec<u16> = name_str.encode_utf16().collect();
    wide_name.push(0);
    unsafe {
        register(
            c.procedure,
            wide_name.as_ptr(),
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
