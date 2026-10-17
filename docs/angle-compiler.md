# ANGLE compiler investigation

ANGLE was the fallback before the native OpenGL/WGL bridge was added.
Godot now renders its Gundam main menu through native GLX on a private Mesa
software display. ANGLE remains an alternate path; its HLSL compiler DLL must
be supplied separately. See `docs/opengl.md` for the current bridge and tests.

A locally owned `d3dcompiler_47.dll` was tested in isolation. It maps successfully
but currently stops at `msvcrt!__dllonexit`; its imports include additional C++
ABI and crypto APIs beyond the old msvcrt.inc subset. It is kept under ignored
`cache/d3dcompiler/input/`, outside the active distribution. No compiler DLL is
redistributed or committed.

To reproduce the optional, currently blocked probe:

```sh
PEEKGAME_D3DCOMPILER=/path/to/d3dcompiler_47.dll python3 tests/test_d3dcompiler.py
```

The fixture calls D3DCompile on a real HLSL pixel shader and requires a DXBC blob.
This test must pass before activating this compiler for games.

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
