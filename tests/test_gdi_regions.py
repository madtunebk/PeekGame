"""Real rectangle region operations, aliases, lifetime and unsupported operations."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/gdi-regions'
work.mkdir(parents=True,exist_ok=True)
exe=work/'gdi_regions.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/gdi_regions.c'),'-lgdi32','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=10)
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: rectangular GDI regions, bounds, AND/COPY aliasing, delete and unsupported OR')
