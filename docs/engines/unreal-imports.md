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
