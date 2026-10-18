# Unreal Engine import ledger

## Baseline

- **Game:** Palworld (Unreal Engine 5), Steam AppID 1623730,
  `Pal/Binaries/Win64/Palworld-Win64-Shipping.exe` (161802312 bytes). `Palworld.exe`
  is a launcher that starts the shipping executable as a child process (unsupported),
  so the shipping executable is run directly with `-dx11` (DXVK translates D3D11 only;
  the default is D3D12).
- **Launch:** `./dist/peekgame <Palworld>/Pal/Binaries/Win64/Palworld-Win64-Shipping.exe --run -dx11`
  Log: `logs/Palworld/pal.log`.
- **Where it stops:** `int3` at `+0x9fda0ec` inside the `.bind` section (the entry
  point): SteamStub DRM's launch check. **Stopped by policy**: DRM is never bypassed,
  patched or answered (same as re3.exe).

## MSVCP140 / VCRUNTIME140 (Microsoft's, local only)

- **Trigger:** 210 imports of MSVCP140.dll (C++ standard library), which games expect
  installed system-wide; PeekGame's own VCRUNTIME140 lacks __CxxFrameHandler4, RTTI and
  type_info helpers that UE5 needs.
- **Implementation:** `tools/extract_msvc_redist.py [--with-vcruntime]` carves the CABs
  out of Steam's local `VC_redist.x64.exe` and extracts the x64 `msvcp140*.dll` (and with
  the flag `vcruntime140*.dll`) into `windows-dlls/` (gitignored; checksums in
  `MSVC-REDIST-PROVENANCE.json`); `tools/stage_dist.py` copies them beside the runner.
  With `--with-vcruntime` Microsoft's VCRUNTIME replaces ours for every program that
  imports it.
- **UCRT additions for them** (`src/api/crt/cxx_support.rs`): _lock_locales/_unlock_locales
  (recursive), ___lc_locale_name_func, ___lc_collate_cp_func, __pctype_func (C ctype
  table), _wsetlocale (C only), isupper/islower, __strncnt, wcsnlen, _wcsdup,
  strtod/strtof/wcstod/wcstof, atol (32-bit), _set_new_handler/_callnewh,
  _seh_filter_dll, __stdio_common_vsprintf_s, _dclass/_fdclass/_dsign/_fdsign, copysign,
  frexp, lgamma/lgammaf/lgammal, _wremove/_wrename/_wrmdir, _fsopen,
  _get_stream_buffer_pointers (pointers into the FILE layout; host-buffered streams
  report an empty buffer).
- **Still unresolved in msvcp140 (stop if called):** time-name tables (_Getdays,
  _Getmonths, _Gettnames, _Strftime, _Wcsftime and W variants), threadpool work/wait/timer,
  CreateFile2, CopyFile2, CreateSymbolicLinkW, CreateHardLinkW.
- **Verification:** Palworld: vcruntime140, vcruntime140_1 and msvcp140 complete
  DLL_PROCESS_ATTACH; the program's TLS callbacks run; then SteamStub. MarioVsLuigi and
  DOOM (which do not load vcruntime140) unchanged; crt_core, crt_port,
  msvc_runtime_basics, seh, `cargo test` pass.

## USER32 GetQueueStatus (CEF in Palworld)

- **Trigger:** Palworld (UE5) now gets past SteamStub. The stub runs unmodified against the real Steam client, now that the command line names the exe by its Windows path. It loads Wwise and CEF, then stops at `STOP: missing Windows API USER32.dll!GetQueueStatus`, called from libcef.dll's message pump.
- **Contract:** the high word holds the QS_* kinds currently queued for the calling thread; the low word holds the kinds added since the last GetQueueStatus/GetMessage/PeekMessage. Both are limited to the flags, and the reported "added" bits are cleared. Unknown flags give ERROR_INVALID_FLAGS.
- **Implementation:** `src/api/window/queue.rs`. Posted messages record their kind (plus QS_ALLPOSTMESSAGE); sent messages record QS_SENDMESSAGE; due timers give QS_TIMER; windows needing paint give QS_PAINT. Retrieval clears the "added" bits.
- **Verification:** `tests/test_queue_status.py`.
- **Note:** libcef.dll has 398 unresolved static imports (GDI, menus, DDE, clipboard, ...). They are handled when a STOP shows they are reached, not ahead of time.

## SteamUser021 auth ticket forwarding (Palworld online login)

- **Trigger:** Palworld reports `ConnectLoginNoEAS(0) failed due to the platform OSS giving an empty auth token`.
- **Contract:** `ISteamUser021::GetAuthSessionTicket` passes the remote `SteamNetworkingIdentity` as its fourth argument to the real Steam client. Earlier bridged SteamUser versions retain their three-argument form.
- **Implementation:** `native/steam_bridge/steam_bridge.c` forwards the fourth argument for SteamUser021 and logs only ticket handle and length when `STEAM_BRIDGE_TRACE=1`.
- **Verification:** bridge ABI test checks that the fourth pointer reaches the Linux SteamUser021 object and that ticket handle, length, and bytes return through the Windows ABI. A live Palworld trace in `/tmp/goagal_in_actiune.txt` shows the SteamStub ownership ticket (196 bytes), but no `GetAuthSessionTicket` call; online login remains unresolved. It repeatedly requests `SteamNetworking006` and `SteamMatchMakingServers002` during `SteamAPI_Init`; both formerly returned NULL. The bridge now forwards those interface requests to Linux Steam and wraps returned objects so initialization can proceed. Their method slots still trap if called and need implementation from an observed call. The fake-client test verifies both wrappers; live validation is pending because the Steam client was not running during this change.
- **Other engines:** older SteamUser interface versions continue using their existing signature.

## Palworld networking interface requests before CrashReportClient

- **Observation:** the main process requested `SteamNetworkingSockets012` and `SteamNetworkingUtils004`, received NULL, then raised `EXCEPTION_ACCESS_VIOLATION` reading address zero at `Palworld-Win64-Shipping.exe+0x15feedc`. It launched CrashReportClient, where `dbgeng.dll!DebugCreate` was the later STOP. The exact null object's identity is not yet proven.
- **Implementation:** `native/steam_bridge/steam_bridge.c` now forwards both interface requests to Linux Steam and returns Windows ABI wrappers. Method calls still stop with a slot diagnostic until their signatures are implemented.
- **Verification:** fake Steam client checks both wrappers. The real Steam client was not running for an isolated host probe; live confirmation is pending.
- **Next live trace:** the real Linux Steam client returned a non-NULL `SteamNetworkingSockets012` object. Palworld called slot 22, which is `InitAuthentication` in Valve's version 012 header. The bridge now forwards `InitAuthentication` and adjacent `GetAuthenticationStatus` to the Linux object, logging their result codes with `STEAM_BRIDGE_TRACE=1`. The fake-client ABI test checks both calls. Live result is pending.
- **Following trace:** `InitAuthentication` returned 3, then Palworld called `SteamNetworkingUtils004` slot 13. Valve's version 004 header identifies it as `SetDebugOutputFunction`; the bridge now adapts its callback between SysV and Windows x64 calling conventions. The fake-client test verifies callback level and message. `openvr_api.dll` GetModuleHandleEx probes returned false later in the trace, but were not the first STOP.
