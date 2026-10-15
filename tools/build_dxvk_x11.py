"""Build vendored DXVK Native with PeekGame X11 WSI; no SDL or network."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--jobs',type=int,default=min(os.cpu_count() or 2,8))
p.add_argument('--build-root',type=Path,default=root/'target/dxvk-x11')
a=p.parse_args()
upstream=root/'sources/dxvk'
work=a.build_root.resolve()
source=work/'source'
build=work/'build'
prefix=work/'prefix'
if not (upstream/'meson.build').is_file():
    raise SystemExit('Missing vendored sources/dxvk; see deps/README.md')
compiler=root/'deps/shader-tools/bin/glslangValidator'
if not compiler.is_file():
    subprocess.run([sys.executable,str(root/'tools/build_glslang.py'),'--jobs',str(a.jobs)],check=True)

def replace(path,old,new):
    text=path.read_text()
    if old not in text:raise RuntimeError(f'Unexpected DXVK source layout: {path.name}')
    path.write_text(text.replace(old,new,1))

if not source.exists():
    shutil.copytree(upstream,source,ignore=shutil.ignore_patterns('.git'))
    metadata=upstream/'PEEKGAME-PROVENANCE.json'
    if metadata.is_file():shutil.copy2(metadata,work/'source-provenance.json')
    replace(source/'meson.build',
        '  if (not lib_sdl3.found() and not lib_sdl2.found() and not lib_glfw.found())',
        "  lib_x11 = dependency('x11')\n  lib_xrandr = dependency('xrandr')\n  compiler_args += ['-DDXVK_WSI_PEEK_X11']\n  if false")
    replace(source/'src/wsi/meson.build',"  'wsi_edid.cpp',",
        "  'wsi_edid.cpp',\n  'peek_x11/platform.cpp',\n  'peek_x11/monitors.cpp',\n  'peek_x11/window.cpp',")
    replace(source/'src/wsi/meson.build','wsi_deps = [ dep_displayinfo ]',
        'wsi_deps = [ dep_displayinfo, lib_x11, lib_xrandr ]')
    replace(source/'src/wsi/wsi_platform.cpp','  static const WsiBootstrap *wsiBootstrap[] = {',
        '  extern WsiBootstrap PeekX11WSI;\n  static const WsiBootstrap *wsiBootstrap[] = {\n    &PeekX11WSI,')
    replace(source/'src/wsi/wsi_window.h','  struct DxvkWindowState {',
        '  struct DxvkWindowState {\n    struct {uint32_t width=0,height=0;} peek;')
if '&PeekX11WSI,' not in (source/'src/wsi/wsi_platform.cpp').read_text():
    raise SystemExit('Existing DXVK work tree lacks PeekX11 registration; choose a new --build-root')
shutil.copytree(root/'native/dxvk_x11',source/'src/wsi/peek_x11',dirs_exist_ok=True)
env=dict(os.environ)
env['PATH']=str(compiler.parent)+os.pathsep+env['PATH']
if not (build/'build.ninja').exists():
    subprocess.run(['meson','setup',str(build),str(source),'--buildtype=release','--wrap-mode=nodownload',
        '--prefix='+str(prefix),'--libdir=lib','-Dnative_sdl3=disabled','-Dnative_sdl2=disabled','-Dnative_glfw=disabled',
        '-Denable_d3d8=false','-Denable_d3d9=false','-Denable_d3d10=false','-Denable_d3d11=true','-Denable_dxgi=true'],env=env,check=True)
subprocess.run(['ninja','-C',str(build),'-j',str(a.jobs)],env=env,check=True)
subprocess.run(['meson','install','-C',str(build),'--quiet'],env=env,check=True)
print('DXVK X11 backend installed:',prefix/'lib')
