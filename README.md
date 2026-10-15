# PeekGame prototype

A Rust static PE analyzer plus the first modular Windows API backend.
The analyzer reads PE files without executing them. The backend builds as a Linux
shared library and uses Windows x64 calling conventions for game-facing functions.
No Rust package dependencies are required. The window backend links directly to
Linux libX11 (development headers/library required at build time); SDL is not used.

## Analyze a game

```sh
cargo build --release --offline
./target/release/peekgame --analyze tests/testgame/testgame.exe
# The EXE-first form is also accepted:
./target/release/peekgame /path/to/game.exe --analyze --output reports/my-game
```

The analyzer writes `imports.json` (schema version 2) and `imports.txt` to
`reports/<exe-stem>/`, or the directory provided by `--output`. Existing reports
in that output directory are overwritten after successful analysis.

It reads normal and delay-load PE import tables, named and ordinal imports,
import hints, and IAT RVAs. It records x86-64 image metadata and recursively scans
DLLs found beside the EXE or importing module. Additional directories can be
provided with repeated `--dll-dir /path/to/dlls`. DLL matching is case-insensitive;
ambiguous matches and malformed binaries produce an error and nonzero exit status.
The search order is an explicit analysis policy, not Windows' complete DLL loader search order.

An external DLL is not labelled missing: a future backend may implement its APIs
without a physical Windows DLL. Every named import is checked against the Rust
export registry: `implemented`, `partial` (with scope), `missing`, or `unknown`.
This inventory is not a claim that the game can already execute. Ordinals have no
Rust implementation mapping yet and remain `unknown`.

## Scope

The prototype analyzes imports, builds `target/release/libgame_imports.so`, and
executes the owned CRT-free testgame through a small PE runner. The runner currently
links the same Rust runtime directly; the packaging command loads a package-local `.so` through its ELF launcher.
Per-game runtime code pruning is still pending. No PE machine code is translated.

The image is mapped at its preferred base, or elsewhere with its base relocations
applied when that range is taken (images without relocations still need it). All
layout, TLS and relocation data comes from the image through `src/pe/`, the one PE
reader shared by the analyzer, the runner and the relinker. Delayed binding,
ordinal binding, external PE DLL loading, SEH support, and arbitrary games are
not supported by this runner yet. Unsupported static imports stop before entry.
Dynamic lookup covers registered runtime modules; unavailable DLLs/exports return
Windows errors 126/127 and are recorded. A trace contains only observed requests,
so an empty trace never proves a game has no other dynamic imports.

For the current `tests/testgame/testgame.exe`, the report contains 20 imports:
7 KERNEL32, 12 USER32, and 1 D3D11. Its list was checked against the existing
`tests/testgame/imports.txt` inventory.

## Checks

```sh
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
cargo build --release --offline
python3 tests/test_imports_abi.py
python3 tests/test_window_abi.py
python3 tests/test_relink.py
```

Parser tests cover named and ordinal imports, delay imports, truncated files,
unmapped names, and missing descriptor terminators. The C ABI test loads the actual
`.so`, calls its resolved pointers through `ms_abi` signatures, verifies real bytes
written to pipes, thread-local errors, process configuration, unsupported calls,
debug output, and process exit in a child.

## Source layout

- `src/main.rs`: entry point and exit status.
- `src/cli.rs`: command arguments and output paths.
- `src/app.rs`: coordinates analysis and report writing.
- `src/model.rs`: analysis result types shared by modules.
- `src/analysis/`: dependency traversal and DLL search policy.
- `src/pe/`: PE headers, checked binary reads, import thunks, and parser tests.
- `src/report/`: independent JSON and text renderers.

Keep files focused on one responsibility. API implementations are grouped by
subsystem under `src/api/`, including `system/` and `filesystem/`. The export
registry maps DLL/function names onto these implementations. Add future `window/`,
`network/`, `audio/`, and `graphics/` modules as those implementations are ported.
`src/host/` holds Linux-side process and stream state; `src/abi.rs` holds Windows
types and constants; `src/runtime.rs` exposes the small Linux loader ABI.

## First shared library

`cargo build --release --offline` builds `target/release/libgame_imports.so`.
The host interface is declared in `include/peekgame_imports.h`:

1. Check `peekgame_abi_version()` (currently 1).
2. Call `peekgame_configure_process(mapped_pe_base, command_line)` once. It copies
   the supplied command line and registers the inherited standard streams.
3. Resolve imports with `peekgame_resolve(dll, name)`. NULL means unavailable.
4. Bind each non-NULL pointer into the PE IAT, then call with its exact Windows
   x64 signature. The prototype does not yet perform PE mapping or IAT binding.

The seven callable APIs are `GetLastError`, `GetCommandLineA`, `GetModuleHandleW`,
`GetStdHandle`, `WriteFile`, `OutputDebugStringA`, and `ExitProcess`. The report
states each supported scope. Named module lookup, arbitrary file handles,
overlapped writes, Windows debugger delivery, TEB integration, and PE DLL detach
callbacks are not implemented. `ExitProcess` exposes only the low 8 bits as the
Linux exit status. Inherited stdout/stderr may be pipes or files; data and lengths
come from each call, not from the test game's observed values.

All 12 USER32 imports now resolve to the native X11 backend. Their status remains
`partial`: basic decorated top-level windows, show/hide, owner-thread messaging,
resize, close, and limited keyboard text are covered. D3D11 remains missing; this
`.so` is not yet sufficient to run `testgame.exe`.

## Native window backend (no SDL)

- `src/api/window/`: Windows structures, class registration, creation/lifecycle,
  cursor lookup, message queues, and native-to-Windows event translation.
- `src/host/x11/`: checked-in Xlib ABI declarations based on the local system
  headers, display/window operations, event decoding, and keyboard translation.
- `src/registry/`: export metadata separated by subsystem.

Windows HWND/HCURSOR values are private tokens. Native Display pointers and X11
window IDs are returned only to Linux host code through `peekgame_x11_window`,
for a future Vulkan surface bridge. No Linux object is given to a Windows WndProc.
Callbacks use `extern "win64"` and run without registry locks or state borrows held.

Layout uses client dimensions, with decorations owned by the compositor;
`AdjustWindowRectEx` does not invent Windows border metrics. Unsupported menus,
child windows, extended styles, class extras/icons/brushes, or custom cursors fail
explicitly. Classes currently require style 0. ShowWindow covers hide, normal,
show, and show without activation; minimize, maximize, and fullscreen are pending.
The WM may apply its own placement and focus policy.

Input currently covers letters, digits, space, and Escape; native keyboard events
can post ASCII WM_CHAR messages. Raw mouse input, mouse capture/confinement,
extended keys, IME, paint-cycle handling, and cross-thread queues are pending.
X11 requires DISPLAY (Xwayland can supply it under Wayland); a native Wayland
backend is not implemented. Display unavailability fails rather than returning
fake windows or cursor handles.

`tests/test_window_abi.py` uses a private Xvfb server and never opens windows on
the desktop. Its C caller verifies Windows structure sizes and calling conventions,
real native dimensions and visibility, callback reentry, filtered/no-remove queues,
actual X11 resize/key/close events, and ordered destruction/quit notifications.

The existing DXVK Native build in sdl3-window uses SDL WSI. Graphics integration
must choose a separate X11/Vulkan WSI path rather than pass X11 tokens to an SDL
backend. This prototype does not yet implement that graphics bridge.

API behavior references are recorded in [docs/api-contracts.md](docs/api-contracts.md).

Future decompiler ideas are recorded in [docs/future-analysis-tools.md](docs/future-analysis-tools.md).

## Run the owned testgame and capture dynamic imports

```sh
python3 tools/build_dxvk_x11.py
cargo build --release --offline
./target/release/peekgame tests/testgame/testgame.exe --run --frames 300
```

The isolated DXVK Native build uses our X11 WSI, without SDL. D3D11/COM support
covers the testgame subset; other COM methods stop explicitly by interface/slot.
Actual Vulkan clear/readback/present/resize passed on private Xvfb with lavapipe.

Root directories:

- `cache/`: reserved for runtime caches; build products remain under `target/`.
- `logs/<exe-stem>/dynamic-imports.tsv`: flushed dynamic lookup events, statuses,
  and cumulative request counts. `--output DIRECTORY` before `--run` overrides it.
- `reports/`: static analysis JSON/text.

The testgame currently makes no dynamic export requests. A separate owned PE
fixture verifies successful lookup and missing export/module errors:

```sh
python3 tests/test_pe_runner.py
python3 tests/test_graphics_abi.py
```

The PE runner lives in `src/loader/`; runtime lookup/tracing lives in
`src/api/system/dynamic.rs`. This is execution of the original Windows machine
code through Rust Win64 API functions, not decompilation.

## Package the testgame

```sh
cargo build --release --offline
./target/release/peekgame tests/testgame/testgame.exe --package
./packages/testgame/play --frames 300
```

`--package --output NEW_DIRECTORY` chooses another destination. Existing
packages are refused so saved data and logs cannot be overwritten by a rebuild.

```text
packages/testgame/
  play                 ELF launcher (keep this filename)
  game.exe             unchanged original PE, renamed locally
  game_imports.so      loaded with dlopen; Win64 import functions come from it
  manifest.json        static imports, API scopes and prototype limitations
  runtime/             actual DXVK X11 libraries and DXVK license
  cache/               local XDG cache
  logs/                run.log, dynamic-imports.tsv and DXVK logs
```

The launcher resolves paths relative to itself, changes to its package directory,
forwards game arguments, and redirects stdout/stderr into `logs/run.log`. You can
move the directory and launch it from another working directory. It requires host
Linux/glibc, X11 and Vulkan drivers; it is not a fully self-contained Linux image.

The first package copies the **full prototype API library** and records the game's
static imports in its manifest. It does not yet compile only the selected API
implementations. Assets and external PE DLLs are not automatically bundled.
The launcher is the PeekGame ELF in package mode and still contains analyzer
code, but the executed game's imports resolve through the adjacent shared library.

`python3 tests/test_package.py` verifies a relocated package (including spaces in
its path), real Vulkan frames/readback, local logs, missing `.so` failure, and
refusal to overwrite an existing package. Artifacts stay under project `cache/`.

## Relink a PE into a native ELF

```sh
cargo build --release --offline
./target/release/peekgame tests/testgame/testgame.exe --relink
./relinked/testgame/testgame --frames 300
```

`--relink` writes one non-PIE Linux executable (default `relinked/<exe-stem>/`,
or `--output NEW_DIRECTORY`; existing directories are refused). There is no PE
loader in it, and no `game.exe` or `game_imports.so` beside it:

- The PE headers and every section become ELF sections linked at their original
  virtual addresses (`.peimage.NN`), so the kernel maps the image as ordinary
  `PT_LOAD` segments at the preferred base. Startup restores each PE section's
  own protection, because `ld` merges neighbouring sections into one segment.
- Every IAT slot is a relocation against `peekgame.<dll>.<name>`, and the static
  linker writes the address. Each registry entry defines that symbol (a direct
  jump; data imports use the variable's export name). The runtime is linked in
  statically from `target/release/libgame_imports.a`.
- Unimplemented static imports link to stubs that stop with
  `STOP: missing Windows API <dll>!<name>` (exit 78) only if they are called.
- `main` calls `peekgame_relinked_main`, which sets up the process, TEB and TLS
  just like `--run` and then jumps to the PE entry point.

The machine code is still the original Windows x64 code. It uses the Windows
calling convention and calls PeekGame's Win64 API implementations, so API
coverage limits it exactly as it limits `--run`. Relinking removes the loader;
it does not translate or recompile anything. Requirements: `cc` with GNU ld
(override with `CC`), the X11 development library, and for D3D11 the DXVK X11
build, which is copied into `runtime/` next to the executable.
`PEEKGAME_IMPORT_TRACE=FILE` records dynamic imports, as `--run` does.
Delay and ordinal imports are refused. The image always sits at its preferred
base, so base relocations are never applied.

`python3 tests/test_relink.py` checks the ELF layout, link-time IAT binding,
dynamic lookup, the missing-import stop and the relinked testgame on private Xvfb.

## First engine fixture

`tests/godot_image/` is a minimal Godot image scene, with a Windows export and
verified native Linux reference. See its README for build commands and current
PeekGame startup blockers. Run `python3 tools/build_godot_image.py` to rebuild.
