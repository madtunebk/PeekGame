# Godot import ledger

## CRT thread termination: msvcrt.dll!_endthreadex

- **Trigger:** Gundam Survivors hung on quit with audio and its window still alive.
  GDB plus thread-handle tracing proved the main thread waited indefinitely on a
  Windows handle whose Linux thread had already exited.
- **Contract:** terminate the calling CRT thread with its exit code; its thread
  handle becomes signaled after cleanup.
- **Implementation:** `src/api/crt/ucrt.rs::end_thread` uses the existing
  `host/thread_entry.rs` return boundary, like `ExitThread`. The owner in
  `api/threading/threads.rs` runs FLS/TLS/DLL detach and signals completion.
  Calling `pthread_exit` directly skipped that owner entirely.
- **Scope/errors:** threads created through the runtime's thread entry boundary;
  a thread without such a boundary stops explicitly. No forced unwinding through
  Windows frames. Applies to other engines using this CRT API too.
- **Verification:** `python3 tests/test_endthread.py` checks repeated CRT thread
  creation, finite joins, exit code 73 and FLS callbacks before join returns.
  **PASS.** A private Xvfb/llvmpipe Godot run using real PulseAudio playback to an
  isolated sink and `--quit-after 120` now returns **0**; before the fix it hung.
  Logs: `logs/gundam-quit-fixed.log` and `logs/gundam-quit-elf.log`; PE and
  relinked ELF both pass (ELF test uses a cache-only symlink to the game PCK).
  Hardware rendering and in-game Quit still
  require a user run; no forced process kill is used as the fix.

## Outstanding static imports

The user's latest Godot relink report lists 111 unresolved imports. This is an
inventory, not proof all paths execute or cause shutdown. API-set routing must be
checked before implementing duplicate registry/security exports. Native menus,
GDI bitmap operations, accessibility, touch, process creation, MIDI and other
features require individual contract tests; success stubs are not acceptable.

## API-set aliases to existing ADVAPI32 implementations

- **Exports:** `api-ms-win-core-registry-l1-1-0.dll!RegCloseKey`,
  `RegEnumKeyExW`, `RegEnumValueW`, `RegGetValueW`, `RegOpenKeyExW`,
  `RegQueryInfoKeyW`, `RegQueryValueExW`, `RegSetValueExW`;
  `api-ms-win-core-processthreads-l1-1-0.dll!OpenProcessToken`;
  `api-ms-win-security-base-l1-1-0.dll!GetTokenInformation`,
  `GetSidSubAuthority`, `GetSidSubAuthorityCount`.
- **Trigger:** present in the user's 111-unresolved Godot static report despite
  existing ADVAPI32 implementations. The previous project resolves core/security
  API sets against kernel32 then advapi32 (`src/peek.c::resolve`, reference only).
- **Implementation:** `src/registry/mod.rs::find` retains direct resolution first,
  then checks existing ADVAPI32 exports only for core/security API contracts that
  otherwise route to kernel32. Both runtime and relink use this registry.
- **Contract/scope:** same pointer, Win64 ABI, ownership, error behavior and partial
  feature coverage as the existing export. No new token privileges or registry
  operations are simulated. Unknown names still fail; actual kernel32 and unrelated
  DLLs do not gain ADVAPI32 exports. COM/WinRT and CRT routing stays separate.
- **Verification:** `python3 tests/test_imports_abi.py` checks all twelve aliases,
  pointer equality and negative lookups. Full registry/security operation coverage
  belongs to their existing subsystem fixtures. Shared across engines.

## Godot WASAPI RATEADJUST (COM methods)

- **Methods:** `IAudioClient::Initialize`, `IAudioClient::GetService`,
  `IAudioClockAdjustment::SetSampleRate`. Not named DLL exports.
- **Trigger:** Initialize rejected Godot's `AUDCLNT_STREAMFLAGS_RATEADJUST`
  (0x100000) with 0x88890021, AUDCLNT_E_INVALID_STREAM_FLAG.
- **Implementation:** `api/audio/{client,adjustment}.rs` and `host/pulse.rs`;
  a real PA_STREAM_VARIABLE_RATE stream and asynchronous PulseAudio sample-rate
  operation. No SDL or graphics-backend coupling. COM pointers use Win64 vtables
  and owned reference counts; host pointers remain private.
- **Scope:** shared render streams; finite integral 8000–192000 Hz adjustment.
  Invalid rates return E_INVALIDARG; GetService without RATEADJUST rejects the
  adjustment interface. Unsupported flags still fail. No capture/exclusive mode.
- **Verification:** `python3 tests/test_wasapi.py`: float32, PCM16 and RATEADJUST
  output captured from an isolated sink passed. Godot PE/ELF quit probes initialized
  44100 Hz playback. User confirmed audible game sound. Shared with Unity.

## OLE file drops on native X11

- **Exports:** `ole32.dll!OleInitialize`, `OleUninitialize`, `RegisterDragDrop`,
  `RevokeDragDrop`, `ReleaseStgMedium`; `shell32.dll!DragQueryFileW`, `DragFinish`.
  `shell32.dll!DragAcceptFiles` continues to report legacy delivery unsupported.
- **Contract:** registration owns a COM reference; callbacks use Win64 and run
  on the window owner's thread. Real IDataObject exposes CF_HDROP in owned HGLOBAL
  storage, with UTF-16 DROPFILES content and explicit release. DragOver coordinates
  are screen coordinates. Registration requires OLE initialization and a valid
  owner-thread window; duplicates fail. Unsupported formats/interfaces fail.
- **Implementation:** `api/window/{ole_drop,drop_data}/`, `host/x11/xdnd.rs`,
  window pump/queue and `api/com/apartment.rs`; registry entries in com/shell.
  Xdnd selection retrieval completes before DragEnter exposes actual file data.
  No Windows callback runs under the pump lock; revoke/destroy during callbacks
  retains enough ownership for safe return.
- **Scope:** incoming copy-only local file URIs, Unicode/multiple files, bounded
  4 MiB selections/4096 files and timeout. No INCR, virtual files, outgoing drops,
  text/image transfer or legacy WM_DROPFILES.
- **Verification:** `python3 tests/test_ole_drop.py` passed real selection transfer,
  enter/over/leave/drop, rejection, retained data and reentrant revoke/destroy on
  private Xvfb. `tests/test_app_identity.py` also passed after registration became
  real. Full desktop file-manager/Godot drop behavior remains user verification.
- **Reference:** earlier project's `src/modules/window/dragdrop.inc`, used only
  for behavior reference; no runtime/build dependency on that repository.

## Window geometry and display modes (verification scope)

- **Affected exports:** `user32.dll!CreateWindowExW`, `CreateWindowExA`,
  `SetWindowPos`, `SetWindowLongPtrW`, `GetClientRect`, `GetWindowRect`,
  `EnumDisplaySettingsW`, `EnumDisplaySettingsA`.
- **Trigger:** game window on the 1920x1080 monitor actually measured 2560x1442,
  had fixed min/max hints and lacked native EWMH fullscreen state.
- **Implementation:** native monitor selection, EWMH fullscreen properties and
  Xinerama fullscreen monitor indices, preserved WM position hints, ConfigureNotify
  updates to window origin/client size/surface size and WM_MOVE/WM_SIZE. XRandR
  output modes feed enumeration. Source: `api/window/{windows,pump,display}.rs`,
  `host/x11/{fullscreen,modes,mod}.rs`, `host/x11/modes.c`.
- **Scope:** GetClientRect reports actual geometry; never hardcodes 720p/1080p.
  Mode discovery does not implement physical exclusive display-mode switching.
  Tiled/multi-output displays fall back to current dimensions. libXinerama is a
  host dependency, including relinked ELF.
- **Verification:** private `tests/test_display_topology.py` and OpenGL resize
  fixture passed; desktop multi-monitor fullscreen behavior remains unverified.
  Shared USER32 implementation affects all engines; no engine-specific constants.

## Mouse/UI offset after native resize

- **Affected path:** X11 ConfigureNotify -> USER32 WM_WINDOWPOSCHANGED,
  GetClientRect/ClientToScreen, DefWindowProcW's WM_MOVE/WM_SIZE handling.
  No new export or engine-specific coordinate scaling.
- **Reproduction:** a 640x360 Godot canvas with a center button. After native
  resize to 800x600 or 960x540, Godot received untransformed client pixels
  (400,300)/(480,270) instead of canvas (320,180); only the initial-size click
  worked. The pointer and client rectangle themselves were correct.
- **Cause:** ConfigureNotify updated runtime dimensions but posted only WM_MOVE
  and WM_SIZE. Godot updates its window/render/input transform from
  WM_WINDOWPOSCHANGED, so its transform remained stale.
- **Fix:** queue an owned Windows-layout WINDOWPOS for the window's owner thread.
  Invoke its procedure after releasing the pump/queue locks; keep the payload
  valid across reentrant calls. DefWindowProc emits move/size notifications when
  the application forwards WINDOWPOSCHANGED, avoiding duplicate posted messages.
- **Verification:** tests/test_godot_input.py exercises a real Godot Windows
  engine and button at three moved/resized sizes on private Xvfb. Godot 4.7.1 and
  Gundam's 4.7.2 engine pass after the fix; before it only one of three clicks
  passed. tests/test_window_abi.py also covers reparented/moved frame geometry,
  client/screen coordinates, raw deltas and button events. Mouse hooks, OpenGL
  PE/ELF and OLE/Xdnd regression suites pass.
- **Limits:** desktop multi-monitor/fullscreen and captured mouse behavior still
  require verification. This fixes a reproduced stale-transform defect; it does
  not certify every reported input symptom. Existing relinked ELF programs must
  be regenerated to include the updated runtime.
