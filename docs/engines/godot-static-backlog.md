# Godot static import backlog

Gundam Survivors / Godot 4.7.2. Latest relink: 591 import slots bound at link time,
3 program-DLL imports bound at startup, **99 unresolved** (previously 111).
Twelve existing ADVAPI32 functions now resolve through core/security API sets.

This list covers static imports only. It does not prove execution or include
all dynamic exports/COM methods. Implement each contract with tests and a ledger
entry; do not replace these entries with unconditional success. The quit hang
was independently fixed in `_endthreadex`, an already-resolved CRT import.

## Remaining imports

- `oleaut32.dll!SafeArrayCreateVector`
- `oleaut32.dll!SafeArrayPutElement`
- `uiautomationcore.dll!UiaGetReservedMixedAttributeValue`
- `uiautomationcore.dll!UiaGetReservedNotSupportedValue`
- `user32.dll!CreateCaret`
- `user32.dll!CreatePopupMenu`
- `user32.dll!DestroyCaret`
- `user32.dll!DestroyMenu`
- `user32.dll!DialogBoxIndirectParamW`
- `user32.dll!DrawTextW`
- `user32.dll!FillRect`
- `user32.dll!GetDlgItem`
- `user32.dll!GetMenuItemCount`
- `user32.dll!GetMenuItemInfoW`
- `user32.dll!GetMenuItemRect`
- `user32.dll!GetUpdateRect`
- `user32.dll!InsertMenuItemW`
- `user32.dll!RegisterClassExA`
- `user32.dll!RemoveMenu`
- `user32.dll!SendInput`
- `user32.dll!SetCaretPos`
- `user32.dll!SetMenuInfo`
- `user32.dll!SetMenuItemInfoW`
- `user32.dll!SetWindowDisplayAffinity`
- `user32.dll!TrackPopupMenuEx`
- `user32.dll!UnregisterClassA`
- `ADVAPI32.dll!GetCurrentHwProfileA`
- `dwmapi.dll!DwmEnableBlurBehindWindow`
- `GDI32.dll!BitBlt`
- `GDI32.dll!CreateBitmap`
- `GDI32.dll!CreateCompatibleBitmap`
- `GDI32.dll!CreateDIBSection`
- `GDI32.dll!CreatePolygonRgn`
- `GDI32.dll!CreateSolidBrush`
- `GDI32.dll!GetDIBits`
- `GDI32.dll!GetPixel`
- `GDI32.dll!GetTextExtentPoint32W`
- `GDI32.dll!SelectObject`
- `GDI32.dll!SetDIBitsToDevice`
- `IMM32.dll!ImmSetCompositionFontA`
- `KERNEL32.dll!CompareStringOrdinal`
- `KERNEL32.dll!ConnectNamedPipe`
- `KERNEL32.dll!CreateNamedPipeW`
- `KERNEL32.dll!CreateProcessW`
- `KERNEL32.dll!CreateSymbolicLinkW`
- `KERNEL32.dll!DeleteProcThreadAttributeList`
- `KERNEL32.dll!GetConsoleProcessList`
- `KERNEL32.dll!GetConsoleWindow`
- `KERNEL32.dll!GetCurrentProcessorNumber`
- `KERNEL32.dll!GetExitCodeProcess`
- `KERNEL32.dll!GetFileTime`
- `KERNEL32.dll!GetFinalPathNameByHandleW`
- `KERNEL32.dll!GetGeoInfoW`
- `KERNEL32.dll!GetLargePageMinimum`
- `KERNEL32.dll!GetLogicalDrives`
- `KERNEL32.dll!GetProcessTimes`
- `KERNEL32.dll!GetShortPathNameW`
- `KERNEL32.dll!GetThreadTimes`
- `KERNEL32.dll!GetUserGeoID`
- `KERNEL32.dll!GetVolumeInformationW`
- `KERNEL32.dll!HeapCreate`
- `KERNEL32.dll!HeapDestroy`
- `KERNEL32.dll!InitializeProcThreadAttributeList`
- `KERNEL32.dll!IsDBCSLeadByteEx`
- `KERNEL32.dll!Module32FirstW`
- `KERNEL32.dll!Module32NextW`
- `KERNEL32.dll!MoveFileExA`
- `KERNEL32.dll!QueryFullProcessImageNameW`
- `KERNEL32.dll!SetConsoleCtrlHandler`
- `KERNEL32.dll!SetThreadIdealProcessor`
- `KERNEL32.dll!TerminateThread`
- `KERNEL32.dll!UpdateProcThreadAttribute`
- `api-ms-win-core-winrt-error-l1-1-1.dll!RoOriginateLanguageException`
- `api-ms-win-security-base-l1-1-0.dll!AdjustTokenPrivileges`
- `api-ms-win-security-lsalookup-l2-1-0.dll!LookupPrivilegeValueW`
- `ntdll.dll!NtQueryInformationFile`
- `ntdll.dll!NtWriteFile`
- `ole32.dll!CoCreateFreeThreadedMarshaler`
- `ole32.dll!GetErrorInfo`
- `ole32.dll!SetErrorInfo`
- `SHELL32.dll!SHCreateItemFromParsingName`
- `SHELL32.dll!SHFileOperationW`
- `SHELL32.dll!Shell_NotifyIconGetRect`
- `SHELL32.dll!Shell_NotifyIconW`
- `SHLWAPI.dll!QISearch`
- `SHLWAPI.dll!SHLoadIndirectString`
- `SHLWAPI.dll!SHStrDupW`
- `USER32.dll!AttachThreadInput`
- `USER32.dll!CloseTouchInputHandle`
- `USER32.dll!GetPointerPenInfo`
- `USER32.dll!GetPointerType`
- `USER32.dll!GetTouchInputInfo`
- `USER32.dll!MessageBeep`
- `WINMM.dll!midiInClose`
- `WINMM.dll!midiInGetDevCapsA`
- `WINMM.dll!midiInGetErrorTextA`
- `WINMM.dll!midiInOpen`
- `WINMM.dll!midiInStart`
- `WINMM.dll!midiInStop`
