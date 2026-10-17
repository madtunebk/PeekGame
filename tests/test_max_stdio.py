"""CRT stream limit must affect real fopen/fclose operations."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/max-stdio'
work.mkdir(parents=True,exist_ok=True)
exe=work/'max_stdio.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/max_stdio.c'),'-lmsvcrt','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=20)
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: CRT stream default/range, real exhaustion, growth, rejected shrink and fclose slot reuse')
