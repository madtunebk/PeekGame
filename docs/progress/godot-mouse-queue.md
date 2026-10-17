# Godot mouse motion queue progress

## Symptom and cause

Gundam Survivors displayed a hover highlight far from the pointer. After
moving the mouse, the highlight could take 30–40 seconds to reach the new
position. The X11 pump sends each motion as `WM_INPUT` followed by
`WM_MOUSEMOVE`. The old queue coalesced `WM_MOUSEMOVE` only when it was the
immediately preceding message, so a burst of alternating messages could leave
the game processing old pointer positions.

## Changes

- `src/api/window/queue.rs` merges pending move messages across unread raw
  motion and keeps the newest move at the tail of the queue. It does not cross
  buttons, wheel input, keyboard messages or another window's messages.
- `src/api/window/input/raw.rs` accumulates relative deltas in an unread raw
  motion packet. Packets observed by `GetRawInputData` stay immutable. Failed
  checked addition leaves the original packet alone.
- `tests/test_godot_input.py` now sends 2500 pointer warps to a private Xvfb
  window, then checks that a click at the final position reaches the Godot
  button promptly.
- `.gitignore` ignores the repository-root `progress/` scratch directory.
  This report under `docs/progress/` remains tracked.

## Verification

- `cargo check --offline`: passed.
- `cargo build --release --offline`: passed; `python3 tools/stage_dist.py` ran.
- `python3 tests/test_godot_input.py`: passed after correcting the test's final
  click coordinate for the last 960×540 window size. The test checks three
  window sizes and the motion burst.

The Xvfb probe confirms timely Godot UI clicks; it does not reproduce every
input mode in Gundam. The owner can now verify hover and mouse control in the
actual game. No game files were changed or desktop game run started here.
