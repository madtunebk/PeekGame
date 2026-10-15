# TODO — what remains for MarioVsLuigi (and Unity games generally)

In rough order of what the game hits next.

1. **IOCP / Discord follow-up**: queue APIs, overlapped file completions and
   Unix-socket byte-pipe clients are implemented; the real Discord SDK passes
   creation/callbacks/destruction against a controlled peer and the running Linux
   Discord client. Rich presence/invitations/overlay still need game-level checks.
   Socket overlapped WSARecv/WSASend,
   AcceptEx/ConnectEx and AFD polling are still missing; unsupported calls fail.
   See `docs/discord-iocp.md`. Unity's next observed stop is `CertOpenStore`.
2. **Audio**: WASAPI (MMDevAPI `IMMDeviceEnumerator`, `IAudioClient`,
   `IAudioRenderClient`) on a host backend (PulseAudio/PipeWire). peek has
   `wasapi.inc`. Without it the game runs silent or fails audio init.
3. **Online play**: HTTPS — crypt32 certificate chain verification (peek
   `crypt32.inc`, `cert_chain.inc`), WinHTTP requests if used, async sockets
   (item 1).
4. **Check rendering on real hardware** (user's GPU instead of lavapipe):
   swapchain resize, fullscreen toggling (`SetWindowPos` on a caption-less
   window), vsync/frame latency.
5. **Input**: gamepads (XInput1_4/XInput9_1_0 or HID via SetupDi — now report
   no devices), IME/text input beyond WM_CHAR, cursor confinement in fullscreen.
6. **Mono details**: `FormatMessage(127/50)` noise hints at a few missing
   lookups; watch `Player.log` for `EntryPointNotFoundException` (P/Invoke
   into kernel32/user32 functions not implemented yet).
7. **Smaller gaps** seen in the import lists: DirectWrite (if the UI uses it),
   `GetSystemPowerStatus`, console APIs (`WriteConsoleW`…), process creation
   (`CreateProcessW`, for the crash handler / browser links), `ShellExecuteW`,
   clipboard via the X selection, dialog resources.
8. **Tests to add**: Vulkan bridge (instance + device on lavapipe), USER32
   input/raw input, UCRT printf formatting, waitable timers, SuspendThread.
9. **`--relink`** of a Unity game end to end (the relinked runtime shares all
   of the above; not tried with DXVK yet).
