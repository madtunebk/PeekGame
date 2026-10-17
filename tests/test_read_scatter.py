"""ReadFileScatter: shuffled page segments, OVERLAPPED offset, event, IOCP packet, EOF and argument errors."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/read-scatter';work.mkdir(parents=True,exist_ok=True)
exe=work/'read_scatter.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/read_scatter.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=15)
assert r.returncode==0,f'read_scatter.exe failed at step {r.returncode}\n{r.stderr}'
print('PASS: ReadFileScatter page segments at the OVERLAPPED offset, event, GetOverlappedResult, IOCP packet, short read at EOF, argument errors')
