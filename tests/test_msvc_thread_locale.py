"""C-only CRT thread locale policy and thread independence, PE/ELF."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'cache/msvc-thread-locale-{os.getpid()}'
work.mkdir(parents=True)
definition=work/'ucrt.def'
definition.write_text('LIBRARY api-ms-win-crt-locale-l1-1-0.dll\nEXPORTS\n_configthreadlocale\nsetlocale\n_errno\n')
library=work/'ucrt.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
exe=work/'thread_locale.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/msvc_thread_locale.c'),str(library),'-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],check=True,timeout=10)
subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(work/'relinked')],check=True,timeout=120)
subprocess.run([str(work/'relinked/thread_locale')],check=True,timeout=10)
print('PASS: CRT thread locale policy, invalid input and worker independence; PE/ELF')
