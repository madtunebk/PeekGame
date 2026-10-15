"""Files, searches and kernel synchronization objects from Windows code."""
import os
from pathlib import Path
import shutil
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'target/files-sync-{os.getpid()}'
work.mkdir(parents=True)
try:
    exe=work/'files_sync.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
        str(root/'tests/files_sync.c'),'-lkernel32','-o',str(exe)],check=True)
    result=subprocess.run([str(root/'target/release/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],
        capture_output=True,text=True,timeout=20,cwd=work)
    assert result.returncode==0,f'files_sync.exe exit {result.returncode}\n{result.stderr}'
    print('PASS: files (create/read/write/seek/overlapped/errors), directories, searches, semaphores, mutexes,'
          ' multi-object waits, CREATE_SUSPENDED')
finally:
    shutil.rmtree(work,ignore_errors=True)
