"""UTF-16 Windows wchar_t to UTF-8 with multibyte truncation and surrogate validation."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/wcstombs'
work.mkdir(parents=True,exist_ok=True)
exe=work/'wcstombs.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',str(root/'tests/wcstombs.c'),'-lmsvcrt','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=10)
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: wcstombs size query, UTF-8, surrogate pairs, byte boundaries, NUL and invalid input')
