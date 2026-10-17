# TODO — what remains for MarioVsLuigi (and Unity games generally)

In rough order of what the game hits next.

1. **IOCP / Discord follow-up**: queue APIs, overlapped file completions and
   Unix-socket byte-pipe clients are implemented; the real Discord SDK passes
   creation/callbacks/destruction against a controlled peer and the running Linux
   Discord client. Rich presence/invitations/overlay still need game-level checks.
   Socket overlapped WSARecv/WSASend,
   AcceptEx/ConnectEx and AFD polling are still missing; unsupported calls fail.
   See `docs/discord-iocp.md`. Certificate-store/chain stops have since been fixed.
2. **Audio follow-up**: shared WASAPI playback through PulseAudio/PipeWire is
   implemented and tested with captured float32/PCM16 samples and actual Unity menu
   music. Capture, exclusive mode, session/endpoint volume, surround and device
   notifications remain. See `docs/audio.md`.
3. **Online play**: CRYPT32 ROOT/memory stores and OpenSSL BASE/SSL validation
   are implemented (see `docs/certificates.md`). Online revocation/AIA retrieval,
   richer certificate policies, WinHTTP requests if used, and async sockets
   (item 1) remain. Verify the full online session on real hardware.
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

## Native OpenGL update

Native X11/GLX WGL and typed GL adapters are implemented. Gundam renders its main
menu on private llvmpipe; PE and ELF GLSL/readback/context tests pass. See
`docs/opengl.md`. Remaining: desktop window sizing confirmation, GL debug callback
adapters, broader WGL formats, and the separate Godot WASAPI flags issue.
MoveFileW now supports atomic same-filesystem moves without replacement.
