"""64-bit descriptor sizes, seek preservation and CRT errors."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/filelength'
work.mkdir(parents=True,exist_ok=True)
with (work/'large.dat').open('wb') as f: f.truncate((1<<32)+321)
(work/'empty.dat').write_bytes(b'')
exe=work/'filelength.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/filelength.c'),'-lmsvcrt','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=15)
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: _filelengthi64 >4GB sparse file, zero length, preserved offset and EBADF')
