"""CRT UTF-16 chdir shares Win32 cwd and path conversion."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/crt-directory'
work.mkdir(parents=True,exist_ok=True)
exe=work/'crt_directory.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/crt_directory.c'),'-lkernel32','-lmsvcrt','-lgcc','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=15)
assert r.returncode==0,(r.returncode,r.stderr)
assert (work/'directory-é/child').is_dir()
print('PASS: _wchdir Unicode, relative and Z: paths, shared Win32 cwd, invalid path/UTF-16/null errors')
