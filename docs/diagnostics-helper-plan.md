# Guided Compatibility & Diagnostics Helper — plan (not scheduled)

Status: **planned, not started.** Deferred (not this week). Feasibility report only; no
code written. Estimates are coarse, assume one dev familiar with the repo.

Goal: a guided helper that inspects a game's imports, attributes crashes/hangs to the
right component (game / PeekGame / DXVK / etc.), organizes per-run diagnostics, helps
investigate hangs with **evidence + uncertainty (never a verdict from one log line or a
module name)**, and supports small, optional, reversible, per-game fixes — with
built-in modules kept separate from opt-in community ones.

## 1. What already exists (reuse, don't rebuild)

- **Static import/dependency analysis** — `--analyze` → `src/analysis/`, `src/report/{json,text}.rs`.
  Recursive dep walk, cross-referenced against the registry (`backend_status`:
  implemented/partial/unknown). Already states "imports are clues, not proof"
  (`report/json.rs` limitations). ~80% of capability #1.
- **Module registry + attribution** — `src/images/` (`all`/`by_address`/`by_name`/`by_handle`;
  `Module` has name/path/base/size), load logging (`LoadLibrary: mapped <path> at 0x… (0x… bytes)`),
  and `src/seh/dispatch.rs:275` `format!("{}+{:#x}", m.name, addr - m.base)` — this is what
  prints `UnityPlayer.dll+0xdb6253`. ~95% of capability #2.
- **Runtime tracing** — `PEEKGAME_TRACE_*` (CERTS/GL/INPUT/AUDIO/FILES/CRYPTO/HTTP/HTTP_BODY/
  SSPI/WINDOW/SOCKETS + HTTP callback enter/exit), `TRACE_LAST_ERROR`, `IMPORT_TRACE`, and
  `logs/<stem>/dynamic-imports.tsv` (every runtime GetProcAddress/LoadLibrary). Full SEH
  unwinding + `STOP` with module+offset and stack dump (`src/seh/`, `api/system/process.rs`).
- **Thread-state primitives** (no orchestrator yet) — `host/teb.rs` tracks every live TEB
  (`TEBS`); `api/threading/suspend.rs` captures a thread's `CONTEXT` via a signal
  (SuspendThread/GetThreadContext); `seh/backtrace.rs` unwinds from a `Context`. PeekGame runs
  **in-process**, so it can suspend+capture+unwind its own threads **without an external
  debugger** — a real advantage.
- **Per-game reversible fixes** — `src/quirks/` (`Quirk` enum, `PROFILES`, engine predicates
  `is_godot`/`is_unreal`, `active()`), plus env toggles (`PEEKGAME_NO_XI2`,
  `PEEKGAME_HTTP_OFFLINE`, `VULKAN_*`). Compile-time today.
- **Per-run logs** — `tools/peek_compat_tool/peek-launch` writes timestamped logs +
  `latest.log` symlink per run in `~/.local/state/peek-compat/`, keyed by SteamAppId; plus
  `logs/<stem>/` and `reports/<stem>/`.
- **Packaging** — `src/package/`.

## 2. Largest missing pieces / blockers

1. **All-thread stack snapshot at a hang** — biggest gap for hang diagnosis. Pieces exist
   (TEBS + suspend + unwind); nothing assembles them, and doing it safely (suspending running
   threads, signal-safe unwinding) is the hard part. Not architecturally blocked.
2. **Unified, structured per-run bundle** — output is scattered (stderr, dynamic-imports.tsv,
   reports/<stem>/, ~/.local/state/peek-compat/). No folder tying exe hash + args + env +
   outcome + logs together. Plumbing, not deep tech.
3. **Community-vs-built-in isolation (capability #6)** — only genuine architectural gap.
   Safe/tractable form = **declarative** opt-in data files that enable/parameterize existing
   built-in quirks and can NEVER disable the base layer. Arbitrary community **code** (native
   fix ABI, trust, sandbox) is a large, risky subsystem — defer or avoid.
4. **A/B run comparison** — nothing records "fix X on in run A, off in run B, outcome delta."
5. **The "guided" UX** — a driver that suggests the next trace and correlates evidence. Must
   show evidence + uncertainty; never guess a cause from one line / a module name.

## 3. Staged plan (smallest useful first)

- **Stage 0 — Run bundle + manifest** *(isolated)*: each `--run` writes
  `reports/<stem>/runs/<timestamp>/` with exe sha256, args, active `PEEKGAME_*` env, start/end,
  exit/STOP summary, + the stderr log, `dynamic-imports.tsv`, and the static analyze report;
  `latest` symlink + index. Underpins everything.
- **Stage 1 — Structured exception record** *(isolated)*: on STOP also emit JSON — exception
  code, faulting module+offset, stack frames (each module+offset), full loaded-module map.
  Reuses `seh/dispatch.rs` + `images`.
- **Stage 2 — On-demand all-thread snapshot** *(core-touching, careful)*: a signal/env trigger
  walks TEBS, suspends each thread, captures context, unwinds, writes every thread's stack with
  module+offset into the bundle. Hang-diagnosis centerpiece.
- **Stage 3 — Hang heuristics (evidence, not verdicts)** *(isolated)*: classify each thread's
  top frames by module/API (recv/select → Winsock; WinHttp* → WinHTTP; .bk2 read → Bink;
  vkAcquire/present → graphics; WaitForSingleObject → sync) and present **per-thread evidence
  with explicit uncertainty**. Pure analysis over Stage 2 data.
- **Stage 4 — Fix toggles + A/B + declarative community layer** *(moderate + one design)*:
  record active fixes in the manifest; add an opt-in, versioned per-game **data** file layered
  OVER built-ins (built-ins win on conflict; the file can only enable/parameterize, never
  disable the base — capability #6 in safe form); add a `compare` over two run bundles.
- **Stage 5 — Native community modules** *(large, optional)*: stable fix ABI + load path +
  trust/sandbox. Only if declarative isn't enough. Defer, possibly indefinitely.

## 4. Rough effort (coarse)

| Stage | Effort |
|---|---|
| 0 Run bundle | ~2–4 days |
| 1 Structured exception record | ~1–2 days |
| 2 All-thread snapshot | ~1–2 weeks (the hard one) |
| 3 Hang heuristics | ~3–5 days |
| 4 Fix toggles + A/B + declarative layer | ~1–2 weeks |
| 5 Native community modules | several weeks+ (defer) |

Stages 0–4 together ≈ **5–7 weeks**. First useful increment (0 + 1) ≈ **1 week**.

## 5. Major changes vs isolated additions

- **Isolated, low-risk:** Stages 0, 1, 3 — hang off existing hooks (run start/exit, SEH
  dispatch, images), no behavior change.
- **Core-touching but additive:** Stage 2 — uses existing suspend/unwind; care, not a rewrite.
- **One real design job:** Stage 4 isolation model (resolution order + declarative schema in
  `src/quirks/`).
- **New subsystem:** only Stage 5 (ABI/trust).
- Nothing requires touching the loader, PE reader, or API registry core.

## 6. First milestone + verification (using existing games/logs)

**Milestone = Stage 0 + 1:** every run leaves a self-contained, findable bundle, and any STOP
is attributed to module+offset with the full module map and stack, in human + JSON form.

Verify against assets already in the repo:
- **DOOM** (`DOOMEternalx64vk`): the `vkAcquireNextImageKHR VK_NOT_READY` STOP and the
  HTTP-offline scenario → bundle captures manifest + HTTP trace + structured attribution.
- **Gundam Survivors / Godot**: the dinput8 `0x504702dc` crash → attributed to
  `BUILTIN_HANDLES` + game module, with `dynamic-imports.tsv` showing the dinput8 LoadLibrary
  immediately prior.
- **Crash Bandicoot** (Unity+DXVK): `VK_ERROR_NATIVE_WINDOW_IN_USE_KHR` → module map separates
  d3d11/dxgi (DXVK) from UnityPlayer and the game.
- Existing `tests/test_*.py` can assert the bundle/manifest is produced and well-formed.

## Assumptions / honest limits

- Keep the "imports are clues, not proof" stance from `--analyze`. **Never infer a fix
  automatically or treat a module name as a cause.**
- **Feasible now:** Stages 0, 1, 3, and the declarative half of 4. **Needs deeper runtime
  work:** Stage 2 (thread suspend/unwind — primitives exist, but it's the real engineering)
  and Stage 5 (native modules — new subsystem).
- No external debugger needed for Stage 2 thanks to in-process suspend+context+unwind.

## Refined workflow (user's vision) + community-module validation model

Intended flow: insert a new game → see its imports and which are actually used → drop a
community fix like `modules/community/mousefix.<ext>` → PeekGame validates it against the
core and **rejects it if it is broken or would overwrite/disable base imports**.

**Intake ("first imports, used or not")** = combine two existing signals, no new runtime work:
- `--analyze`: what the game *could* call + base status (implemented/partial/missing) — static.
- `logs/<stem>/dynamic-imports.tsv`: what it *actually* requested at runtime — live.
- The helper shows a **diff**: used-and-supported / used-and-missing / declared-but-never-called.
  Small; part of Stage 0/1 plus a diff view.

**Community fixes — the safe, enforceable (declarative) model:**
- A community fix is a **versioned data manifest** (TOML/JSON under e.g. `modules/community/`),
  opt-in, that can only **enable or parameterize existing knobs** (quirks in `src/quirks/` +
  env-style toggles like `PEEKGAME_HTTP_OFFLINE`, `PEEKGAME_VULKAN_PRESENT`, `PEEKGAME_NO_XI2`).
  It **cannot register or rebind an API** — the base registry is the single source of truth and
  already always wins ("DLLs/APIs PeekGame implements win over files on disk").
- **Validate-or-reject** is then enforceable at load: schema-check → confirm it references only
  **known** knobs and touches **no base import** → else **reject + log the reason**, base layer
  untouched. Resolution order: built-ins first; community file may add/tune, never override or
  disable. Each fix carries a version + target (game stem / engine predicate) so it is auditable
  and A/B-comparable (Stage 4).
- Most fixes built this session are already expressible this way (mouse XI2 default, present
  mode, HTTP offline, dinput8 suppression) — good sign the declarative model is sufficient.

**Honest limits of the vision:**
- `.inc` is a C/peek textual-include idiom; the runtime-loadable equivalent in Rust is a **data
  file**, not an included source file. A community fix that needs *new code* (a different mouse
  *implementation*, not a toggle) is the native-module path (Stage 5): a stable ABI + trust +
  sandbox, where "validate it's not broke" is not static — it becomes *run isolated, auto-disable
  on crash*. Recommend declarative first; treat native modules as a separate, later decision.

### Module format lineage (sdl3-window `.inc`)

The user's module mental model comes from the prior project `../sdl3-window/src/modules/`:
C **code modules**, one per subsystem (`window/`, `filesystem/`, `com/`, `crt/`, …), each
implementing Windows APIs as `WIN`-ABI functions and textually `#include`d (the
`peek win32_window.inc` lineage in CLAUDE.md). Example: `window/dragdrop.inc` implements
GlobalAlloc/Lock/Free etc. **This is the direct ancestor of PeekGame's `src/api/<subsystem>/`
Rust modules + `src/registry/`.** So "built-in modules" already exist in PeekGame; the
community layer wants the same *code-module* feel, not just data toggles.

Because the registry is the single source of truth and emits one link symbol per export
(`peekgame.<dll>.<name>`), "validate or reject if it overwrites base imports" is concrete: a
community module registers exports the same way, and PeekGame **rejects any export that
collides with a base/built-in registry entry** (built-ins always win) and anything that fails
to build/load or pass tests. The fork for the community layer:

- **(A) Compile-time code modules** — `modules/community/<name>.<rs|inc>` added under a
  community dir, pulled into the build; validation = **no registry collision + tests pass**.
  Closest to the `.inc` feel, fully enforceable, moderate effort. Not runtime drop-in
  (rebuild to add one). **Recommended starting point for the module system.**
- **(B) Runtime-loadable `.so` modules** — true drop-in like a dropped `.inc`, but needs a
  stable ABI + trust + run-isolated/auto-disable-on-crash. Stage 5 (large).
- **(C) Declarative data toggles** — safest, runtime, but only flips existing built-in knobs,
  not the code-module feel. Good for trivial per-game fixes (present mode, offline, suppress).

Likely direction: **(A)** for the community code-module system (registry-collision validation
is the "can't break/override the base" guarantee), with **(C)** for trivial toggles and
**(B)** deferred.

