# tests/testgame: a Windows x64 D3D11 test game for the native runner

`testgame.exe` is a small Windows program we own: no DRM, nothing from RE3. It does what a D3D11 game does once it is running:
1. Registers a window class and creates a window.
2. Creates a D3D11 device and a DXGI swap chain.
3. Pumps messages with `PeekMessageW(PM_REMOVE)`.
4. Clears and presents frames, cycling red, green and blue every 60 frames.
5. Handles `WM_SIZE` (calls `ResizeBuffers`), `WM_KEYDOWN` (Esc closes the window), `WM_CLOSE` → `DestroyWindow` → `WM_DESTROY` → `PostQuitMessage`.
6. Verifies the first frame by copying the back buffer to a staging texture and checking the centre pixel.

**Purpose:** it is the milestone target for `peek`'s window and D3D11 layer. Everything it needs, RE3 needs too once it gets past its launch check.

**Owner split:** this directory is test material and a spec. The `peek.c` work is Codex's.

## Build

```sh
make            # x86_64-w64-mingw32-gcc, no C runtime: testgame.exe + imports.txt
```

- `-nostdlib -nostartfiles -Wl,-e,entry`: there is no CRT. Every import is a real Win32/D3D API, and `imports.txt` is the exact list `peek` must provide. The program supplies its own `memset`/`memcpy`.
- **Image:** base `0x140000000` (the same as `re3.exe`; `peek` needs the preferred base), console subsystem, 16 sections, entry RVA `0x1d30`. `peek` maps it fine (`peek testgame.exe` → "Inspection only").
- **Arguments:** `testgame.exe [--frames N]`, default 300 frames (`0` = run until closed). `peek` doesn't forward arguments yet, so runs under `peek` use the default.
- **Output:** every message goes to stdout through `WriteFile(GetStdHandle(STD_OUTPUT_HANDLE))` and also to `OutputDebugStringA`. Both already exist in `peek`.
- **Exit codes:** 0 pass, 1 setup failure (prints `FAIL <api> hr=0x...`), 2 readback mismatch.

## Status under peek (2026-10-13): PASS

`make test` runs `run_under_peek.sh`, which uses a private Xvfb and llvmpipe. It ends with `RESULT PASS`, exit 0.

- **All 20 imports are provided.** `GetModuleHandleW` and `ExitProcess` are in `peek.c`. The USER32 subset is in `src/modules/window/win32_window.inc`, on the shared SDL3. `D3D11CreateDeviceAndSwapChain` plus the COM methods are in `src/modules/graphics/d3d11/d3d11_bridge.inc`, on DXVK Native.
- **On the owner's desktop:** the RTX 3060 through DXVK (llvmpipe skipped). Readback `0xcc1919ff`, 300 frames, `RESULT PASS`.
- **On Xvfb:**
  - Mid-run screenshots show the window's colours cycling.
  - Esc → `WM_KEYDOWN vk=0x1b` → `WM_CLOSE` → `WM_DESTROY` → exit 0.
  - Readback every 100 frames matched the expected colour each time.
  - llvmpipe on Xvfb has no vsync, so 300 frames take about 0.25 s.

The rest of this file is the original spec, kept as the reference for each API's semantics.

## What peek must implement: exact semantics this program depends on

All entry points are `ms_abi`, like everything else in `resolve()`.

**Window procedures.** `WNDPROC` values come from Windows code. The loader calls them as `LRESULT (__attribute__((ms_abi)) *)(HWND, UINT, WPARAM, LPARAM)`, on the thread that owns the window. That is the `nw_init` thread from `window/`.

### KERNEL32

| API | Semantics needed here |
|---|---|
| `GetModuleHandleW(NULL)` | Returns the main image base (`0x140000000`). The wide form of the existing `GetModuleHandleA`; non-NULL names should behave like the A version. |
| `ExitProcess(code)` | Flush, then `exit(code)`. Should not return. |

### USER32 (backend: `window/`'s `nw_*` API, or SDL3 directly)

| API | Semantics needed here | Backend |
|---|---|---|
| `RegisterClassExW(&wc)` | Validate `cbSize` and store `lpfnWndProc`, `hInstance` and the class name (UTF-16). Return a non-zero ATOM. A duplicate name returns 0 with `ERROR_CLASS_ALREADY_EXISTS` (1410). | loader table |
| `AdjustWindowRectEx(&rc, style, menu, exstyle)` | Return TRUE. **Recommended model: zero non-client area.** SDL sizes are client sizes and the compositor draws the decorations, so return `rc` unchanged. `CreateWindowEx` sizes then become client sizes. `GetClientRect`/`GetWindowRect` must use the same model later. | none |
| `CreateWindowExW(...)` | Look up the class, then `nw_window_create(title→UTF-8, w, h, RESIZABLE if WS_THICKFRAME)`. The window must also get `SDL_WINDOW_VULKAN` (see "Blockers"). Allocate an `HWND` (a loader handle, **not** the `SDL_Window*`) mapped to the `NwWindow`. Send `WM_NCCREATE`/`WM_CREATE` through the WndProc. Send an initial `WM_SIZE(SIZE_RESTORED, MAKELPARAM(w,h))`, because `nw_*` emits no resize at creation (see `window/README.md`). This program ignores the size if it is unchanged. `CW_USEDEFAULT` for x/y means the platform picks the position. | `nw_window_create` |
| `ShowWindow(hwnd, SW_SHOW)` | Returns whether the window was previously visible (FALSE here). SDL windows are shown at creation. A later `SW_HIDE`/`SW_MINIMIZE` maps to `SDL_HideWindow`/`SDL_MinimizeWindow`. | SDL |
| `LoadCursorW(NULL, IDC_ARROW)` | A non-NULL handle for the system arrow. This program only stores it in the class. Back it with `SDL_CreateSystemCursor(SDL_SYSTEM_CURSOR_DEFAULT)`, or a loader token resolved later by `SetCursor`. Return NULL with an error for resource IDs that aren't supported. | SDL |
| `PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)` | Pump `nw_peek_event`, translate the events into the loader's per-thread `MSG` queue, deliver sent messages, then return the first match. The queue model, filters and `PM_*` semantics are spelled out in `window/README.md` under "Mapping to Win32 messages". `WM_QUIT` (wParam = exit code) comes back only after `PostQuitMessage`. | loader queue + `nw_peek_event` |
| `TranslateMessage(&msg)` | For `WM_KEYDOWN` it may post `WM_CHAR`. This program ignores `WM_CHAR`, so returning nonzero for key messages and zero otherwise is enough for now. Text input is a `window/` gap. | — |
| `DispatchMessageW(&msg)` | If `msg.hwnd` is a live window, call its class `WNDPROC` with `(hwnd, message, wParam, lParam)` and return the result. Thread messages (hwnd NULL) aren't dispatched. | loader |
| `PostMessageW(hwnd, WM_CLOSE, 0, 0)` | Append to the `MSG` queue and return TRUE. An invalid hwnd gives FALSE with `ERROR_INVALID_WINDOW_HANDLE` (1400). | loader queue |
| `DefWindowProcW(hwnd, msg, wp, lp)` | At minimum: `WM_CLOSE` → `DestroyWindow(hwnd)`, return 0. `WM_NCCREATE` → TRUE. Everything else returns 0. This program handles its own `WM_CLOSE`, but `CreateWindowExW` sends `WM_NCCREATE`/`WM_CREATE` and they land here. | loader |
| `DestroyWindow(hwnd)` | Send `WM_DESTROY`, then `WM_NCDESTROY`, to the WndProc. `nw_window_destroy`. Remove the HWND mapping and that window's queued messages. Return TRUE. | `nw_window_destroy` |
| `PostQuitMessage(code)` | Set the thread's quit flag. `PeekMessage`/`GetMessage` return `WM_QUIT` with `wParam=code` once the posted messages ahead of it have been retrieved. | loader queue |

**Event mapping for this program:**
- **Close:** `NW_EVENT_CLOSE` → send `WM_SYSCOMMAND(SC_CLOSE)` → `DefWindowProc` → `WM_CLOSE`, or send `WM_CLOSE` directly. The program then calls `DestroyWindow`.
- **Keys:** `NW_EVENT_KEY_DOWN/UP` → post `WM_KEYDOWN/UP` with a virtual-key code (Esc = `0x1B`).
- **Resize:** `NW_EVENT_RESIZE` → send `WM_SIZE`.

### d3d11.dll (backend: `deps/dxvk_host/` DXVK Native)

`D3D11CreateDeviceAndSwapChain` is an `ms_abi` thunk that calls DXVK's export. **Before the call:**
- set the real environment `DXVK_WSI_DRIVER=SDL3`
- copy `DXGI_SWAP_CHAIN_DESC` and replace `OutputWindow` (a loader HWND) with that window's `SDL_Window*`

**After the call,** wrap the three returned interfaces in Windows-side COM wrappers. Each wrapper has its own `ms_abi` vtable and holds the native pointer (see `deps/dxvk_host/README.md`, "Adapters the PE loader needs").

**First milestone:** a hand-written wrapper covering just the methods this program calls. Every other slot traps by name.

| Interface (vtable slot from the mingw headers) | Methods used |
|---|---|
| `ID3D11Device` | `CreateTexture2D` (5), `CreateRenderTargetView` (9), `Release` (2) |
| `ID3D11DeviceContext` | `OMSetRenderTargets` (33), `CopyResource` (47), `Map` (14), `Unmap` (15), `ClearRenderTargetView` (50), `ClearState` (110), `Release` (2) |
| `IDXGISwapChain` | `GetBuffer` (9), `ResizeBuffers` (13), `Present` (8), `Release` (2) |
| `ID3D11Texture2D` | `GetDesc` (10), `Release` (2) |
| `ID3D11RenderTargetView` | `Release` (2) |

The slot numbers count `IUnknown` 0-2 and were checked against the C `…Vtbl` structs in `deps/dxvk_host/prefix/include/dxvk/d3d11.h` and `dxgi.h`.

**Argument handling:**
- `OMSetRenderTargets` takes an **array** of interface pointers, each to be unwrapped.
- `CopyResource` and `Map`/`Unmap` take wrapped resources, to be unwrapped.
- `GetBuffer` with `IID_ID3D11Texture2D` returns a new wrapper.
- `ClearRenderTargetView` takes `const FLOAT[4]` by pointer, so there's no float-register remapping, but `ms_abi` handles that anyway.
- `Map` fills `D3D11_MAPPED_SUBRESOURCE`, whose layout is the same in both ABIs (pointer, UINT, UINT).

## Blockers already known (from `deps/dxvk_host/README.md`)

1. **One SDL3 per process.** DXVK `dlopen`s `libSDL3.so.0`, while `window/` links SDL3 statically. A window from the static copy isn't valid in DXVK's copy. Either switch `window/` to the shared `deps/dxvk_host/prefix/lib/libSDL3.so.0`, or build the USER32 layer directly on that shared SDL.
2. **`SDL_WINDOW_VULKAN`.** `SDL_Vulkan_CreateSurface` refuses windows created without it, and `nw_window_create` doesn't set it.
3. **No `nw_window_fill` once DXVK presents.** The software window surface and the Vulkan swapchain can't share a window.

## Success criteria

`timeout 20 ../peek testgame.exe --run` should print:
```
window created
WM_SIZE wParam=0
D3D11 device created, feature level 0xb000
readback rgba=0xcc1919ff          (frame 0 = 0.8, 0.1, 0.1, 1.0; 0xcc1a1aff on llvmpipe, ±1 accepted)
frame limit reached
WM_DESTROY
frames presented: 300
RESULT PASS
```
It should exit 0, with a red → green → blue window visible for about 5 s.

**Manual checks:**
- Resize the window: expect `WM_SIZE` and `resized width=…`, then presenting continues.
- Press Esc: expect `WM_KEYDOWN vk=0x1b`, `WM_CLOSE`, `WM_DESTROY`, exit 0.
- Click the close button: expect `WM_CLOSE` through `NW_EVENT_CLOSE`.

The program hasn't run anywhere yet: there's no Windows machine or Wine here, by choice. `peek` will be the first thing to run it.

**What has been verified:**
- it compiles with `-Werror`
- the import table is exactly `imports.txt`
- `peek` maps it
- `peek` stops at the expected first missing API
