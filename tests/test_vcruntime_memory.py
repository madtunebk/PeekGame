"""VCRUNTIME explicit memory exports and module discovery, PE/ELF."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'cache/vcruntime-memory-{os.getpid()}'
work.mkdir(parents=True)
definition=work/'ucrt.def'
definition.write_text('LIBRARY VCRUNTIME140.dll\nEXPORTS\nmemcpy\nmemmove\nmemset\nmemcmp\n')
library=work/'ucrt.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
exe=work/'memory.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-fno-builtin','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/vcruntime_memory.c'),str(library),'-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],check=True,timeout=10)
subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(work/'relinked')],check=True,timeout=120)
subprocess.run([str(work/'relinked/memory')],check=True,timeout=10)
print('PASS: VCRUNTIME memory overlap/comparison and C SEH export discovery; PE/ELF')
