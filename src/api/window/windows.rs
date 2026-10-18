//! Window lifetime, geometry, styles and the default window procedure. A window's
//! rectangle is its client area: the window manager draws any frame outside it, so
//! AdjustWindowRectEx leaves rectangles unchanged and a window created W x H has a
//! W x H client area.
use super::{
    classes, input, queue,
    state::{self, connection, windows, Window},
    strings,
    types::*,
};
use crate::{api::system::error, host::graphics::surface::Surface};
use std::{collections::HashMap, ffi::c_void, sync::atomic::Ordering, sync::Arc};
fn fail<T>(code: u32, value: T) -> T {
    error::set(code);
    value
}
const CW_USEDEFAULT: i32 = i32::MIN;

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
    let title_text = if title.is_null() {
        String::new()
    } else {
        unsafe { strings::wide(title) }.unwrap_or_default()
    };
    unsafe {
        create(
            ex_style,
            class,
            &title_text,
            title,
            style,
            x,
            y,
            width,
            height,
            parent,
            menu,
            instance,
            parameter,
        )
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe extern "win64" fn create_window_ex_a(
    ex_style: u32,
    class: *const i8,
    title: *const i8,
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
    let class_wide: Vec<u16>;
    let class_pointer = if (class as usize) < 65536 {
        class as *const u16
    } else {
        class_wide = unsafe { strings::narrow(class) }
            .unwrap_or_default()
            .encode_utf16()
            .chain([0])
            .collect();
        class_wide.as_ptr()
    };
    let title_text = if title.is_null() {
        String::new()
    } else {
        unsafe { strings::narrow(title) }.unwrap_or_default()
    };
    let title_wide: Vec<u16> = title_text.encode_utf16().chain([0]).collect();
    unsafe {
        create(
            ex_style,
            class_pointer,
            &title_text,
            title_wide.as_ptr(),
            style,
            x,
            y,
            width,
            height,
            parent,
            menu,
            instance,
            parameter,
        )
    }
}
#[allow(clippy::too_many_arguments)]
unsafe fn create(
    ex_style: u32,
    class: *const u16,
    title: &str,
    title_pointer: *const u16,
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
        let class_record = match unsafe { classes::find(class, instance) } {
            Ok(c) => c,
            Err(e) => {
                let class_name = if (class as usize) < 65536 {
                    format!("#{}", class as usize)
                } else {
                    unsafe { strings::wide(class) }.unwrap_or_else(|_| "invalid".into())
                };
                eprintln!("CreateWindowEx: classes::find failed for '{class_name}', instance={instance:#x}: err {e}");
                return Err(e);
            }
        };
        if trace_window() {
            eprintln!("[window] CreateWindowEx ({x},{y}) {width}x{height} style={style:#x} ex={ex_style:#x} parent={parent:#x}");
        }
        let message_only = parent == HWND_MESSAGE;
        let child = style & WS_CHILD != 0 && !message_only;
        let parent = if message_only { 0 } else { parent };
        if child && !state::exists(parent) {
            eprintln!("CreateWindowEx: child window but parent {parent:#x} does not exist");
            return Err(1406); // ERROR_TLW_WITH_WSCHILD
        }
        let parent_native = if child {
            windows().get(&parent).map(|w| w.native).unwrap_or(0)
        } else {
            0
        };
        let (screen_w, screen_h) = crate::host::x11::primary_size().unwrap_or((1920, 1080));
        let (mut w, mut h) = (width, height);
        if w == CW_USEDEFAULT {
            w = screen_w * 3 / 4;
            h = screen_h * 3 / 4;
        }
        let (w, h) = (w.clamp(0, 32767), h.clamp(0, 32767));
        let (x, y) = (
            if x == CW_USEDEFAULT { 0 } else { x },
            if y == CW_USEDEFAULT || x == CW_USEDEFAULT {
                0
            } else {
                y
            },
        );
        let hwnd = state::handle();
        let (native, surface) = if message_only {
            (0, None)
        } else {
            let c = match connection() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("CreateWindowEx: connection() failed: err {e}");
                    return Err(e);
                }
            };
            let cursor = input::x_cursor_for(class_record.cursor);
            let native = match c.create(
                parent_native,
                title,
                x,
                y,
                w.max(1) as u32,
                h.max(1) as u32,
                cursor,
            ) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("CreateWindowEx: c.create failed: err {e}");
                    return Err(e);
                }
            };
            if !child {
                if style & WS_CAPTION != WS_CAPTION {
                    c.set_decorated(native, false);
                }
                if style & WS_THICKFRAME == 0 {
                    c.set_resizable(native, false, w, h);
                }
            }
            let surface = Arc::new(Surface::new(
                c.clone(),
                native,
                w.max(1) as u32,
                h.max(1) as u32,
            ));
            (native, Some(surface))
        };
        windows().insert(
            hwnd,
            Window {
                native,
                surface,
                procedure: class_record.procedure,
                class: class_record.atom,
                style: style & !(WS_VISIBLE | WS_MINIMIZE | WS_MAXIMIZE),
                ex_style,
                parent: if child { parent } else { 0 },
                owner: state::current_thread(),
                instance,
                id: if child { menu } else { 0 },
                user_data: 0,
                extra: vec![0; class_record.window_extra],
                title: title.to_owned(),
                props: HashMap::new(),
                x,
                y,
                width: w,
                height: h,
                visible: false,
                minimized: false,
                maximized: false,
                fullscreen_monitor:None,
                destroying: false,
                paint: false,
                icons: [0; 2],
                track_leave: false,
                expected: None,
                expected_from: None,
                map_guard: None,
                own_unmaps: 0,
            },
        );
        let create = CreateStruct {
            parameter,
            instance,
            menu,
            parent,
            height: h,
            width: w,
            y,
            x,
            style: style as i32,
            title: title_pointer,
            class,
            ex_style,
        };
        let lparam = &create as *const _ as isize;
        // No lock is held while Windows code runs.
        if queue::call(hwnd, WM_NCCREATE, 0, lparam) == 0 {
            eprintln!("CreateWindowEx: WM_NCCREATE returned 0 (refused creation)");
            destroy(hwnd, false);
            return Err(31); // the window procedure refused creation
        }
        if !state::exists(hwnd) {
            eprintln!("CreateWindowEx: window destroyed during WM_NCCREATE");
            return Err(ERROR_WINDOW);
        }
        if queue::call(hwnd, WM_CREATE, 0, lparam) == -1 {
            eprintln!("CreateWindowEx: WM_CREATE returned -1 (refused creation)");
            destroy(hwnd, true);
            return Err(31);
        }
        if !state::exists(hwnd) {
            eprintln!("CreateWindowEx: window destroyed during WM_CREATE");
            return Err(ERROR_WINDOW);
        }
        let (cw, ch, cx, cy) = state::with_window(hwnd, |w| (w.width, w.height, w.x, w.y))
            .unwrap_or((w, h, x, y));
        queue::call(hwnd, WM_SIZE, 0, pack(cw, ch));
        queue::call(hwnd, WM_MOVE, 0, pack(cx, cy));
        if !state::exists(hwnd) {
            return Err(ERROR_WINDOW);
        }
        update_fullscreen(hwnd);
        if style & WS_VISIBLE != 0 {
            show_window(
                hwnd,
                if style & WS_MAXIMIZE != 0 {
                    3
                } else if style & WS_MINIMIZE != 0 {
                    2
                } else {
                    5
                },
            );
        }
        Ok(hwnd)
    })();
    result.unwrap_or_else(|code| fail(code, 0))
}

/// Removes a window and its children (WM_DESTROY first when `notify`).
fn destroy(hwnd: usize, notify: bool) -> bool {
    let started = state::with_window(hwnd, |w| {
        let first = !w.destroying;
        w.destroying = true;
        if let Some(s) = &w.surface {
            s.raw.alive.store(0, Ordering::Release);
        }
        first
    });
    if started != Ok(true) {
        return false;
    }
    if notify {
        queue::call(hwnd, WM_DESTROY, 0, 0);
    }
    let children: Vec<usize> = windows()
        .iter()
        .filter(|(_, w)| w.parent == hwnd)
        .map(|(&h, _)| h)
        .collect();
    for c in children {
        destroy(c, notify);
    }
    if notify {
        queue::call(hwnd, WM_NCDESTROY, 0, 0);
    }
    super::ole_drop::forget(hwnd);
    input::forget_window(hwnd);
    queue::drop_window(hwnd);
    let removed = {
        let mut list = windows();
        let w = list.remove(&hwnd);
        // A child's X window goes with its parent's when the parent is being destroyed.
        let parent_going = w
            .as_ref()
            .is_some_and(|w| w.parent != 0 && list.get(&w.parent).is_none_or(|p| p.destroying));
        w.filter(|_| !parent_going)
    };
    if let Some(w) = removed {
        if w.native != 0 {
            if let Ok(c) = connection() {
                c.destroy(w.native);
            }
        }
    }
    true
}
pub(crate) extern "win64" fn destroy_window(hwnd: usize) -> i32 {
    if destroy(hwnd, true) {
        1
    } else {
        fail(ERROR_WINDOW, 0)
    }
}

fn set_visible(hwnd: usize, visible: bool) -> Option<bool> {
    let (native, old) = state::with_window(hwnd, |w| {
        let old = w.visible;
        w.visible = visible;
        if visible {
            w.paint = true;
        }
        // Window managers may apply their own initial size when they first manage a
        // window (GNOME restores DOOM's 32x32 creation size over the 1280x720 it set
        // before ShowWindow). On Windows the program's size stands.
        if visible && !old && w.native != 0 && w.parent == 0 && !w.minimized && !w.maximized {
            w.map_guard = Some((w.width, w.height, std::time::Instant::now() + std::time::Duration::from_secs(2)));
        }
        if let Some(s) = &w.surface {
            s.raw.visible.store(visible as u32, Ordering::Release);
        }
        if old && !visible && w.native != 0 {
            w.own_unmaps += 1;
        }
        (w.native, old)
    })
    .ok()?;
    if native != 0 && old != visible {
        if let Ok(c) = connection() {
            c.show(native, visible);
        }
    }
    if old != visible {
        queue::call(hwnd, WM_SHOWWINDOW, visible as usize, 0);
    }
    Some(old)
}
/// ShowWindow: hide, show (with or without activation), minimize, maximize and
/// restore. Returns whether the window was visible.
pub(crate) extern "win64" fn show_window(hwnd: usize, command: i32) -> i32 {
    if trace_window() {
        eprintln!("[window] ShowWindow {hwnd:#x} command={command}");
    }
    let Ok((native, minimized, maximized)) =
        state::with_window(hwnd, |w| (w.native, w.minimized, w.maximized))
    else {
        return fail(ERROR_WINDOW, 0);
    };
    let c = connection().ok();
    let old = match command {
        0 => set_visible(hwnd, false),
        2 | 6 | 7 | 11 => {
            let old = set_visible(hwnd, true);
            let _ = state::with_window(hwnd, |w| w.minimized = true);
            if let (Some(c), true) = (&c, native != 0) {
                c.iconify(native);
            }
            old
        }
        3 => {
            let old = set_visible(hwnd, true);
            let _ = state::with_window(hwnd, |w| w.maximized = true);
            if let (Some(c), true) = (&c, native != 0) {
                c.set_state(
                    native,
                    true,
                    c"_NET_WM_STATE_MAXIMIZED_VERT",
                    Some(c"_NET_WM_STATE_MAXIMIZED_HORZ"),
                );
            }
            old
        }
        1 | 4 | 5 | 8 | 9 | 10 => {
            let old = set_visible(hwnd, true);
            if (command == 1 || command == 9) && (minimized || maximized) {
                let _ = state::with_window(hwnd, |w| {
                    w.minimized = false;
                    w.maximized = false;
                });
                if let (Some(c), true) = (&c, native != 0 && maximized) {
                    c.set_state(
                        native,
                        false,
                        c"_NET_WM_STATE_MAXIMIZED_VERT",
                        Some(c"_NET_WM_STATE_MAXIMIZED_HORZ"),
                    );
                }
            }
            if !matches!(command, 4 | 8) {
                input::activate_on_map(hwnd);
            }
            old
        }
        _ => return fail(87, 0),
    };
    old.unwrap_or(false) as i32
}
/// CloseWindow minimizes (it does not destroy): ShowWindow(SW_MINIMIZE) with a
/// success result instead of the previous visibility.
pub(crate) extern "win64" fn close_window(hwnd: usize) -> i32 {
    if trace_window() {
        eprintln!("[window] CloseWindow {hwnd:#x}");
    }
    if state::with_window(hwnd, |_| ()).is_err() {
        return fail(ERROR_WINDOW, 0);
    }
    show_window(hwnd, 6);
    1
}
pub(crate) unsafe extern "win64" fn adjust_window_rect_ex(
    rect: *mut Rect,
    _style: u32,
    menu: i32,
    _ex_style: u32,
) -> i32 {
    if rect.is_null() {
        return fail(87, 0);
    }
    if menu != 0 {
        return fail(50, 0); // menus are not supported
    }
    if trace_window() {
        let r = unsafe { *rect };
        eprintln!("[window] AdjustWindowRectEx ({},{})-({},{}) unchanged (no frame here)", r.left, r.top, r.right, r.bottom);
    }
    1
}
pub(crate) unsafe extern "win64" fn adjust_window_rect_ex_for_dpi(
    rect: *mut Rect,
    style: u32,
    menu: i32,
    ex_style: u32,
    _dpi: u32,
) -> i32 {
    unsafe { adjust_window_rect_ex(rect, style, menu, ex_style) }
}
pub(crate) unsafe extern "win64" fn adjust_window_rect(
    rect: *mut Rect,
    style: u32,
    menu: i32,
) -> i32 {
    unsafe { adjust_window_rect_ex(rect, style, menu, 0) }
}

/// Applies a position/size change and tells the window (WM_WINDOWPOSCHANGING/CHANGED).
/// `PEEKGAME_TRACE_WINDOW`: log CreateWindowEx, ShowWindow, CloseWindow, SetWindowPos,
/// style changes, AdjustWindowRectEx, window placement and client-size changes.
fn trace_window() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PEEKGAME_TRACE_WINDOW").is_some())
}
pub(crate) extern "win64" fn set_window_pos(
    hwnd: usize,
    after: usize,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    flags: u32,
) -> i32 {
    if trace_window() {
        eprintln!("[window] SetWindowPos {hwnd:#x} after={after:#x} ({x},{y}) {cx}x{cy} flags={flags:#x}");
    }
    let Ok((native, parent, style, ox, oy, ow, oh)) = state::with_window(hwnd, |w| {
        (w.native, w.parent, w.style, w.x, w.y, w.width, w.height)
    }) else {
        return fail(ERROR_WINDOW, 0);
    };
    let mut pos = WindowPos {
        window: hwnd,
        after,
        x,
        y,
        width: cx,
        height: cy,
        flags,
    };
    if flags & 0x400 == 0 {
        queue::call(hwnd, WM_WINDOWPOSCHANGING, 0, &mut pos as *mut _ as isize);
    }
    let flags = pos.flags;
    let (nx, ny) = if flags & 2 != 0 {
        (ox, oy)
    } else {
        (pos.x, pos.y)
    };
    let (nw, nh) = if flags & 1 != 0 {
        (ow, oh)
    } else {
        (pos.width.max(0), pos.height.max(0))
    };
    let _ = state::with_window(hwnd, |w| {
        w.x = nx;
        w.y = ny;
        if (nw, nh) != (ow, oh) {
            w.width = nw;
            w.height = nh;
            w.expected = Some((nw, nh));
            w.expected_from = Some((ow, oh));
            if let Some((_, _, until)) = w.map_guard {
                w.map_guard = Some((nw, nh, until));
            }
            w.paint = true;
            if let Some(s) = &w.surface {
                s.raw.width.store(nw.max(1) as u32, Ordering::Release);
                s.raw.height.store(nh.max(1) as u32, Ordering::Release);
            }
        }
    });
    if native != 0 {
        if let Ok(c) = connection() {
            if flags & 3 != 3 {
                // Fixed-size hints first: the window manager applies the hints current
                // when it handles the request, so the old size (min = max) would clamp
                // the new one (DOOM's 32x32 creation size stuck in windowed mode).
                if parent == 0 && style & WS_THICKFRAME == 0 {
                    c.set_resizable(native, false, nw, nh);
                }
                c.move_resize(
                    native,
                    nx,
                    ny,
                    nw.max(1) as u32,
                    nh.max(1) as u32,
                    parent == 0,
                );
            }
            if flags & 4 == 0 && (after == 0 || after == usize::MAX) && flags & 0x10 == 0 {
                c.raise_and_focus(native);
            }
        }
    }
    update_fullscreen(hwnd);
    if flags & 0x40 != 0 {
        show_window(hwnd, if flags & 0x10 != 0 { 8 } else { 5 });
    } else if flags & 0x80 != 0 {
        show_window(hwnd, 0);
    }
    pos.x = nx;
    pos.y = ny;
    pos.width = nw;
    pos.height = nh;
    if (nx, ny) == (ox, oy) {
        pos.flags |= 2;
    }
    if (nw, nh) == (ow, oh) {
        pos.flags |= 1;
    }
    queue::call(hwnd, WM_WINDOWPOSCHANGED, 0, &pos as *const _ as isize);
    1
}
pub(crate) extern "win64" fn move_window(
    hwnd: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    _repaint: i32,
) -> i32 {
    set_window_pos(hwnd, 0, x, y, width, height, 4 | 0x10)
}
pub(crate) extern "win64" fn bring_window_to_top(hwnd: usize) -> i32 {
    set_window_pos(hwnd, 0, 0, 0, 0, 0, 3)
}

/// GetWindowLongPtr values: GWL_EXSTYLE -20, GWL_STYLE -16, GWLP_ID -12,
/// GWLP_HWNDPARENT -8, GWLP_HINSTANCE -6, GWLP_WNDPROC -4, GWLP_USERDATA -21,
/// and the window's extra bytes from 0.
fn get_long(hwnd: usize, index: i32, bytes: usize) -> isize {
    let value = state::with_window(hwnd, |w| match index {
        -20 => Some(w.ex_style as isize),
        -16 => Some(
            (w.style
                | if w.visible { WS_VISIBLE } else { 0 }
                | if w.minimized { WS_MINIMIZE } else { 0 }
                | if w.maximized { WS_MAXIMIZE } else { 0 }) as i32 as isize,
        ),
        -12 => Some(w.id as isize),
        -8 => Some(w.parent as isize),
        -6 => Some(w.instance as isize),
        -4 => Some(w.procedure as usize as isize),
        -21 => Some(w.user_data),
        i if i >= 0 && (i as usize) + bytes <= w.extra.len() => {
            let mut v = [0u8; 8];
            v[..bytes].copy_from_slice(&w.extra[i as usize..i as usize + bytes]);
            Some(if bytes == 4 {
                i32::from_le_bytes(v[..4].try_into().unwrap()) as isize
            } else {
                isize::from_le_bytes(v)
            })
        }
        _ => None,
    });
    match value {
        Ok(Some(v)) => {
            error::set(0);
            v
        }
        Ok(None) => fail(1413, 0), // ERROR_INVALID_INDEX
        Err(e) => fail(e, 0),
    }
}
fn set_long(hwnd: usize, index: i32, value: isize, bytes: usize) -> isize {
    if trace_window() && matches!(index, -16 | -20) {
        let name = if index == -16 { "GWL_STYLE" } else { "GWL_EXSTYLE" };
        eprintln!("[window] SetWindowLong {hwnd:#x} {name}={value:#x}");
    }
    let old = get_long(hwnd, index, bytes);
    if error::get_last_error() != 0 {
        return 0;
    }
    let mut style_change = None;
    let _ = state::with_window(hwnd, |w| match index {
        -20 => w.ex_style = value as u32,
        -16 => {
            let style = value as u32;
            style_change = Some((w.native, w.parent, w.style, style, w.width, w.height));
            w.style = style & !(WS_VISIBLE | WS_MINIMIZE | WS_MAXIMIZE);
        }
        -12 => w.id = value as usize,
        -6 => w.instance = value as usize,
        -4 => {
            if value != 0 {
                w.procedure = unsafe { std::mem::transmute::<isize, WndProc>(value) };
            }
        }
        -21 => w.user_data = value,
        i if i >= 0 => {
            let i = i as usize;
            w.extra[i..i + bytes].copy_from_slice(&value.to_le_bytes()[..bytes]);
        }
        _ => {}
    });
    if let Some((native, parent, old_style, style, width, height)) = style_change {
        if native != 0 && parent == 0 {
            if let Ok(c) = connection() {
                if (old_style ^ style) & WS_CAPTION != 0 {
                    c.set_decorated(native, style & WS_CAPTION == WS_CAPTION);
                }
                if (old_style ^ style) & WS_THICKFRAME != 0 {
                    c.set_resizable(native, style & WS_THICKFRAME != 0, width, height);
                }
            }
        }
    }
    if index==-16 {update_fullscreen(hwnd);}
    error::set(0);
    old
}
pub(crate) extern "win64" fn get_window_long_ptr_w(hwnd: usize, index: i32) -> isize {
    get_long(hwnd, index, 8)
}
pub(crate) extern "win64" fn set_window_long_ptr_w(hwnd: usize, index: i32, value: isize) -> isize {
    set_long(hwnd, index, value, 8)
}
pub(crate) extern "win64" fn get_window_long_w(hwnd: usize, index: i32) -> i32 {
    get_long(hwnd, index, 4) as i32
}
pub(crate) extern "win64" fn set_window_long_w(hwnd: usize, index: i32, value: i32) -> i32 {
    set_long(hwnd, index, value as isize, 4) as i32
}

pub(crate) unsafe extern "win64" fn get_client_rect(hwnd: usize, rect: *mut Rect) -> i32 {
    if rect.is_null() {
        return fail(87, 0);
    }
    // The desktop window is the primary monitor, as on Windows (not the virtual screen).
    let size = if hwnd == DESKTOP {
        crate::host::x11::primary_size()
    } else {
        state::with_window(hwnd, |w| {
            if w.minimized {
                (0, 0)
            } else {
                (w.width, w.height)
            }
        })
        .ok()
    };
    match size {
        Some((w, h)) => {
            if trace_window() && hwnd != DESKTOP {
                // Only changes, so per-frame polling does not flood the log.
                static LAST: std::sync::Mutex<Option<(usize, i32, i32)>> = std::sync::Mutex::new(None);
                let mut last = LAST.lock().unwrap();
                if *last != Some((hwnd, w, h)) {
                    *last = Some((hwnd, w, h));
                    eprintln!("[window] GetClientRect {hwnd:#x} -> {w}x{h}");
                }
            }
            unsafe {
                rect.write(Rect {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                })
            };
            1
        }
        None => fail(ERROR_WINDOW, 0),
    }
}
/// Screen rectangle of a window (its client area: the frame is the compositor's).
pub(crate) fn screen_rect(hwnd: usize) -> Option<Rect> {
    if hwnd == DESKTOP {
        let (right, bottom) = crate::host::x11::primary_size()?;
        return Some(Rect {
            left: 0,
            top: 0,
            right,
            bottom,
        });
    }
    let (width, height, native, x, y) =
        state::with_window(hwnd, |w| (w.width, w.height, w.native, w.x, w.y)).ok()?;
    let (left, top) = if native != 0 {
        state::client_origin(hwnd).unwrap_or((x, y))
    } else {
        (x, y)
    };
    Some(Rect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
}
pub(crate) unsafe extern "win64" fn get_window_rect(hwnd: usize, rect: *mut Rect) -> i32 {
    if rect.is_null() {
        return fail(87, 0);
    }
    match screen_rect(hwnd) {
        Some(r) => {
            unsafe { rect.write(r) };
            1
        }
        None => fail(ERROR_WINDOW, 0),
    }
}
pub(crate) unsafe extern "win64" fn get_window_info(hwnd: usize, info: *mut WindowInfo) -> i32 {
    if info.is_null() {
        return fail(87, 0);
    }
    if unsafe { (*info).size } != std::mem::size_of::<WindowInfo>() as u32 {
        return fail(87, 0);
    }
    if hwnd == DESKTOP {
        let (w, h) = crate::host::x11::primary_size().unwrap_or((1920, 1080));
        let rect = Rect {
            left: 0,
            top: 0,
            right: w,
            bottom: h,
        };
        unsafe {
            (*info).window = rect;
            (*info).client = rect;
            (*info).style = WS_VISIBLE;
            (*info).ex_style = 0;
            (*info).window_status = 1;
            (*info).border_x = 0;
            (*info).border_y = 0;
            (*info).window_type = 0;
            (*info).creator_version = 0x0400;
        }
        return 1;
    }

    let win = state::with_window(hwnd, |w| {
        (
            w.style,
            w.ex_style,
            w.class,
            w.minimized,
            w.x,
            w.y,
            w.width,
            w.height,
        )
    });
    let Ok((style, ex_style, class, minimized, x, y, width, height)) = win else {
        return fail(ERROR_WINDOW, 0);
    };

    let w_rect = screen_rect(hwnd).unwrap_or(Rect {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    });
    let c_rect = if minimized {
        Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        }
    } else {
        w_rect
    };

    unsafe {
        (*info).window = w_rect;
        (*info).client = c_rect;
        (*info).style = style;
        (*info).ex_style = ex_style;
        (*info).window_status = 1;
        (*info).border_x = 0;
        (*info).border_y = 0;
        (*info).window_type = class;
        (*info).creator_version = 0x0400;
    }
    1
}
pub(crate) unsafe extern "win64" fn client_to_screen(hwnd: usize, point: *mut Point) -> i32 {
    let Some(r) = screen_rect(hwnd) else {
        return fail(ERROR_WINDOW, 0);
    };
    if point.is_null() {
        return fail(87, 0);
    }
    unsafe {
        (*point).x += r.left;
        (*point).y += r.top;
    }
    1
}
pub(crate) unsafe extern "win64" fn screen_to_client(hwnd: usize, point: *mut Point) -> i32 {
    let Some(r) = screen_rect(hwnd) else {
        return fail(ERROR_WINDOW, 0);
    };
    if point.is_null() {
        return fail(87, 0);
    }
    unsafe {
        let before = *point;
        (*point).x -= r.left;
        (*point).y -= r.top;
        static LAST: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(i64::MIN);
        let packed = (before.x as u32 as u64 | ((before.y as u32 as u64) << 32)) as i64;
        if LAST.swap(packed, Ordering::Relaxed) != packed {
            input::trace_input(format_args!(
                "ScreenToClient hwnd={hwnd:#x} {},{} -> {},{} rect={r:?}",
                before.x,
                before.y,
                (*point).x,
                (*point).y
            ));
        }
    }
    1
}
/// MapWindowPoints: the offset between the two windows' client origins in the low
/// and high words.
pub(crate) unsafe extern "win64" fn map_window_points(
    from: usize,
    to: usize,
    points: *mut Point,
    count: u32,
) -> i32 {
    let origin = |h: usize| {
        if h == 0 {
            Some(Rect::default())
        } else {
            screen_rect(h)
        }
    };
    let (Some(a), Some(b)) = (origin(from), origin(to)) else {
        return fail(ERROR_WINDOW, 0);
    };
    let (dx, dy) = (a.left - b.left, a.top - b.top);
    for i in 0..count as usize {
        unsafe {
            (*points.add(i)).x += dx;
            (*points.add(i)).y += dy;
        }
    }
    pack(dx, dy) as i32
}
pub(crate) extern "win64" fn is_window(hwnd: usize) -> i32 {
    (state::exists(hwnd) || hwnd == DESKTOP) as i32
}
pub(crate) extern "win64" fn is_window_visible(hwnd: usize) -> i32 {
    let windows = windows();
    let mut current = hwnd;
    loop {
        match windows.get(&current) {
            Some(w) if w.visible => {
                if w.parent == 0 {
                    return 1;
                }
                current = w.parent;
            }
            _ => return 0,
        }
    }
}
pub(crate) extern "win64" fn is_iconic(hwnd: usize) -> i32 {
    state::with_window(hwnd, |w| w.minimized as i32).unwrap_or(0)
}
pub(crate) extern "win64" fn is_zoomed(hwnd: usize) -> i32 {
    state::with_window(hwnd, |w| w.maximized as i32).unwrap_or(0)
}
pub(crate) extern "win64" fn is_window_enabled(hwnd: usize) -> i32 {
    state::exists(hwnd) as i32
}
pub(crate) extern "win64" fn enable_window(_hwnd: usize, _enable: i32) -> i32 {
    0
}
pub(crate) extern "win64" fn get_parent(hwnd: usize) -> usize {
    state::with_window(hwnd, |w| w.parent).unwrap_or_else(|e| fail(e, 0))
}
pub(crate) extern "win64" fn get_desktop_window() -> usize {
    DESKTOP
}
/// GetAncestor: GA_PARENT 1, GA_ROOT 2, GA_ROOTOWNER 3.
pub(crate) extern "win64" fn get_ancestor(hwnd: usize, kind: u32) -> usize {
    let windows = windows();
    let Some(w) = windows.get(&hwnd) else {
        return 0;
    };
    match kind {
        1 => {
            if w.parent == 0 {
                DESKTOP
            } else {
                w.parent
            }
        }
        2 | 3 => {
            let mut current = hwnd;
            while let Some(p) = windows.get(&current).map(|w| w.parent).filter(|&p| p != 0) {
                current = p;
            }
            current
        }
        _ => 0,
    }
}
/// GetWindow: GW_HWNDFIRST 0, LAST 1, NEXT 2, PREV 3, OWNER 4, CHILD 5.
pub(crate) extern "win64" fn get_window(hwnd: usize, command: u32) -> usize {
    let windows = windows();
    let Some(parent) = windows.get(&hwnd).map(|w| w.parent) else {
        return fail(ERROR_WINDOW, 0);
    };
    let siblings: Vec<usize> = windows
        .iter()
        .filter(|(_, w)| w.parent == parent && w.native != 0)
        .map(|(&h, _)| h)
        .collect();
    let position = siblings.iter().position(|&h| h == hwnd);
    match command {
        0 => siblings.first().copied().unwrap_or(0),
        1 => siblings.last().copied().unwrap_or(0),
        2 => position
            .and_then(|i| siblings.get(i + 1))
            .copied()
            .unwrap_or(0),
        3 => position
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| siblings.get(i))
            .copied()
            .unwrap_or(0),
        5 => windows
            .iter()
            .find(|(_, w)| w.parent == hwnd)
            .map(|(&h, _)| h)
            .unwrap_or(0),
        _ => 0,
    }
}
pub(crate) extern "win64" fn get_top_window(hwnd: usize) -> usize {
    if hwnd == 0 || hwnd == DESKTOP {
        return windows()
            .iter()
            .find(|(_, w)| w.parent == 0 && w.native != 0)
            .map(|(&h, _)| h)
            .unwrap_or(0);
    }
    get_window(hwnd, 5)
}
type EnumProc = unsafe extern "win64" fn(usize, isize) -> i32;
fn enumerate(list: Vec<usize>, callback: Option<EnumProc>, parameter: isize) -> i32 {
    let Some(callback) = callback else {
        return fail(87, 0);
    };
    for h in list {
        if state::exists(h) && unsafe { callback(h, parameter) } == 0 {
            break;
        }
    }
    1
}
/// EnumWindows: top-level windows with an X window (message-only windows are not
/// enumerated, as on Windows).
pub(crate) extern "win64" fn enum_windows(callback: Option<EnumProc>, parameter: isize) -> i32 {
    let list = windows()
        .iter()
        .filter(|(_, w)| w.parent == 0 && w.native != 0)
        .map(|(&h, _)| h)
        .collect();
    enumerate(list, callback, parameter)
}
pub(crate) extern "win64" fn enum_child_windows(
    parent: usize,
    callback: Option<EnumProc>,
    parameter: isize,
) -> i32 {
    let list = windows()
        .iter()
        .filter(|(_, w)| w.parent == parent && parent != 0)
        .map(|(&h, _)| h)
        .collect();
    enumerate(list, callback, parameter)
}
pub(crate) extern "win64" fn enum_thread_windows(
    thread: u32,
    callback: Option<EnumProc>,
    parameter: isize,
) -> i32 {
    let list = windows()
        .iter()
        .filter(|(_, w)| w.owner == thread && w.parent == 0)
        .map(|(&h, _)| h)
        .collect();
    enumerate(list, callback, parameter)
}
pub(crate) unsafe extern "win64" fn get_window_thread_process_id(
    hwnd: usize,
    process: *mut u32,
) -> u32 {
    let owner = if hwnd == DESKTOP {
        Some(0)
    } else {
        state::with_window(hwnd, |w| w.owner).ok()
    };
    match owner {
        Some(tid) => {
            if !process.is_null() {
                unsafe { *process = crate::api::system::startup::process_id() };
            }
            tid
        }
        None => fail(ERROR_WINDOW, 0),
    }
}

fn set_text(hwnd: usize, text: String) -> i32 {
    let native = state::with_window(hwnd, |w| {
        w.title = text.clone();
        (w.native, w.parent)
    });
    match native {
        Ok((native, 0)) if native != 0 => {
            if let Ok(c) = connection() {
                c.set_title(native, &text);
            }
            1
        }
        Ok(_) => 1,
        Err(e) => fail(e, 0),
    }
}
pub(crate) unsafe extern "win64" fn set_window_text_w(hwnd: usize, text: *const u16) -> i32 {
    let text = if text.is_null() {
        String::new()
    } else {
        unsafe { strings::wide(text) }.unwrap_or_default()
    };
    set_text(hwnd, text)
}
pub(crate) unsafe extern "win64" fn set_window_text_a(hwnd: usize, text: *const i8) -> i32 {
    let text = if text.is_null() {
        String::new()
    } else {
        unsafe { strings::narrow(text) }.unwrap_or_default()
    };
    set_text(hwnd, text)
}
pub(crate) unsafe extern "win64" fn get_window_text_w(
    hwnd: usize,
    out: *mut u16,
    capacity: i32,
) -> i32 {
    match state::with_window(hwnd, |w| w.title.clone()) {
        Ok(t) => unsafe { strings::copy_w(&t, out, capacity) },
        Err(e) => fail(e, 0),
    }
}
pub(crate) unsafe extern "win64" fn get_window_text_a(
    hwnd: usize,
    out: *mut i8,
    capacity: i32,
) -> i32 {
    match state::with_window(hwnd, |w| w.title.clone()) {
        Ok(t) => unsafe { strings::copy_a(&t, out, capacity) },
        Err(e) => fail(e, 0),
    }
}
pub(crate) extern "win64" fn get_window_text_length_w(hwnd: usize) -> i32 {
    state::with_window(hwnd, |w| w.title.encode_utf16().count() as i32)
        .unwrap_or_else(|e| fail(e, 0))
}
unsafe fn prop_name(name: *const u16) -> String {
    if (name as usize) < 65536 {
        format!("#{}", name as usize)
    } else {
        unsafe { strings::wide(name) }
            .unwrap_or_default()
            .to_lowercase()
    }
}
pub(crate) unsafe extern "win64" fn set_prop_w(hwnd: usize, name: *const u16, value: usize) -> i32 {
    let name = unsafe { prop_name(name) };
    state::with_window(hwnd, |w| {
        w.props.insert(name, value);
        1
    })
    .unwrap_or_else(|e| fail(e, 0))
}
pub(crate) unsafe extern "win64" fn get_prop_w(hwnd: usize, name: *const u16) -> usize {
    let name = unsafe { prop_name(name) };
    state::with_window(hwnd, |w| w.props.get(&name).copied().unwrap_or(0)).unwrap_or(0)
}
pub(crate) unsafe extern "win64" fn remove_prop_w(hwnd: usize, name: *const u16) -> usize {
    let name = unsafe { prop_name(name) };
    state::with_window(hwnd, |w| w.props.remove(&name).unwrap_or(0)).unwrap_or(0)
}
/// WINDOWPLACEMENT: length 44, flags, showCmd, ptMinPosition, ptMaxPosition,
/// rcNormalPosition.
#[repr(C)]
pub struct Placement {
    length: u32,
    flags: u32,
    show: u32,
    min: Point,
    max: Point,
    normal: Rect,
}
pub(crate) unsafe extern "win64" fn get_window_placement(hwnd: usize, out: *mut Placement) -> i32 {
    if out.is_null() {
        return fail(87, 0);
    }
    let Some(rect) = screen_rect(hwnd) else {
        return fail(ERROR_WINDOW, 0);
    };
    let (minimized, maximized, visible) =
        state::with_window(hwnd, |w| (w.minimized, w.maximized, w.visible)).unwrap_or_default();
    if trace_window() {
        eprintln!("[window] GetWindowPlacement {hwnd:#x} normal=({},{})-({},{}) min={minimized} max={maximized}", rect.left, rect.top, rect.right, rect.bottom);
    }
    unsafe {
        out.write(Placement {
            length: 44,
            flags: 0,
            show: if minimized {
                2
            } else if maximized {
                3
            } else if visible {
                1
            } else {
                0
            },
            min: Point { x: -1, y: -1 },
            max: Point { x: -1, y: -1 },
            normal: rect,
        })
    };
    1
}
pub(crate) unsafe extern "win64" fn set_window_placement(
    hwnd: usize,
    placement: *const Placement,
) -> i32 {
    if placement.is_null() {
        return fail(87, 0);
    }
    let p = unsafe { &*placement };
    let r = p.normal;
    if trace_window() {
        eprintln!("[window] SetWindowPlacement {hwnd:#x} normal=({},{})-({},{}) show={}", r.left, r.top, r.right, r.bottom, p.show);
    }
    set_window_pos(
        hwnd,
        0,
        r.left,
        r.top,
        r.right - r.left,
        r.bottom - r.top,
        4 | 0x10,
    );
    show_window(hwnd, p.show as i32);
    1
}
pub(crate) extern "win64" fn update_window(hwnd: usize) -> i32 {
    let pending = state::with_window(hwnd, |w| w.paint);
    match pending {
        Ok(true) => {
            queue::call(hwnd, WM_PAINT, 0, 0);
            1
        }
        Ok(false) => 1,
        Err(e) => fail(e, 0),
    }
}
pub(crate) extern "win64" fn invalidate_rect(hwnd: usize, _rect: *const Rect, _erase: i32) -> i32 {
    if hwnd == 0 {
        for w in windows().values_mut() {
            w.paint = true;
        }
        return 1;
    }
    state::with_window(hwnd, |w| {
        w.paint = true;
        1
    })
    .unwrap_or_else(|e| fail(e, 0))
}
pub(crate) extern "win64" fn invalidate_rgn(hwnd: usize, _hrgn: usize, erase: i32) -> i32 {
    invalidate_rect(hwnd, std::ptr::null(), erase)
}
pub(crate) unsafe extern "win64" fn get_update_rect(
    hwnd: usize,
    rect: *mut Rect,
    _erase: i32,
) -> i32 {
    state::with_window(hwnd, |w| {
        if w.paint {
            if !rect.is_null() {
                unsafe {
                    *rect = Rect {
                        left: 0,
                        top: 0,
                        right: w.width,
                        bottom: w.height,
                    };
                }
            }
            1
        } else {
            0
        }
    })
    .unwrap_or_else(|e| fail(e, 0))
}
pub(crate) extern "win64" fn set_window_display_affinity(hwnd: usize, affinity: u32) -> i32 {
    if !state::exists(hwnd) {
        return fail(1400, 0);
    }
    if affinity != 0 && affinity != 1 && affinity != 0x11 {
        return fail(87, 0);
    }
    1
}
pub(crate) extern "win64" fn validate_rect(hwnd: usize, _rect: *const Rect) -> i32 {
    if hwnd == 0 {
        return 1;
    }
    state::with_window(hwnd, |w| {
        w.paint = false;
        1
    })
    .unwrap_or_else(|e| fail(e, 0))
}
/// RedrawWindow: RDW_INVALIDATE (1) marks for WM_PAINT, RDW_VALIDATE (8) clears,
/// RDW_UPDATENOW (0x100) paints now.
pub(crate) extern "win64" fn redraw_window(
    hwnd: usize,
    _rect: *const Rect,
    _region: usize,
    flags: u32,
) -> i32 {
    if flags & 3 != 0 {
        invalidate_rect(hwnd, std::ptr::null(), 0);
    }
    if flags & 8 != 0 && _region == 0 {
        validate_rect(hwnd, std::ptr::null());
    }
    if flags & 0x100 != 0 {
        if hwnd != 0 {
            update_window(hwnd);
        } else {
            let hwnds: Vec<usize> = state::windows().keys().copied().collect();
            for h in hwnds {
                update_window(h);
            }
        }
    }
    1
}
/// PAINTSTRUCT (x64): hdc, fErase, rcPaint, fRestore, fIncUpdate, rgbReserved[32].
pub(crate) unsafe extern "win64" fn begin_paint(hwnd: usize, paint: *mut u8) -> usize {
    let Ok((width, height)) = state::with_window(hwnd, |w| {
        w.paint = false;
        (w.width, w.height)
    }) else {
        return fail(ERROR_WINDOW, 0);
    };
    let dc = super::misc::get_dc(hwnd);
    if !paint.is_null() {
        unsafe {
            paint.write_bytes(0, 72);
            paint.cast::<usize>().write_unaligned(dc);
            paint.add(12).cast::<Rect>().write_unaligned(Rect {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            });
        }
    }
    dc
}
pub(crate) extern "win64" fn end_paint(_hwnd: usize, _paint: *const u8) -> i32 {
    1
}
pub(crate) extern "win64" fn flash_window_ex(_info: *const u8) -> i32 {
    1
}

/// DefWindowProc: what Windows does for messages a window procedure passes on.
pub(crate) extern "win64" fn def_window_proc_w(
    hwnd: usize,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match message {
        0x80 => super::icons::set(hwnd, wparam, lparam as usize), // WM_SETICON
        0x7f => state::with_window(hwnd, |w| w.icons.get(wparam).copied().unwrap_or(0) as isize)
            .unwrap_or(0), // WM_GETICON
        WM_NCCREATE => 1,
        WM_CLOSE => {
            destroy_window(hwnd);
            0
        }
        WM_PAINT => {
            let _ = state::with_window(hwnd, |w| w.paint = false);
            0
        }
        WM_ERASEBKGND => 1,
        WM_NCHITTEST => 1, // HTCLIENT
        WM_NCACTIVATE => 1,
        WM_SETCURSOR => {
            // In the client area the class cursor is shown.
            if lparam & 0xffff == 1 {
                if let Ok(atom) = state::with_window(hwnd, |w| w.class) {
                    if let Some(class) = classes::by_atom(atom) {
                        if class.cursor != 0 {
                            input::set_cursor(class.cursor);
                        }
                    }
                }
            }
            0
        }
        WM_SETTEXT => {
            if lparam != 0 {
                let text = unsafe { strings::wide(lparam as *const u16) }.unwrap_or_default();
                set_text(hwnd, text);
            }
            1
        }
        WM_WINDOWPOSCHANGED if lparam != 0 => {
            // WM_MOVE and WM_SIZE unless WINDOWPOS says SWP_NOMOVE/SWP_NOSIZE.
            let pos = unsafe { *(lparam as *const WindowPos) };
            if pos.flags & 2 == 0 {
                let (x, y) = state::client_origin(hwnd).unwrap_or((pos.x, pos.y));
                queue::call(hwnd, WM_MOVE, 0, pack(x, y));
            }
            if pos.flags & 1 == 0 {
                let kind = state::with_window(hwnd, |w| {
                    if w.minimized {
                        1
                    } else if w.maximized {
                        2
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
                queue::call(hwnd, WM_SIZE, kind, pack(pos.width, pos.height));
            }
            0
        }
        WM_SYSCOMMAND => {
            match wparam & 0xfff0 {
                0xf060 => {
                    queue::call(hwnd, WM_CLOSE, 0, 0);
                }
                0xf020 => {
                    show_window(hwnd, 6);
                }
                0xf030 => {
                    show_window(hwnd, 3);
                }
                0xf120 => {
                    show_window(hwnd, 9);
                }
                _ => {}
            }
            0
        }
        WM_SYSKEYDOWN if wparam == 0x73 && lparam & (1 << 29) != 0 => {
            // Alt+F4
            queue::call(hwnd, WM_SYSCOMMAND, 0xf060, 0);
            0
        }
        WM_ACTIVATE => {
            if wparam & 0xffff != 0 {
                input::set_focus(hwnd);
            }
            0
        }
        _ => 0,
    }
}
pub(crate) extern "win64" fn call_window_proc_w(
    procedure: Option<WndProc>,
    hwnd: usize,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match procedure {
        Some(p) => unsafe { p(hwnd, message, wparam, lparam) },
        None => 0,
    }
}

/// Borderless windows covering a monitor use EWMH fullscreen on that monitor.
/// This applies to every engine and uses the actual monitor layout, never a
/// fixed resolution. Keep fixed-size hints out of the WM's fullscreen resize.
pub(super) fn update_fullscreen(hwnd:usize){
 let Ok((native,parent,style,x,y,w,h,old))=state::with_window(hwnd,|v|(v.native,v.parent,v.style,v.x,v.y,v.width,v.height,v.fullscreen_monitor)) else{return};
 if native==0||parent!=0{return}
 let all=crate::host::x11::monitors();
 let monitor=crate::host::x11::monitors::select(&all,[x,y,x+w,y+h],0).filter(|m|style&WS_CAPTION!=WS_CAPTION&&x<=m.x&&y<=m.y&&x+w>=m.x+m.width&&y+h>=m.y+m.height);
 if let Ok(c)=connection(){
  c.set_resizable(native,monitor.is_some()||style&WS_THICKFRAME!=0,w,h);
  let chosen=monitor.map(|m|m.id);
  if chosen!=old{let _=state::with_window(hwnd,|v|v.fullscreen_monitor=chosen);c.fullscreen(native,monitor);}
 }
}

