from pathlib import Path
import os, subprocess
r=Path(__file__).resolve().parents[1]; w=r/'cache/private-heap'; w.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,TMPDIR=str(w))
exe=w/'private_heap.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(r/'tests/private_heap.c'),'-lkernel32','-o',str(exe)],check=True,env=env)
subprocess.run([str(r/'dist/peekgame'),str(exe),'--output',str(w/'logs'),'--run'],check=True,timeout=20)
print('PASS: private heap ownership, realloc, destroy, process heap and unsupported modes')
