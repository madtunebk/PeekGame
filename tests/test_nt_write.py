"""NT synchronous write data, status, last-error and unsupported arguments."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'cache/nt-write-{os.getpid()}'
work.mkdir(parents=True)
definition=work/'ucrt.def'
definition.write_text('LIBRARY ntdll.dll\nEXPORTS\nNtWriteFile\n')
library=work/'ucrt.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
exe=work/'nt_write.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/nt_write.c'),str(library),'-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],check=True,timeout=10,cwd=work)
subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(work/'relinked')],check=True,timeout=120)
subprocess.run([str(work/'relinked/nt_write')],check=True,timeout=10,cwd=work)
print('PASS: NT synchronous write bytes, errors, current position and stdout; PE/ELF')
