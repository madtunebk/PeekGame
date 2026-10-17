"""Actual UCRT API-set IAT binding and shared app-type state as PE and ELF."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'cache/msvc-app-type-{os.getpid()}'
work.mkdir(parents=True)
definition=work/'ucrt.def'
definition.write_text('LIBRARY api-ms-win-crt-runtime-l1-1-0.dll\nEXPORTS\n_set_app_type\n_query_app_type\n')
library=work/'ucrt.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
exe=work/'app_type.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/msvc_app_type.c'),str(library),'-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],check=True,timeout=10)
subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(work/'relinked')],check=True,timeout=120)
subprocess.run([str(work/'relinked/app_type')],check=True,timeout=10)
print('PASS: UCRT API-set app type, legacy shared state, PE and ELF')
