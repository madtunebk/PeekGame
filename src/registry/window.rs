use super::*;
use crate::api::window::{classes, display, input, misc, queue, windows};
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
        "process-wide classes; icons/brushes/menus accepted, not drawn",
        classes::register_class_ex_w
    ),
    window!(
        "RegisterClassW",
        "process-wide classes",
        classes::register_class_w
    ),
    window!(
        "UnregisterClassW",
        "complete",
        classes::unregister_class_w
    ),
    window!(
        "GetClassNameW",
        "complete",
        classes::get_class_name_w
    ),
    window!(
        "CreateWindowExW",
        "X11 top-level and child windows, message-only windows; client-size model",
        windows::create_window_ex_w
    ),
    window!(
        "CreateWindowExA",
        "as CreateWindowExW",
        windows::create_window_ex_a
    ),
    window!(
        "DestroyWindow",
        "children first-notified, X resources freed",
        windows::destroy_window
    ),
    window!(
        "ShowWindow",
        "hide, show, minimize, maximize, restore",
        windows::show_window
    ),
    window!(
        "AdjustWindowRectEx",
        "client-size model; compositor owns borders; menus unsupported",
        windows::adjust_window_rect_ex
    ),
    window!(
        "AdjustWindowRectExForDpi",
        "as AdjustWindowRectEx",
        windows::adjust_window_rect_ex_for_dpi
    ),
    window!(
        "AdjustWindowRect",
        "as AdjustWindowRectEx",
        windows::adjust_window_rect
    ),
    window!(
        "SetWindowPos",
        "move/resize/z-order/show; fullscreen for caption-less screen-sized windows",
        windows::set_window_pos
    ),
    window!(
        "MoveWindow",
        "as SetWindowPos",
        windows::move_window
    ),
    window!(
        "BringWindowToTop",
        "raise and focus",
        windows::bring_window_to_top
    ),
    window!(
        "GetWindowLongPtrW",
        "styles, procedure, user data, id, extra bytes",
        windows::get_window_long_ptr_w
    ),
    window!(
        "GetWindowLongPtrA",
        "as W",
        windows::get_window_long_ptr_w
    ),
    window!(
        "SetWindowLongPtrW",
        "styles (decorations follow), subclassing, user data, extra bytes",
        windows::set_window_long_ptr_w
    ),
    window!(
        "SetWindowLongPtrA",
        "as W",
        windows::set_window_long_ptr_w
    ),
    window!(
        "GetWindowLongW",
        "32-bit values",
        windows::get_window_long_w
    ),
    window!(
        "GetWindowLongA",
        "32-bit values",
        windows::get_window_long_w
    ),
    window!(
        "SetWindowLongW",
        "32-bit values",
        windows::set_window_long_w
    ),
    window!(
        "SetWindowLongA",
        "32-bit values",
        windows::set_window_long_w
    ),
    window!(
        "GetClientRect",
        "complete",
        windows::get_client_rect
    ),
    window!(
        "GetWindowRect",
        "client area on screen (frame is the compositor's)",
        windows::get_window_rect
    ),
    window!(
        "ClientToScreen",
        "complete",
        windows::client_to_screen
    ),
    window!(
        "ScreenToClient",
        "complete",
        windows::screen_to_client
    ),
    window!(
        "MapWindowPoints",
        "complete",
        windows::map_window_points
    ),
    window!(
        "IsWindow",
        "complete",
        windows::is_window
    ),
    window!(
        "IsWindowVisible",
        "complete",
        windows::is_window_visible
    ),
    window!(
        "IsIconic",
        "complete",
        windows::is_iconic
    ),
    window!(
        "IsZoomed",
        "complete",
        windows::is_zoomed
    ),
    window!(
        "IsWindowEnabled",
        "windows are always enabled",
        windows::is_window_enabled
    ),
    window!(
        "EnableWindow",
        "windows are always enabled",
        windows::enable_window
    ),
    window!(
        "GetParent",
        "complete",
        windows::get_parent
    ),
    window!(
        "GetAncestor",
        "complete",
        windows::get_ancestor
    ),
    window!(
        "GetWindow",
        "siblings and first child",
        windows::get_window
    ),
    window!(
        "GetDesktopWindow",
        "the screen",
        windows::get_desktop_window
    ),
    window!(
        "EnumWindows",
        "top-level windows",
        windows::enum_windows
    ),
    window!(
        "EnumChildWindows",
        "direct children",
        windows::enum_child_windows
    ),
    window!(
        "EnumThreadWindows",
        "complete",
        windows::enum_thread_windows
    ),
    window!(
        "GetWindowThreadProcessId",
        "complete",
        windows::get_window_thread_process_id
    ),
    window!(
        "SetWindowTextW",
        "X title",
        windows::set_window_text_w
    ),
    window!(
        "SetWindowTextA",
        "X title",
        windows::set_window_text_a
    ),
    window!(
        "GetWindowTextW",
        "complete",
        windows::get_window_text_w
    ),
    window!(
        "GetWindowTextA",
        "complete",
        windows::get_window_text_a
    ),
    window!(
        "GetWindowTextLengthW",
        "complete",
        windows::get_window_text_length_w
    ),
    window!(
        "SetPropW",
        "complete",
        windows::set_prop_w
    ),
    window!(
        "GetPropW",
        "complete",
        windows::get_prop_w
    ),
    window!(
        "RemovePropW",
        "complete",
        windows::remove_prop_w
    ),
    window!(
        "GetWindowPlacement",
        "complete",
        windows::get_window_placement
    ),
    window!(
        "SetWindowPlacement",
        "position and show state",
        windows::set_window_placement
    ),
    window!(
        "UpdateWindow",
        "sends a pending WM_PAINT",
        windows::update_window
    ),
    window!(
        "InvalidateRect",
        "whole-window update region",
        windows::invalidate_rect
    ),
    window!(
        "ValidateRect",
        "whole-window update region",
        windows::validate_rect
    ),
    window!(
        "RedrawWindow",
        "invalidate, validate, update now",
        windows::redraw_window
    ),
    window!(
        "BeginPaint",
        "validates; no GDI drawing",
        windows::begin_paint
    ),
    window!(
        "EndPaint",
        "complete",
        windows::end_paint
    ),
    window!(
        "FlashWindowEx",
        "no-op",
        windows::flash_window_ex
    ),
    window!(
        "DefWindowProcW",
        "close, paint, cursor, move/size, system commands, Alt+F4",
        windows::def_window_proc_w
    ),
    window!(
        "DefWindowProcA",
        "as W",
        windows::def_window_proc_w
    ),
    window!(
        "CallWindowProcW",
        "complete",
        windows::call_window_proc_w
    ),
    window!(
        "CallWindowProcA",
        "complete",
        windows::call_window_proc_w
    ),
    window!(
        "PeekMessageW",
        "per-thread queues, filters, sent messages, WM_PAINT, timers",
        queue::peek_message_w
    ),
    window!(
        "PeekMessageA",
        "as W",
        queue::peek_message_w
    ),
    window!(
        "GetMessageW",
        "as PeekMessage, blocking",
        queue::get_message_w
    ),
    window!(
        "GetMessageA",
        "as W",
        queue::get_message_w
    ),
    window!(
        "WaitMessage",
        "complete",
        queue::wait_message
    ),
    window!(
        "PostMessageW",
        "any thread, broadcast",
        queue::post_message_w
    ),
    window!(
        "PostMessageA",
        "as W",
        queue::post_message_w
    ),
    window!(
        "PostThreadMessageW",
        "complete",
        queue::post_thread_message_w
    ),
    window!(
        "PostThreadMessageA",
        "complete",
        queue::post_thread_message_w
    ),
    window!(
        "PostQuitMessage",
        "current-thread quit notification",
        queue::post_quit_message
    ),
    window!(
        "SendMessageW",
        "same thread directly, other threads through their queue",
        queue::send_message_w
    ),
    window!(
        "SendMessageA",
        "as W",
        queue::send_message_w
    ),
    window!(
        "SendMessageTimeoutW",
        "complete",
        queue::send_message_timeout_w
    ),
    window!(
        "SendMessageTimeoutA",
        "complete",
        queue::send_message_timeout_w
    ),
    window!(
        "DispatchMessageW",
        "window procedures and TIMERPROCs",
        queue::dispatch_message_w
    ),
    window!(
        "DispatchMessageA",
        "as W",
        queue::dispatch_message_w
    ),
    window!(
        "TranslateMessage",
        "WM_CHAR/WM_SYSCHAR from the X layout",
        misc::translate_message
    ),
    window!(
        "GetMessageTime",
        "complete",
        queue::get_message_time
    ),
    window!(
        "GetMessagePos",
        "complete",
        queue::get_message_pos
    ),
    window!(
        "GetMessageExtraInfo",
        "0",
        queue::get_message_extra_info
    ),
    window!(
        "InSendMessage",
        "0",
        queue::in_send_message
    ),
    window!(
        "SetTimer",
        "window and thread timers",
        queue::set_timer
    ),
    window!(
        "KillTimer",
        "complete",
        queue::kill_timer
    ),
    window!(
        "MsgWaitForMultipleObjects",
        "objects and queue input",
        queue::msg_wait
    ),
    window!(
        "MsgWaitForMultipleObjectsEx",
        "objects, queue input, alertable",
        queue::msg_wait_ex
    ),
    window!(
        "RegisterWindowMessageW",
        "complete",
        misc::register_window_message_w
    ),
    window!(
        "RegisterWindowMessageA",
        "complete",
        misc::register_window_message_a
    ),
    window!(
        "GetKeyState",
        "from the message stream",
        input::get_key_state
    ),
    window!(
        "GetAsyncKeyState",
        "current key state",
        input::get_async_key_state
    ),
    window!(
        "GetKeyboardState",
        "complete",
        input::get_keyboard_state
    ),
    window!(
        "GetKeyboardLayout",
        "the user's language",
        input::get_keyboard_layout
    ),
    window!(
        "GetKeyboardLayoutNameW",
        "the user's language",
        input::get_keyboard_layout_name_w
    ),
    window!(
        "GetKeyboardLayoutList",
        "one layout",
        input::get_keyboard_layout_list
    ),
    window!(
        "ActivateKeyboardLayout",
        "one layout",
        input::activate_keyboard_layout
    ),
    window!(
        "MapVirtualKeyW",
        "Windows scan codes; characters from the X layout",
        input::map_virtual_key_w
    ),
    window!(
        "MapVirtualKeyA",
        "as W",
        input::map_virtual_key_w
    ),
    window!(
        "MapVirtualKeyExW",
        "as MapVirtualKey",
        input::map_virtual_key_ex_w
    ),
    window!(
        "MapVirtualKeyExA",
        "as MapVirtualKey",
        input::map_virtual_key_ex_w
    ),
    window!(
        "ToUnicode",
        "X layout; no dead keys",
        input::to_unicode
    ),
    window!(
        "ToUnicodeEx",
        "X layout; no dead keys",
        input::to_unicode_ex
    ),
    window!(
        "GetKeyNameTextW",
        "Windows key names",
        input::get_key_name_text_w
    ),
    window!(
        "GetFocus",
        "X keyboard focus",
        input::get_focus
    ),
    window!(
        "SetFocus",
        "X input focus",
        input::set_focus
    ),
    window!(
        "GetActiveWindow",
        "complete",
        input::get_active_window
    ),
    window!(
        "SetActiveWindow",
        "raise and focus",
        input::set_active_window
    ),
    window!(
        "GetForegroundWindow",
        "complete",
        input::get_foreground_window
    ),
    window!(
        "SetForegroundWindow",
        "raise and focus",
        input::set_foreground_window
    ),
    window!(
        "AllowSetForegroundWindow",
        "always allowed",
        input::allow_set_foreground_window
    ),
    window!(
        "LockSetForegroundWindow",
        "no-op",
        input::lock_set_foreground_window
    ),
    window!(
        "SetCapture",
        "mouse messages to the capturing window",
        input::set_capture
    ),
    window!(
        "ReleaseCapture",
        "complete",
        input::release_capture
    ),
    window!(
        "GetCapture",
        "complete",
        input::get_capture
    ),
    window!(
        "TrackMouseEvent",
        "WM_MOUSELEAVE",
        input::track_mouse_event
    ),
    window!(
        "GetCursorPos",
        "X pointer",
        input::get_cursor_pos
    ),
    window!(
        "GetPhysicalCursorPos",
        "X pointer",
        input::get_physical_cursor_pos
    ),
    window!(
        "SetCursorPos",
        "X warp",
        input::set_cursor_pos
    ),
    window!(
        "ClipCursor",
        "confines to the program window containing the rectangle",
        input::clip_cursor
    ),
    window!(
        "GetClipCursor",
        "complete",
        input::get_clip_cursor
    ),
    window!(
        "LoadCursorW",
        "system cursors as X font cursors; program cursors show the arrow",
        input::load_cursor_w
    ),
    window!(
        "LoadCursorA",
        "as W",
        input::load_cursor_a
    ),
    window!(
        "SetCursor",
        "all program windows; NULL hides",
        input::set_cursor_w
    ),
    window!(
        "GetCursor",
        "complete",
        input::get_cursor
    ),
    window!(
        "ShowCursor",
        "display counter",
        input::show_cursor
    ),
    window!(
        "GetCursorInfo",
        "complete",
        input::get_cursor_info
    ),
    window!(
        "DestroyCursor",
        "complete",
        input::destroy_cursor
    ),
    window!(
        "RegisterRawInputDevices",
        "mouse and keyboard",
        input::register_raw_input_devices
    ),
    window!(
        "GetRegisteredRawInputDevices",
        "complete",
        input::get_registered_raw_input_devices
    ),
    window!(
        "GetRawInputData",
        "RAWINPUT for WM_INPUT",
        input::get_raw_input_data
    ),
    window!(
        "GetRawInputBuffer",
        "unread RAWINPUTs of the thread",
        input::get_raw_input_buffer
    ),
    window!(
        "GetRawInputDeviceList",
        "one mouse, one keyboard",
        input::get_raw_input_device_list
    ),
    window!(
        "GetRawInputDeviceInfoW",
        "name, info, no preparsed data",
        input::get_raw_input_device_info_w
    ),
    window!(
        "GetRawInputDeviceInfoA",
        "name, info, no preparsed data",
        input::get_raw_input_device_info_a
    ),
    window!(
        "DefRawInputProc",
        "complete",
        input::define_raw_input_proc
    ),
    window!(
        "GetDC",
        "token DC (no GDI drawing)",
        misc::get_dc
    ),
    window!(
        "GetDCEx",
        "token DC",
        misc::get_dc_ex
    ),
    window!(
        "GetWindowDC",
        "token DC",
        misc::get_window_dc
    ),
    window!(
        "ReleaseDC",
        "complete",
        misc::release_dc
    ),
    window!(
        "WindowFromDC",
        "complete",
        misc::window_from_dc
    ),
    window!(
        "OpenClipboard",
        "within this process",
        misc::open_clipboard
    ),
    window!(
        "CloseClipboard",
        "within this process",
        misc::close_clipboard
    ),
    window!(
        "EmptyClipboard",
        "within this process",
        misc::empty_clipboard
    ),
    window!(
        "SetClipboardData",
        "within this process",
        misc::set_clipboard_data
    ),
    window!(
        "GetClipboardData",
        "within this process",
        misc::get_clipboard_data
    ),
    window!(
        "IsClipboardFormatAvailable",
        "within this process",
        misc::is_clipboard_format_available
    ),
    window!(
        "RegisterClipboardFormatW",
        "complete",
        misc::register_clipboard_format_w
    ),
    window!(
        "MessageBoxW",
        "text to stderr; default button",
        misc::message_box_w
    ),
    window!(
        "MessageBoxA",
        "text to stderr; default button",
        misc::message_box_a
    ),
    window!(
        "DialogBoxParamW",
        "dialog resources not shown: -1",
        misc::dialog_box_param_w
    ),
    window!(
        "EndDialog",
        "complete",
        misc::end_dialog
    ),
    window!(
        "SetDlgItemTextW",
        "no dialogs",
        misc::set_dlg_item_text
    ),
    window!(
        "SetDlgItemTextA",
        "no dialogs",
        misc::set_dlg_item_text
    ),
    window!(
        "SendDlgItemMessageW",
        "no dialogs",
        misc::send_dlg_item_message_w
    ),
    window!(
        "GetDpiForWindow",
        "96",
        misc::get_dpi_for_window
    ),
    window!(
        "GetDpiForSystem",
        "96",
        misc::get_dpi_for_system
    ),
    window!(
        "SetProcessDPIAware",
        "physical pixels",
        misc::set_process_dpi_aware
    ),
    window!(
        "SetProcessDpiAwarenessContext",
        "physical pixels",
        misc::set_process_dpi_awareness_context
    ),
    window!(
        "SetThreadDpiAwarenessContext",
        "physical pixels",
        misc::set_thread_dpi_awareness_context
    ),
    window!(
        "GetSystemMetricsForDpi",
        "as GetSystemMetrics",
        misc::get_system_metrics_for_dpi
    ),
    window!(
        "EnableNonClientDpiScaling",
        "no-op",
        misc::enable_non_client_dpi_scaling
    ),
    window!(
        "EnableMouseInPointer",
        "classic mouse messages only",
        misc::enable_mouse_in_pointer
    ),
    window!(
        "IsMouseInPointerEnabled",
        "0",
        misc::is_mouse_in_pointer_enabled
    ),
    window!(
        "GetPointerInfo",
        "no pointer messages",
        misc::get_pointer_info
    ),
    window!(
        "RegisterTouchWindow",
        "no touch input",
        misc::register_touch_window
    ),
    window!(
        "UnregisterTouchWindow",
        "no touch input",
        misc::unregister_touch_window
    ),
    window!(
        "GetThreadDesktop",
        "the default desktop",
        misc::get_thread_desktop
    ),
    window!(
        "GetUserObjectInformationA",
        "flags and name",
        misc::get_user_object_information_a
    ),
    window!(
        "GetUserObjectInformationW",
        "flags and name",
        misc::get_user_object_information_w
    ),
    window!(
        "WaitForInputIdle",
        "0",
        misc::wait_for_input_idle
    ),
    window!(
        "DisableProcessWindowsGhosting",
        "no-op",
        misc::disable_process_windows_ghosting
    ),
    window!(
        "RegisterDeviceNotificationW",
        "accepted; no device events",
        misc::register_device_notification_w
    ),
    window!(
        "RegisterDeviceNotificationA",
        "accepted; no device events",
        misc::register_device_notification_w
    ),
    window!(
        "UnregisterDeviceNotification",
        "complete",
        misc::unregister_device_notification
    ),
    window!(
        "LoadIconW",
        "handles never drawn",
        misc::load_icon
    ),
    window!(
        "LoadIconA",
        "handles never drawn",
        misc::load_icon
    ),
    window!(
        "LoadImageW",
        "system cursors; other images are tokens",
        misc::load_image_w
    ),
    window!(
        "DestroyIcon",
        "complete",
        misc::destroy_icon
    ),
    window!(
        "CreateIconIndirect",
        "token handle",
        misc::create_icon_indirect
    ),
    window!(
        "CopyRect",
        "complete",
        misc::copy_rect
    ),
    window!(
        "SetRect",
        "complete",
        misc::set_rect
    ),
    window!(
        "SetRectEmpty",
        "complete",
        misc::set_rect_empty
    ),
    window!(
        "OffsetRect",
        "complete",
        misc::offset_rect
    ),
    window!(
        "InflateRect",
        "complete",
        misc::inflate_rect
    ),
    window!(
        "PtInRect",
        "complete",
        misc::pt_in_rect
    ),
    window!(
        "IsRectEmpty",
        "complete",
        misc::is_rect_empty
    ),
    window!(
        "IntersectRect",
        "complete",
        misc::intersect_rect
    ),
    window!(
        "UnionRect",
        "complete",
        misc::union_rect
    ),
    window!(
        "EqualRect",
        "complete",
        misc::equal_rect
    ),
    window!(
        "WindowFromPoint",
        "program windows",
        misc::window_from_point
    ),
    window!(
        "GetDisplayConfigBufferSizes",
        "ERROR_NOT_SUPPORTED",
        misc::get_display_config_buffer_sizes
    ),
    window!(
        "QueryDisplayConfig",
        "ERROR_NOT_SUPPORTED",
        misc::query_display_config
    ),
    window!(
        "DisplayConfigGetDeviceInfo",
        "ERROR_NOT_SUPPORTED",
        misc::display_config_get_device_info
    ),
    export!(
        "gdi32.dll",
        "GetDeviceCaps",
        "window",
        "partial",
        "screen size, 32 bpp, 96 DPI, 60 Hz",
        misc::get_device_caps
    ),
    export!(
        "imm32.dll",
        "ImmGetContext",
        "window",
        "partial",
        "no input method",
        misc::imm_get_context
    ),
    export!(
        "imm32.dll",
        "ImmReleaseContext",
        "window",
        "partial",
        "no input method",
        misc::imm_release_context
    ),
    export!(
        "imm32.dll",
        "ImmAssociateContext",
        "window",
        "partial",
        "no input method",
        misc::imm_associate_context
    ),
    export!(
        "imm32.dll",
        "ImmAssociateContextEx",
        "window",
        "partial",
        "no input method",
        misc::imm_associate_context_ex
    ),
    export!(
        "imm32.dll",
        "ImmGetCompositionStringW",
        "window",
        "partial",
        "empty composition",
        misc::imm_get_composition_string_w
    ),
    export!(
        "imm32.dll",
        "ImmSetCompositionStringW",
        "window",
        "partial",
        "no input method",
        misc::imm_set_composition_string_w
    ),
    export!(
        "imm32.dll",
        "ImmNotifyIME",
        "window",
        "partial",
        "no input method",
        misc::imm_notify_ime
    ),
    export!(
        "imm32.dll",
        "ImmGetConversionStatus",
        "window",
        "partial",
        "no input method",
        misc::imm_get_conversion_status
    ),
    export!(
        "imm32.dll",
        "ImmSetConversionStatus",
        "window",
        "partial",
        "no input method",
        misc::imm_set_conversion_status
    ),
    export!(
        "imm32.dll",
        "ImmSetOpenStatus",
        "window",
        "partial",
        "no input method",
        misc::imm_set_open_status
    ),
    export!(
        "imm32.dll",
        "ImmGetOpenStatus",
        "window",
        "partial",
        "no input method",
        misc::imm_get_open_status
    ),
    export!(
        "imm32.dll",
        "ImmSetCandidateWindow",
        "window",
        "partial",
        "no input method",
        misc::imm_set_candidate_window
    ),
    export!(
        "imm32.dll",
        "ImmSetCompositionWindow",
        "window",
        "partial",
        "no input method",
        misc::imm_set_composition_window
    ),
    export!(
        "dwmapi.dll",
        "DwmGetWindowAttribute",
        "window",
        "partial",
        "extended frame bounds, cloaked",
        misc::dwm_get_window_attribute
    ),
    export!(
        "dwmapi.dll",
        "DwmSetWindowAttribute",
        "window",
        "partial",
        "accepted",
        misc::dwm_set_window_attribute
    ),
    export!(
        "dwmapi.dll",
        "DwmIsCompositionEnabled",
        "window",
        "partial",
        "enabled",
        misc::dwm_is_composition_enabled
    ),
    export!(
        "dwmapi.dll",
        "DwmFlush",
        "window",
        "partial",
        "no-op",
        misc::dwm_flush
    ),
    export!(
        "shcore.dll",
        "SetProcessDpiAwareness",
        "window",
        "partial",
        "physical pixels",
        misc::set_process_dpi_awareness
    ),
    export!(
        "shcore.dll",
        "GetDpiForMonitor",
        "window",
        "partial",
        "96",
        misc::get_dpi_for_monitor
    ),
    export!(
        "shcore.dll",
        "GetProcessDpiAwareness",
        "window",
        "partial",
        "per-monitor aware",
        misc::get_process_dpi_awareness
    ),
}
