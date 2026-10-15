# PeekGame — notes (state at the end of this session)

PeekGame runs unmodified Windows x64 games on Linux: the EXE and the DLLs a game
ships are mapped as PE images, their imports are bound to Rust implementations of
the Windows API (`src/api`, listed in `src/registry`), and `--relink` turns an EXE
into a native ELF.

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
IOCP and `FreeLibraryAndExitThread` work, including the real Discord SDK. CRYPT32
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
