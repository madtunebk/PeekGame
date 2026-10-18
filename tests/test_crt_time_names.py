"""UCRT _Getdays/_Getmonths/_W_ variants, _Gettnames (__crt_lc_time_data layout) and _Strftime for msvcp140 (C locale)."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'crt-time-names-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
exe=work/'crt_time_names.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/crt_time_names.c'),'-lkernel32','-lucrtbase','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=30)
assert r.returncode==0,f'crt_time_names.exe failed at line {r.returncode}\n{r.stderr}'
print('PASS: _Getdays/_Getmonths (+wide), _Gettnames/_W_Gettnames 704-byte layout and strings, _Strftime')
