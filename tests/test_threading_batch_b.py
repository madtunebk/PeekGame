"""Threading audit batch B: CreateEvent security attributes and named events, OpenEvent, timer completion APCs and no thread pile-up when re-armed, critical section ownership fields."""
from pathlib import Path
import os,subprocess,time
root=Path(__file__).resolve().parents[1]
work=root/'cache/threading-batch-b';work.mkdir(parents=True,exist_ok=True)
marker=Path('/tmp/peekgame-batch-b.ready');marker.unlink(missing_ok=True)
exe=work/'threading_batch_b.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/threading_batch_b.c'),'-lkernel32','-o',str(exe)],check=True)
p=subprocess.Popen([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
threads=None
deadline=time.time()+20
while time.time()<deadline and p.poll() is None:
    if marker.exists():
        threads=len(os.listdir(f'/proc/{p.pid}/task'));break
    time.sleep(0.05)
out,err=p.communicate(timeout=30)
marker.unlink(missing_ok=True)
assert p.returncode==0,f'threading_batch_b.exe failed at step {p.returncode}\n{err}'
assert threads is not None and threads<40,f'{threads} threads after re-arming a timer 300 times'
print(f'PASS: CreateEvent with SECURITY_ATTRIBUTES, named/Local events, OpenEvent, name collision with mutex, CreateEventEx names, timer APC alertable-only and periodic, {threads} threads after 300 re-arms, critical section OwningThread/RecursionCount/LockCount, attribute lists, CreateProcess ERROR_NOT_SUPPORTED')
