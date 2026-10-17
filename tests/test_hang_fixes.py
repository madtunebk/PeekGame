"""Threading regressions: abandoned mutex WAIT_ABANDONED (single/multiple), critical section re-init, FlushProcessWriteBuffers, INIT_ONCE on reused memory."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/hang-fixes';work.mkdir(parents=True,exist_ok=True)
exe=work/'hang_fixes.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/hang_fixes.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=30)
assert r.returncode==0,f'hang_fixes.exe failed at step {r.returncode}\n{r.stderr}'
print('PASS: abandoned mutex -> WAIT_ABANDONED (single, multiple +1), critical section re-init, FlushProcessWriteBuffers x4 threads, INIT_ONCE reused memory reruns callback')
