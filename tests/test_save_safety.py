"""Save safety: rename-over through handles (kernel32 + NT), delete dispositions, DELETE_ON_CLOSE with duplicates, SetFileTime special/pre-1970 values, read-only via FileBasicInfo, NT unknown class is an error."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/save-safety';work.mkdir(parents=True,exist_ok=True)
exe=work/'save_safety.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/save_safety.c'),'-lntdll','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20)
assert r.returncode==0,f'save_safety.exe failed at step {r.returncode}\n{r.stderr}'
print('PASS: atomic rename-over (kernel32 + NT path), no-replace refusal, delete disposition set/cancel, DELETE_ON_CLOSE with duplicate, SetFileTime keep/1965, read-only attribute, NT unknown class error')
