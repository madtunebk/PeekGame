use super::{
    classes, lifecycle,
    state::{handle, Window, STATE},
    strings,
    types::*,
};
use crate::api::system::error;
use crate::host::graphics::surface::Surface;
use std::{ffi::c_void, sync::Arc};
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe extern "win64" fn create_window_ex_w(
    ex_style: u32,
    class: *const u16,
    title: *const u16,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: usize,
    menu: usize,
    instance: usize,
    parameter: *mut c_void,
) -> usize {
    let result = (|| {
        if ex_style != 0 || parent != 0 || menu != 0 || style & !0x10cf0000 != 0 {
            return Err(50);
        }
        if width <= 0 || height <= 0 || width > 65535 || height > 65535 {
            return Err(87);
        }
        let class_record = unsafe { classes::find(class, instance) }?;
        let caption = if title.is_null() {
            String::new()
        } else {
            unsafe { strings::wide(title) }?
        };
        let hwnd = handle();
        let visible = style & 0x10000000 != 0;
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            let cursor = if class_record.cursor == 0 {
                0
            } else {
                state
                    .arrow
                    .filter(|a| a.0 == class_record.cursor)
                    .ok_or(1402u32)?
                    .1
            };
            let native = state.connection()?.create(
                &caption,
                if x == i32::MIN { 0 } else { x },
                if y == i32::MIN { 0 } else { y },
                width as u32,
                height as u32,
                cursor,
            )?;
            let surface = Arc::new(Surface::new(
                state.connection.as_ref().unwrap().clone(),
                native,
                width as u32,
                height as u32,
            ));
            state.windows.insert(
                hwnd,
                Window {
                    native,
                    surface,
                    procedure: class_record.procedure,
                    width,
                    height,
                    visible: false,
                    destroying: false,
                },
            );
            Ok::<(), u32>(())
        })?;
        let create = CreateStruct {
            parameter,
            instance,
            menu,
            parent,
            height,
            width,
            y,
            x,
            style: style as i32,
            title,
            class,
            ex_style,
        };
        let proc = class_record.procedure;
        // No registry or RefCell borrow is held while entering Windows code.
        if unsafe { proc(hwnd, WM_NCCREATE, 0, &create as *const _ as isize) } == 0 {
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                if let Some(w) = s.windows.remove(&hwnd) {
                    s.connection.as_ref().unwrap().destroy(w.native);
                }
            });
            return Err(31); // ERROR_GEN_FAILURE: the application refused window creation.
        }
        if !STATE.with(|s| s.borrow().windows.contains_key(&hwnd)) {
            return Err(ERROR_WINDOW);
        }
        if unsafe { proc(hwnd, WM_CREATE, 0, &create as *const _ as isize) } == -1 {
            lifecycle::destroy_window(hwnd);
            return Err(31); // ERROR_GEN_FAILURE: the application refused window creation.
        }
        let alive = STATE.with(|s| s.borrow().windows.contains_key(&hwnd));
        if !alive {
            return Err(ERROR_WINDOW);
        }
        unsafe {
            proc(
                hwnd,
                WM_SIZE,
                0,
                ((height as u32 & 0xffff) << 16 | (width as u32 & 0xffff)) as isize,
            );
        }
        if !STATE.with(|s| s.borrow().windows.contains_key(&hwnd)) {
            return Err(ERROR_WINDOW);
        }
        if visible {
            lifecycle::show_window(hwnd, 5);
        }
        Ok(hwnd)
    })();
    match result {
        Ok(hwnd) => hwnd,
        Err(code) => {
            error::set(code);
            0
        }
    }
}
