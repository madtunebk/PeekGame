# Local dependency setup

PeekGame uses only its own directory plus installed Linux build tools/drivers.
The earlier project is optional reference material, never a build/runtime input.

## DXVK Native (X11, no SDL)

Pinned upstream revisions are in `deps/manifest.json`; the imported DXVK change
is preserved in `deps/dxvk/imported-source.patch`. Large snapshots stay ignored.

```sh
python3 tools/fetch_sources.py          # explicit network operation, once
python3 tools/build_dxvk_x11.py         # local/offline build after source setup
cargo build --release --offline
make -C tests/testgame
```

Sources live under `sources/{dxvk,glslang}`. The local shader compiler lives at
`deps/shader-tools/bin/glslangValidator`; when missing, it is built from the local
glslang snapshot. To rebuild explicitly: `python3 tools/build_glslang.py`.
Native DXVK work/output lives under `target/dxvk-x11`; `--build-root DIRECTORY`
allows another clean build. Requires C/C++, CMake, Ninja, Meson, pkg-config and
DXVK's X11/Xrandr/Vulkan development dependencies. No system installs are made.

## Windows DXVK (Unity)

`deps/dxvk/windows-x64/PROVENANCE.json` records the existing working runtime's
version, upstream download URL and SHA-256 hashes. DLLs stay local, out of Git.
On a fresh checkout, download the recorded release and put its x64 `d3d11.dll`
and `dxgi.dll` in `deps/dxvk/windows-x64/`, then:

```sh
python3 tools/stage_windows_dlls.py
# For the optional root-level ./peekgame copy:
python3 tools/stage_windows_dlls.py --destination windows-dlls
```

Staging verifies checksums. Alternatively, set `PEEKGAME_WINDOWS_DLLS` to the
absolute local runtime directory. The Native source snapshot and retained
Windows DLLs are separate inputs; rebuilding one is not claimed to reproduce
the other. GPU/Vulkan drivers and Linux system libraries remain host dependencies.

## Files excluded from Git

`games/`, `sources/`, installed ILSpy/RetDec, dependency binaries, executable test
fixtures, packaged/relinked games, caches, logs and decompiled output are ignored.
Original source fixtures remain tracked and can be rebuilt locally. Copies of
upstream license notices accompany dependency metadata.
