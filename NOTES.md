# PeekGame — notes (state at the end of this session)

PeekGame runs unmodified Windows x64 games on Linux: the EXE and the DLLs a game
ships are mapped as PE images, their imports are bound to Rust implementations of
the Windows API (`src/api`, listed in `src/registry`), and `--relink` turns an EXE
into a native ELF.

## Required import documentation

For the next engine and all new API work, document every added or changed import
in its engine ledger alongside the code change. Follow
[Import tracking](docs/import-tracking.md): contract, trigger, implementation,
limits, verification and impact on other engines.

## Running MarioVsLuigi (Unity 6000.3.10f1, Mono, Win64 build)

```sh
cargo build --release
python3 tools/stage_dist.py
G="$PWD/games/MarioVsLuigi-Win64-v2.2.0.1/MarioVsLuigi-Win64"
./dist/peekgame "$G/NSMB-MarioVsLuigi.exe" --run
```

Needs a host Vulkan driver (`libvulkan.so.1` + ICD). In the session sandbox it
ran on Xvfb with Mesa lavapipe.

Where it gets now: UnityPlayer and Mono start, the engine initializes, DXVK
creates the D3D11 device and swapchain through the `vulkan-1.dll` bridge, frames
are presented, the game's C# code runs (main menu, translations), plugins load.
Shared WASAPI playback now reaches PulseAudio/PipeWire; Unity menu samples were
captured from an isolated sink. See `docs/audio.md`. IOCP and
`FreeLibraryAndExitThread` work, including the real Discord SDK. CRYPT32
now provides ROOT/memory stores and OpenSSL chain/SSL policy validation. The latest
private-Xvfb check stayed running for 25 seconds without a STOP or Curl error 35.

Logs: Unity's `Player.log` is under `~/.local/share/peekgame/LocalLow/<company>/<game>/`.
PeekGame prints `STOP: missing Windows API DLL!Name` for the next unimplemented
call; unhandled exceptions are printed before the program's crash handler runs
(delay-load failures name the DLL).

## Architecture in brief

- `src/pe`, `src/images`: PE parsing, mapping, relocation, imports (by name,
  ordinal, forwarders, API sets), TLS, DllMain order, LoadLibrary search.
  `PEEKGAME_WINDOWS_DLLS` (default `windows-dlls/` beside the binary) holds
  Windows DLLs that replace PeekGame's own (DXVK).
- `src/seh`: Windows x64 SEH — RaiseException, unwinding (.pdata and run-time
  function tables), vectored handlers, hardware faults from signals.
- `src/host/teb.rs`: TEB/PEB in gs (TLS slots, ProcessParameters).
- `src/api/window`: USER32 on X11 — process-wide windows, per-thread queues,
  input (keyboard by scan code/layout, mouse, raw input), cursors, focus.
- `src/api/graphics/vulkan.rs`: `vulkan-1.dll` — one generic Windows x64 →
  System V thunk for Vulkan commands, Win32 surfaces as Xlib surfaces.
- `src/api/crt`: msvcrt + UCRT (printf on the Windows va_list, onexit, …).
- `src/api/threading`: events, semaphores, mutexes, APCs, waitable timers,
  SuspendThread/GetThreadContext (signal-based, for Mono's GC), SRW, FLS, TLS.
- `src/api/network`: Winsock on Linux sockets, SSPI table without packages,
  WinHTTP proxy queries.
- Registry: HKCU persisted to `APPDATA/registry/user.reg` (PlayerPrefs).

## Testing

`python3 tests/test_<name>.py` (window_abi, crt_core, files_sync, seh,
tls_threads, pe_dlls, imports_abi, relink, …). They build small mingw programs
and run them through PeekGame (window tests on a private Xvfb).

## Local dependency ownership

DXVK/glslang snapshots are in `sources/`; the compiler and retained Windows DLLs
are in `deps/`. They remain local, with upstream revisions/checksums/patches
tracked in `deps/`. `tools/fetch_sources.py` restores pinned source inputs for a
fresh checkout. The Unity game copy is under `games/`. No build/runtime input
comes from the earlier project. Use `dist/peekgame` for runs; `target/` is build
output. See `docs/discord-iocp.md` for queue/pipe support and Discord probe limits.

## CRT export parity

The former msvcrt.inc subset's 183 exports now resolve; 87 missing exports were
ported into separate Rust modules. See `docs/crt-port.md` for scope and tests.
OpenMutexW/independent mutex lifetime and process priority classes are available.
AVRT registration fails explicitly with ERROR_NOT_SUPPORTED (no MMCSS backend).

## Native OpenGL update

Native X11/GLX WGL and typed GL adapters are implemented. Gundam renders its main
menu on private llvmpipe; PE and ELF GLSL/readback/context tests pass. See
`docs/opengl.md`. Remaining: desktop window sizing confirmation, GL debug callback
adapters, broader WGL formats, and desktop fullscreen verification. Godot RATEADJUST playback now works through
PulseAudio; the user confirmed sound. CRT `_endthreadex` now signals thread handles
after cleanup; real Godot PE/ELF `--quit-after 120` probes exit with code 0.
See `docs/engines/godot-imports.md` and the 99-entry static backlog.
MoveFileW now supports atomic same-filesystem moves without replacement.

## Unity IL2CPP locale bring-up

TouhouFumoRacing reaches GameAssembly/baselib initialization with real thread
accounting. Its exercised GetLocaleInfo startup queries now use an ICU-backed
NLS adapter for currency, grouping, signs, patterns and calendar names/rules.
`tests/test_locale_info.py` passes PE/ELF and en-US/ro-RO/ja-JP checks.
New host dependency: ICU (`libicu-dev` to build), also linked by `--relink`.
The current private Touhou probe next stops at ole32!CoGetContextToken.
See `docs/engines/unity-imports.md`; this is not full game startup certification.

## Modularization stage 1

System export declarations now live in `src/registry/system/` behind a small index;
input is split into keyboard/focus/cursor/raw modules behind its existing facade.
All 347 system declarations and 1,583 link-time API symbols are preserved.
Private input/GL/drop, registry ABI and locale/thread regressions pass.
See `docs/modularization.md` for ownership, evidence and the per-game runtime build
plan. `game_imports.so` is still global; this split does not claim code pruning.

## Godot input resize correction

Native X11 ConfigureNotify now sends owned WM_WINDOWPOSCHANGED data on the
window owner thread. Godot's canvas/input transform previously stayed stale after
native resizing, even though GetClientRect was correct. A real Godot button
regression now passes at three moved/resized sizes (4.7.1 and Gundam 4.7.2).
See docs/engines/godot-imports.md. dist/ is staged; old relinked ELFs need rebuilding.

## Steam bridge integration

Native bridge now linked into PeekGame; Windows LoadLibrary/GetProcAddress and PE import binding use it. See docs/steam-bridge.md for verification and remaining discovery limitations.

## Bink private heaps
HeapCreate/HeapDestroy now support separate growable libc-backed heaps. Fixed-size/executable/exception-mode creation remains unsupported (error 50). HeapSetInformation explicitly returns unsupported; HeapQueryInformation reports standard mode, not LFH. Initial storage is reserved separately; allocations are not a contiguous Windows arena. Test: python3 tests/test_private_heap.py.

## Bink SetHandleCount

SetHandleCount is a Windows NT no-op and returns the requested count. It does not impose a file-handle cap. Verified with a Bink loader probe after staging.

GetWindowsDirectoryA now matches the existing W path and Win32 buffer-size contract. Bink advanced to this call after SetHandleCount.

## DOOM EXE TLS ordering

DOOM passed Bink/steam_api64/oo2core attach and faulted reading EXE TLS at +0x320. Imported DLLs had reserved TLS slots first; set_exe_template only populated slot 0 when the template list was empty. It now fills the reserved EXE slot. Regression: cargo test imported_dll_before_exe_keeps_exe_tls_template.

## DOOM BCrypt RNG

The call at DOOMEternalx64vk.exe+0x3e2fcd opens algorithm `RNG` with default provider, then calls BCryptGenRandom with that handle. BCryptOpenAlgorithmProvider, CloseAlgorithmProvider and GenRandom now validate provider handles and route bytes through Linux getrandom. Other algorithms return STATUS_NOT_FOUND, unsupported flags return a failure status.
