"""Filesystem audit batch A: GENERIC_READ create dispositions, OFD byte-range locks, NtCreateFile RootDirectory/options/Information/disposition."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/fs-batch-a';work.mkdir(parents=True,exist_ok=True)
exe=work/'fs_batch_a.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/fs_batch_a.c'),'-lntdll','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20)
assert r.returncode==0,f'fs_batch_a.exe failed at step {r.returncode}\n{r.stderr}'
print('PASS: read-only OPEN_ALWAYS/CREATE_NEW/CREATE_ALWAYS, TRUNCATE_EXISTING needs write, exclusive/shared/zero-length/duplicated locks, EOF read error, NtCreateFile root-relative created/opened, directory options, bad disposition')
