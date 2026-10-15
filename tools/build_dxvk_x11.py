"""Build an isolated DXVK Native snapshot with PeekGame's X11 WSI. No network."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
p=argparse.ArgumentParser()
p.add_argument('--peek-root',type=Path,default=Path(__file__).resolve().parents[2]/'sdl3-window')
p.add_argument('--jobs',type=int,default=min(os.cpu_count() or 2,8))
args=p.parse_args()
root=Path(__file__).resolve().parents[1]
upstream=args.peek_root/'deps/dxvk_host/upstream/dxvk'
work=root/'target/dxvk-x11'
source=work/'source'
build=work/'build'
prefix=work/'prefix'
if not source.exists():
    shutil.copytree(upstream,source,ignore=shutil.ignore_patterns('.git'))
    (work/'source-revision.txt').write_text(subprocess.check_output(['git','-C',str(upstream),'rev-parse','HEAD'],text=True))
    (work/'source-local-changes.patch').write_text(subprocess.check_output(['git','-C',str(upstream),'diff'],text=True))
    path=source/'meson.build';text=path.read_text()
    text=text.replace("  if (not lib_sdl3.found() and not lib_sdl2.found() and not lib_glfw.found())", "  lib_x11 = dependency('x11')\n  lib_xrandr = dependency('xrandr')\n  compiler_args += ['-DDXVK_WSI_PEEK_X11']\n  if false")
    path.write_text(text)
    path=source/'src/wsi/meson.build';text=path.read_text().replace("  'wsi_edid.cpp',", "  'wsi_edid.cpp',\n  'peek_x11/platform.cpp',\n  'peek_x11/monitors.cpp',\n  'peek_x11/window.cpp',")
    text=text.replace('wsi_deps = [ dep_displayinfo ]','wsi_deps = [ dep_displayinfo, lib_x11, lib_xrandr ]');path.write_text(text)
    path=source/'src/wsi/wsi_platform.cpp';text=path.read_text().replace('  WsiDriver', '  WsiDriver',1)
    text=text.replace('  const WsiBootstrap *wsiBootstrap[] = {','  extern WsiBootstrap PeekX11WSI;\n  const WsiBootstrap *wsiBootstrap[] = {\n    &PeekX11WSI,');path.write_text(text)
    path=source/'src/wsi/wsi_window.h';text=path.read_text().replace('  struct DxvkWindowState {','  struct DxvkWindowState {\n    struct {uint32_t width=0,height=0;} peek;');path.write_text(text)
path=source/'src/wsi/wsi_platform.cpp'
text=path.read_text()
if '&PeekX11WSI,' not in text:
    text=text.replace('  static const WsiBootstrap *wsiBootstrap[] = {', '  extern WsiBootstrap PeekX11WSI;\n  static const WsiBootstrap *wsiBootstrap[] = {\n    &PeekX11WSI,')
    if '&PeekX11WSI,' not in text:
        raise RuntimeError('DXVK WSI bootstrap declaration changed')
    path.write_text(text)
shutil.copytree(root/'native/dxvk_x11',source/'src/wsi/peek_x11',dirs_exist_ok=True)
env=dict(os.environ)
env['PATH']=str(args.peek_root/'deps/dxvk_host/prefix/bin')+os.pathsep+env['PATH']
if not (build/'build.ninja').exists():
    subprocess.run(['meson','setup',str(build),str(source),'--buildtype=release','--wrap-mode=nodownload',
        '--prefix='+str(prefix),'--libdir=lib','-Dnative_sdl3=disabled','-Dnative_sdl2=disabled','-Dnative_glfw=disabled',
        '-Denable_d3d8=false','-Denable_d3d9=false','-Denable_d3d10=false','-Denable_d3d11=true','-Denable_dxgi=true'],env=env,check=True)
subprocess.run(['ninja','-C',str(build),'-j',str(args.jobs)],env=env,check=True)
subprocess.run(['meson','install','-C',str(build),'--quiet'],env=env,check=True)
print('DXVK X11 backend installed:',prefix/'lib')
