"""UCRT file-mode configuration, errors and shared storage as PE and ELF."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'cache/msvc-fmode-{os.getpid()}'
work.mkdir(parents=True)
definition=work/'ucrt.def'
definition.write_text('LIBRARY api-ms-win-crt-stdio-l1-1-0.dll\nEXPORTS\n_set_fmode\n_get_fmode\n__p__fmode\n__p__commode\n_errno\n')
library=work/'ucrt.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
exe=work/'fmode.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/msvc_fmode.c'),str(library),'-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],check=True,timeout=10)
subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(work/'relinked')],check=True,timeout=120)
subprocess.run([str(work/'relinked/fmode')],check=True,timeout=10)
print('PASS: UCRT fmode state, defaults, pointer access and errors; PE/ELF')
