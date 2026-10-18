"""UCRT for PlayFab Party: _mkgmtime64 normalisation/range, _difftime64, fmax, _dtest, __stdio_common_vsnprintf_s truncation contract, __stdio_common_vsscanf translated conversions."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'crt-party-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
exe=work/'crt_party_batch.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-Wno-format','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/crt_party_batch.c'),'-lkernel32','-lucrtbase','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=30)
assert r.returncode==0,f'crt_party_batch.exe failed at line {r.returncode}\n{r.stderr}'
print('PASS: _mkgmtime64, _difftime64, fmax, _dtest, vsnprintf_s (_TRUNCATE/count/ERANGE), vsscanf (hhu/I64d/ld/x/lf/f, secure %s sizes, scansets, input length, wide rejected)')
