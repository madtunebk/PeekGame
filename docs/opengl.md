# Native OpenGL / WGL

The Windows OpenGL API now forwards through typed Win64-to-SysV adapters to host
libGL. WGL uses GLX on the existing X11 windows. Native GLX context pointers stay
inside the runtime; games receive owned HGLRC tokens. This path uses no SDL,
ANGLE, DXVK or D3D compiler for OpenGL rendering. D3D11 and Vulkan retain their
existing routes.

## Modules and build

- `src/api/graphics/opengl/pixel.rs`: per-window pixel format, choose/describe/set.
- `contexts.rs`: legacy/attribute contexts, sharing at creation, thread ownership,
  current DC/context, swap, deletion and worker-exit release.
- `extensions.rs`: WGL procedure lookup, extension strings and native swap interval.
- `src/host/graphics/glx/`: host GLX operations, matching the window's actual visual.
- `tools/gen_gl_bridge.py`: typed adapters generated in Cargo OUT_DIR from host
  Khronos GL headers. Pointer returns, float/double and stack arguments keep their
  actual C signatures; Windows callbacks are excluded from generic forwarding.

Build requires the host GL/X11 development headers and libGL. `--relink` also links
libGL. Run `cargo build --release --offline` and `python3 tools/stage_dist.py`.
`PEEKGAME_TRACE_GL=1` enables format/procedure diagnostics.

## Verification

`python3 tests/test_opengl_bridge.py` runs on private Xvfb/llvmpipe, both loaded PE
and relinked ELF. It verifies a GL3.3 core context, real GLSL compile/link/draw and
pixel readback, float/double ABI, pointer returns, mapped buffers, resize, shared
objects, cross-thread ownership exclusion, worker-exit release and invalid handles.
The existing window/TLS tests pass. The D3D11 relink fixture presents six Vulkan
frames, reads back pixels and exits cleanly. Its cleanup required the real
`RtlDllShutdownInProgress` process-state query, also present in the earlier project.

A bounded private Gundam 4.7.2 launch rendered the main menu using Mesa GL4.5 core;
`logs/gundam-native-opengl.png` records the local screenshot. There were no HLSL
compiler/shader-link errors. This is software-rendering evidence, not a hardware,
full-game or clean-shutdown certification. WASAPI flags and OLE drops remain
separate limitations. Window sizing on the user's desktop still needs confirmation.

## Scope

One installed pixel format per window, RGB >=8-bit components, chosen from GLX
formats compatible with the existing X visual. Reapplying the same format is
idempotent. Context sharing must be requested at creation; `wglShareLists` returns
ERROR_NOT_SUPPORTED. Negative swap intervals and unsupported creation attributes
fail. Swap-control extensions are advertised only when the GLX provider supports
them. Xvfb may provide no swap control. Callback-taking GL entry points (including
debug callbacks), overlays, pbuffers, stereo and changing pixel formats are not
implemented; unsupported procedure lookups return NULL. Availability/version of
ordinary GL extensions remains the host driver's responsibility.
