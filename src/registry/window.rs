use super::*;
use crate::api::window::display;
exports! {
    export!(
        "user32.dll",
        "MonitorFromWindow",
        "window",
        "partial",
        "one monitor: the default X screen",
        display::monitor_from_window
    ),
    export!(
        "user32.dll",
        "MonitorFromPoint",
        "window",
        "partial",
        "one monitor: the default X screen",
        display::monitor_from_point
    ),
    export!(
        "user32.dll",
        "MonitorFromRect",
        "window",
        "partial",
        "one monitor: the default X screen",
        display::monitor_from_rect
    ),
    export!(
        "user32.dll",
        "GetMonitorInfoW",
        "window",
        "partial",
        "screen bounds; work area is the whole screen",
        display::monitor_info_w
    ),
    export!(
        "user32.dll",
        "GetMonitorInfoA",
        "window",
        "partial",
        "screen bounds; work area is the whole screen",
        display::monitor_info_a
    ),
    export!(
        "user32.dll",
        "EnumDisplayMonitors",
        "window",
        "partial",
        "the one monitor; no HDC or clip",
        display::enum_display_monitors
    ),
    export!(
        "user32.dll",
        "EnumDisplayDevicesW",
        "window",
        "partial",
        "one adapter and its monitor",
        display::enum_display_devices_w
    ),
    export!(
        "user32.dll",
        "EnumDisplayDevicesA",
        "window",
        "partial",
        "one adapter and its monitor",
        display::enum_display_devices_a
    ),
    export!(
        "user32.dll",
        "EnumDisplaySettingsW",
        "window",
        "partial",
        "current mode only; 32 bpp, 60 Hz",
        display::enum_display_settings_w
    ),
    export!(
        "user32.dll",
        "EnumDisplaySettingsA",
        "window",
        "partial",
        "current mode only; 32 bpp, 60 Hz",
        display::enum_display_settings_a
    ),
    export!(
        "user32.dll",
        "GetSystemMetrics",
        "window",
        "partial",
        "screen, monitors, mouse; no non-client area; unmapped indexes stop",
        display::system_metrics
    ),
    export!(
        "user32.dll",
        "SystemParametersInfoA",
        "window",
        "partial",
        "work area, accessibility keys, mouse defaults; others stop",
        display::system_parameters_info
    ),
    export!(
        "user32.dll",
        "SystemParametersInfoW",
        "window",
        "partial",
        "work area, accessibility keys, mouse defaults; others stop",
        display::system_parameters_info
    ),
    export!(
        "user32.dll",
        "GetDoubleClickTime",
        "window",
        "implemented",
        "Windows default 500 ms",
        display::double_click_time
    ),
    export!(
        "user32.dll",
        "GetCaretBlinkTime",
        "window",
        "implemented",
        "Windows default 530 ms",
        display::caret_blink_time
    ),
    export!(
        "user32.dll",
        "IsProcessDPIAware",
        "window",
        "implemented",
        "physical pixels everywhere",
        display::is_process_dpi_aware
    ),
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
}
