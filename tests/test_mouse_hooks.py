"""WH_MOUSE retrieval, Win64 callbacks, chaining, suppression and unhook lifetime."""
from pathlib import Path
import os,subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1]
work=root/'cache/mouse-hooks'
work.mkdir(parents=True,exist_ok=True)
exe=work/'mouse_hooks.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/mouse_hooks.c'),'-luser32','-lkernel32','-o',str(exe)],check=True)
with display() as d:
 r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],env={**os.environ,'DISPLAY':d},capture_output=True,text=True,timeout=15)
 assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: WH_MOUSE A/W, HC_ACTION/HC_NOREMOVE, newest-first chaining, suppression and unhook')
