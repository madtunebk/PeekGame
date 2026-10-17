"""Audit regressions: desktop rects, TECHNOLOGY, MoveFileEx (no folder creation, COPY_ALLOWED only across filesystems), LoadIcon, MessageBox without display."""
from pathlib import Path
import os,subprocess,sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from private_x11 import display as private_display
root=Path(__file__).resolve().parents[1]
work=root/'cache/audit-fixes';work.mkdir(parents=True,exist_ok=True)
exe=work/'audit_fixes.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/audit_fixes.c'),'-luser32','-lgdi32','-lkernel32','-o',str(exe)],check=True)
cross='1' if os.stat('/tmp').st_dev!=os.stat('/dev/shm').st_dev else '0'
def run(env):
    r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],env=env,capture_output=True,text=True,timeout=30)
    assert r.returncode==0,f'audit_fixes.exe failed at step {r.returncode}\n{r.stderr}'
with private_display() as display:
    run(dict(os.environ,DISPLAY=display,PEEK_AUDIT_CROSS=cross))
env={k:v for k,v in os.environ.items() if k not in ('DISPLAY','WAYLAND_DISPLAY')}
run(dict(env,PEEK_AUDIT_MODE='nodisplay'))
print('PASS: desktop window = primary monitor, TECHNOLOGY, MoveFileEx semantics'+(' incl. cross-filesystem' if cross=='1' else '')+', LoadIcon names, MessageBox fails without a display')
