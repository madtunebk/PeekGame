# Gundam Survivors startup findings

## AppData and shader cache

The earlier compatibility notes identified the APPDATA requirement. PeekGame
already had shell folder mapping, but initialized APPDATA only after a known-folder
API was called. Godot reads the environment directly earlier in startup.

Process configuration now creates/resolves APPDATA before entering any PE code.
The existing default (`$XDG_DATA_HOME/peekgame`, otherwise
`~/.local/share/peekgame`) is preserved. A supplied APPDATA remains authoritative;
no save directories are moved. Initialization failure is reported.

`tests/test_early_appdata.py` verifies early visibility, a real directory, Unicode
and consistency with SHGetFolderPathW, including caller overrides.

A private Xvfb Gundam 4.7.2 run with isolated XDG_DATA_HOME created:

```text
Godot/app_userdata/Gundam Survivors — Operation Stardust/shader_cache/
Godot/app_userdata/Gundam Survivors — Operation Stardust/shader_cache/EGL/
```

The shader-cache directory error disappeared. This does not establish shader
compilation: ANGLE's missing D3D compiler remains a separate issue.

## Resource icons

CreateIconFromResourceEx supports resource PNG and uncompressed 32-bit DIB,
including Windows AND-mask transparency when the DIB has no alpha, and nearest
neighbor resizing. Unsupported cursor formats/flags fail explicitly. libpng is a
host-only dependency; Windows receives an owned icon handle. WM_SETICON/GETICON
track per-window handles and the big icon is published as X11 _NET_WM_ICON.
DestroyIcon validates ownership and lifetime.

`tests/test_resource_icon.py` verifies PNG/DIB decoding, resize, handle lifetime,
message behavior and the exact ARGB pixel in the native X11 property on Xvfb.
Static relinking now also links libpng16.

The same private game run passed resource icon creation and stopped next at
USER32!RegisterClipboardFormatA. WASAPI initialization still rejects a stream
flag and remains to be diagnosed; rendering/playability are not yet verified.

## Native OpenGL rendering

The GLX/WGL bridge now renders the main menu without ANGLE or a D3D compiler.
The private llvmpipe capture is `logs/gundam-native-opengl.png`; the renderer reports
Mesa GL4.5 core. Real GLSL/readback tests pass as both PE and relinked ELF.
See `docs/opengl.md` for scope and verification. WASAPI stream flags remain
unsupported; no claim of audio or full-game stability is made.

`MoveFileW` is now registered and shares atomic rename-without-replacement behavior
with the filesystem backend. Unicode/relative/Z: paths, directories, existing
files (including dangling symlinks), missing paths and null/invalid UTF-16 inputs
are covered by `tests/test_move_file.py`. Cross-filesystem copy/delete remains
unsupported. No Windows-only Rust imports are required.

## Audio and quit follow-up

Godot shared playback now accepts RATEADJUST with a real PulseAudio variable-rate
stream and IAudioClockAdjustment, including validation; the user confirmed sound.
The three modes in `tests/test_wasapi.py` pass against an isolated captured sink.

Quit also exposed a separate CRT bug: `_endthreadex` used pthread_exit and skipped
Windows thread completion bookkeeping. It now uses the ExitThread return boundary.
`tests/test_endthread.py` passes, including FLS callbacks and signaled joins.
A private real-game `--quit-after 120` run exits with code 0 instead of hanging.
See `engines/godot-imports.md`; the static unresolved inventory is separate from
which API paths have been exercised.
