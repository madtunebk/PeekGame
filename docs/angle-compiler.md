# ANGLE compiler investigation

ANGLE was the fallback before the native OpenGL/WGL bridge was added.
Godot now renders its Gundam main menu through native GLX on a private Mesa
software display. ANGLE remains an alternate path; its HLSL compiler DLL must
be supplied separately. See `docs/opengl.md` for the current bridge and tests.

A locally owned `d3dcompiler_47.dll` from Palworld's CEF3 directory now passes
the isolated HLSL-to-DXBC probe after adding `msvcrt!__isascii` and
`msvcrt!_clearfp`. The DLL is copied only into the user's local
`dist/windows-dlls` and installed Steam compatibility tool for a live Palworld
test; it is not redistributed or committed. The first live test still failed
because Steam launched `./dist/peekgame`, whose separate `dist/windows-dlls`
directory lacked the compiler. The CrashReportClient dynamic import trace
recorded two `module_not_found` results for that DLL. Other engine DLLs may
still need CRT imports when exercised.

After copying the DLL to `dist/windows-dlls`, the next live run loaded it and
reached `msvcrt!_mbstrlen` in CrashReportClient. The bridge now supplies that
function for its supported single-byte C locale. The isolated D3DCompile probe
continues to pass; the live CrashReportClient path needs another run.
The following live run reached `msvcrt!_fpclass`; PeekGame now returns the
MSVC `_FPCLASS_*` mask from the IEEE-754 bits of a double, including signed
zero, denormals and signaling versus quiet NaNs. The shader probe still passes.

To reproduce the optional, currently blocked probe:

```sh
PEEKGAME_D3DCOMPILER=/path/to/d3dcompiler_47.dll python3 tests/test_d3dcompiler.py
```

The fixture calls D3DCompile on a real HLSL pixel shader and requires a DXBC blob.
It passed with the Palworld CEF3 copy on 2026-10-18. Live CEF renderer validation
is still pending.

The `user://` directory error was fixed separately by initializing APPDATA before
any game code runs. A private Gundam launch created its app_userdata/shader_cache
and EGL directories without that error. This establishes directory creation,
not successful HLSL shader compilation. Godot's unsupported WASAPI initialization
flags remain a separate issue.

## Selected direction

The owner selected native OpenGL/WGL on X11 for Godot as the next implementation
path. The earlier SDL window prototype provides window/event reference material,
not a ready OpenGL bridge. Reuse the earlier OpenGL ABI approach, adapting context
and presentation to the local X11 backend. Preserve Vulkan for Bevy and D3D11/DXVK
for Unity. Avoid engine-specific magic values in shared APIs.

Keep ANGLE/HLSL as a future alternate path. The locally tested compiler has
additional CRT, C++ exception ABI, private heap and crypto dependencies, beginning
with __dllonexit. The older msvcrt.inc's 183 exports do not cover all compiler
requirements. Require the isolated real-DXBC compiler test to pass before staging
this DLL for games. Do not activate an unverified compiler merely to hide error126.
