"""Relink batch A: GetFileAttributesExA, CopyFileA, SearchPathA, GetVolumeInformationA, global atoms (string/integer/refcount), FileTimeToDosDateTime round trip, GetProcessId, CompareStringA."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/import-batch-a';work.mkdir(parents=True,exist_ok=True)
exe=work/'import_batch_a.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/import_batch_a.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20)
assert r.returncode==0,f'import_batch_a.exe failed at step {r.returncode}\n{r.stderr}'
print('PASS: GetFileAttributesExA, CopyFileA, SearchPathA, GetVolumeInformationA, atoms (string/integer/refcount), DOS date/time round trip, GetProcessId, CompareStringA')
