use super::*;
pub(super) static EXPORTS: &[Export] = &[
    window!(
        "RegisterClassExW",
        "process-wide application classes; no extra bytes, icons, menus or brushes",
        classes::register_class_ex_w
    ),
    window!(
        "CreateWindowExW",
        "decorated top-level X11 windows; no child windows, menus or extended styles",
        create::create_window_ex_w
    ),
    window!(
        "AdjustWindowRectEx",
        "client-size layout model; compositor owns external borders; menus unsupported",
        lifecycle::adjust_window_rect_ex
    ),
    window!(
        "ShowWindow",
        "hide, normal, show and show-without-activation; no minimize/maximize yet",
        lifecycle::show_window
    ),
    window!(
        "DestroyWindow",
        "owner-thread lifecycle callbacks and X11 resource destruction",
        lifecycle::destroy_window
    ),
    window!(
        "DefWindowProcW",
        "WM_NCCREATE and WM_CLOSE defaults; other default behavior pending",
        lifecycle::def_window_proc_w
    ),
    window!(
        "LoadCursorW",
        "shared system arrow on owner X11 connection; custom cursors unsupported",
        cursor::load_cursor_w
    ),
    window!(
        "PeekMessageW",
        "owner-thread queue, filters, no-remove/remove; native resize, close and basic keys",
        messages::peek_message_w
    ),
    window!(
        "PostMessageW",
        "same-thread windows and current-thread messages; no broadcast/cross-thread yet",
        messages::post_message_w
    ),
    window!(
        "PostQuitMessage",
        "current-thread quit notification",
        messages::post_quit_message
    ),
    window!(
        "DispatchMessageW",
        "calls live owner-thread WndProc with Windows x64 ABI",
        messages::dispatch_message_w
    ),
    window!(
        "TranslateMessage",
        "ASCII from native letter/digit/space keys; extended text/IME pending",
        messages::translate_message
    ),
];
