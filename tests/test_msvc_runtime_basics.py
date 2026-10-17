"""Initial UCRT environment snapshot and typed math calls, PE/ELF."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'cache/msvc-basics-{os.getpid()}'
work.mkdir(parents=True)
os.environ['PEEKGAME_ENV_PROBE']='initial-value'
definition=work/'ucrt.def'
definition.write_text('LIBRARY api-ms-win-crt-stdio-l1-1-0.dll\nEXPORTS\n_get_initial_narrow_environment\ntrunc\nround\ntruncf\nroundf\nexp2\nexp2f\n')
library=work/'ucrt.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
exe=work/'basics.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-fno-builtin','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/msvc_runtime_basics.c'),str(library),'-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],check=True,timeout=10)
subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(work/'relinked')],check=True,timeout=120)
subprocess.run([str(work/'relinked/basics')],check=True,timeout=10)
print('PASS: UCRT initial environment and math adapters, including signed zero; PE/ELF')
