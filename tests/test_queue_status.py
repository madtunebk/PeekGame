"""USER32 GetQueueStatus (CEF in Unreal/Palworld): queued kinds, added-since-last bits, removal, invalid flags."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'queue-status-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
exe=work/'queue_status.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/queue_status.c'),'-lkernel32','-luser32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=30)
assert r.returncode==0,f'queue_status.exe failed at line {r.returncode}\n{r.stderr}'
print('PASS: GetQueueStatus queued/added words, mask, removal, ERROR_INVALID_FLAGS')
