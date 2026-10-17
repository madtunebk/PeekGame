# Runtime modularization

## Stage 1: completed structural split

Functional baseline: GetThreadLocale/SetThreadLocale now use per-thread state,
creation-time inheritance and TEB.CurrentLocale. PE/ELF, TLS and CRT exit tests pass.
A private Touhou probe passes that call; it then faults at DXGI +0xe44b on lavapipe.
The user's hardware run reaches scene loading and later requests stack capture.
Neither result establishes complete game support.

`src/registry/system.rs` is now a 42-line catalog index. Its 347 unchanged export
declarations live in 16 subsystem tables under `registry/system/`; the largest
is 337 lines. Complete declarations (including target functions and scope strings)
were compared with the local pre-migration snapshot. Existing export order is
not a stable public API; names and bindings are preserved.

`api/window/input.rs` is now a 45-line facade over:

- `input/keyboard.rs`: key state, layouts and translation (202 lines).
- `input/focus.rs`: focus, activation and capture (176 lines).
- `input/cursor.rs`: position, confinement and cursor objects (226 lines).
- `input/raw.rs`: registration, records and buffered delivery (450 lines).

Raw state stays private to raw.rs; window destruction and cursor warps call small
ownership helpers. Entry points and Win64 signatures stay compatible.

## Verification

All 1,583 `peekgame.*` symbols remain identical as a set. The registry unit test
checks every catalog entry resolves to its declared implementation without duplicate
DLL/name pairs. Window ABI tests now include real X11 motion/buttons, raw bytes,
cursor coordinate round trips, capture and button state. Mouse hooks, OLE drops,
GL shader/readback PE/ELF, locale PE/ELF and import ABI tests pass.

Local backups and declaration/symbol snapshots: `cache/reorganize-stage1/`.
No FPS, memory reduction or game-specific runtime pruning is claimed by this split.

## Development rules and next stages

Keep registry roots as composition only; add declarations to the owning leaf.
Split implementation files when responsibilities diverge; roughly 300–500 lines
is a review prompt, not a reason to split one coherent algorithm. Keep host backend
objects behind the Win64 API boundary and document ownership/lock order.

Next: define the per-game dependency manifest and generated export catalog. The
current global registry references implementations and retains the complete runtime.
Source modularity alone does not prune it. Build selection must include EXE/DLL
static imports, known dynamic imports and their transitive implementation/backend
dependencies, with explicit failure for uncovered paths. A trace is not exhaustive.

A minimal runner must load the selected runtime without embedding the full global
one. Cache artifacts by game inputs, selected implementations, build options and
runtime/backend versions. Pin working game artifacts until explicit rebuild;
retain rollback versions. Rebuild cannot transparently preserve a running game's
state when a previously unseen dynamic import appears.

Per-game library construction and reduced startup dependencies are **not yet
implemented**. Benchmark startup, RSS, frame timing and contention before claiming
performance improvements.
