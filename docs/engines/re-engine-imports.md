# RE ENGINE import ledger

Capcom's RE ENGINE. First brought up with Resident Evil 3 (2020). See
`docs/import-tracking.md` for the per-entry template and the completion rule.

## Baseline

- **Game:** Resident Evil 3 (RE ENGINE), Steam AppID 952060, `re3.exe`
  sha256 `f7f707743243c058e5657f270b7f9f5d01174b0e2e5b679461c2392ae8130a30`,
  152,758,680 bytes, image base `0x140000000`, entry RVA `0x93b3310`.
- **Renderer:** the game's **Direct3D 12** renderer (`d3d12.dll`, `dxgi.dll`, with
  `d3d11.dll` also imported) backed by vkd3d-proton + DXVK from `windows-dlls/`
  over Vulkan; `amd_ags_x64.dll` (AMD GPU Services vendor extensions) is shipped by
  the game. Host: fill in GPU, driver and X server at trace time.
- **Shipped third-party DLLs** loaded as real PE modules (not PeekGame registry):
  `amd_ags_x64.dll`, `steam_api64.dll`, `iZotope.dll`. The Steam bridge forwards to
  the real `steamclient.so`; DRM is never answered on Steam's behalf.
- **Launch:** `./dist/peekgame ~/.steam/debian-installation/steamapps/common/RE3/re3.exe --run`
  with Steam running. (Record the exact log path once a run is traced.)
- **Where it reaches now:** the RE ENGINE boot splash renders (black screen, moon /
  hand logo). This is engine early-boot + first swapchain present only — **not** a
  claim of main menu, shader-cache build, gameplay, audio, save or shutdown support.
  No runtime import trace has been captured yet; the surface below is static
  analysis, which is evidence of what the game *may* call, not proof of execution.

## Static import surface

Static analysis: `./dist/peekgame --analyze re3.exe` →
`reports/re3/imports.{json,txt}`. Analyzer covered 3 PE modules (re3.exe +
`amd_ags_x64.dll` + `steam_api64.dll`), 630 import slots; `re3.exe` itself has 467
unique DLL/function pairs.

DLLs imported by `re3.exe` (counts), with how PeekGame resolves each:

| DLL | Imports | Resolution |
| --- | --- | --- |
| KERNEL32.dll | 279 | PeekGame registry |
| USER32.dll | 67 | PeekGame registry (X11) |
| api-ms-win-crt-* | ~112 | PeekGame CRT (UCRT) |
| WS2_32.dll | 25 | PeekGame Winsock |
| VCRUNTIME140 / MSVCP140 / CONCRT140 | 32 | real MS runtime DLLs in `windows-dlls/` |
| amd_ags_x64.dll | 15 | shipped PE DLL (loads real module) |
| WINHTTP.dll | 13 | PeekGame WinHTTP |
| steam_api64.dll | 12 | shipped PE DLL → Steam bridge |
| ole32.dll / OLEAUT32.dll / PROPSYS.dll | 13 | PeekGame COM + shell |
| ADVAPI32.dll | 7 | PeekGame registry/crypto |
| SETUPAPI.dll | 5 | PeekGame (device enumeration) |
| d3d12.dll / dxgi.dll / d3d11.dll | 4 | vkd3d-proton + DXVK in `windows-dlls/` |
| XINPUT1_3.dll / DINPUT8.dll | 5 | PeekGame input |
| dbghelp.dll | 3 | PeekGame |
| MFReadWrite.dll | 1 | **not implemented** (Media Foundation) |
| SHELL32 / WINMM / IMM32 / GDI32 / SETUPAPI | misc | PeekGame |

Note on the report's `backend: unknown` tag: for `re3.exe` imports it means there is
no PeekGame registry entry because the target is a **real PE DLL** loaded from
`windows-dlls/` or shipped beside the game (d3d12/dxgi, concrt140/msvcp140,
amd_ags, steam_api64). Those modules in turn import kernel32/etc., which PeekGame
backs. `unknown` there is expected, not a missing API. The one true gap flagged
below is Media Foundation.

## Risk surfaces to document as bring-up proceeds

Fill one entry per import (template in `docs/import-tracking.md`) as each is first
exercised and verified at runtime. Likely order and known risks:

- **D3D12 / vkd3d-proton present path** — confirm `d3d12core.dll` loads, the
  swapchain and first present go through DXGI; whether `amd_ags_x64` vendor
  extensions are called or gracefully skipped (RE ENGINE can run without AGS).
- **Shader cache build** — the real test after the splash; heavy threading
  (CONCRT140 concurrent queues) and filesystem writes to the save/cache prefix.
- **MFReadWrite.dll!MFCreateSourceReaderFromByteStream** — Media Foundation source
  reader for intro/cutscene video. **Not implemented**; expected STOP when the
  first movie plays. Decide: implement a reader backend or confirm the engine
  tolerates its absence.
- **Networking** — WS2_32 (25) + WINHTTP (13): telemetry / RE:NET. Capture whether
  any is load-bearing for the menu.
- **Input** — DINPUT8 `DirectInput8Create` (partial) and XINPUT1_3; verify keyboard
  and pad reach the engine.
- **Steam** — `steam_api64.dll` init via the bridge; confirm no DRM shortcut.

## Verification log

- 2026-10-18: static `--analyze` captured (above). RE ENGINE boot splash observed
  rendering under PeekGame on the `doom-eternal-bringup` branch. No runtime
  import/dynamic trace captured yet — next step is a traced run to record the first
  real stopping point and begin per-import entries.
