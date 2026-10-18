"""kernel32 for PlayFab Party/msvcp140: thread pool work/timer/wait (+cleanup-group rejection, FreeLibraryWhenCallbackReturns), CreateFile2, CopyFile2, symbolic and hard links, TerminateThread on the caller, WindowsConcatString."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'kernel32-party-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
exe=work/'kernel32_party_batch.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/kernel32_party_batch.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=60)
assert r.returncode==0,f'kernel32_party_batch.exe failed at line {r.returncode}\n{r.stderr}'
assert os.path.islink(work/'link.txt') and os.readlink(work/'link.txt')=='created2.txt'
assert os.stat(work/'hard.txt').st_nlink==2
print('PASS: thread pool work/timer/wait/FreeLibraryWhenCallbackReturns, cleanup group rejected, CreateFile2, CopyFile2, symlink (relative), hard link, TerminateThread(self), WindowsConcatString')
