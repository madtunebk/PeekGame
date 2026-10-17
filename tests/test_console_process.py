"""Console Unicode output, redirected handles, standard handles and termination."""
import os, pty, subprocess, tty
from pathlib import Path
root=Path(__file__).resolve().parents[1]
work=root/'cache/console-process'
work.mkdir(parents=True,exist_ok=True)
exe=work/'console_process.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/console_process.c'),'-lkernel32','-o',str(exe)],check=True)
cmd=[str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run']
r=subprocess.run(cmd,capture_output=True,timeout=10)
assert r.returncode==42,(r.returncode,r.stderr)
assert not r.stdout,r.stdout
master,slave=pty.openpty()
try:
 tty.setraw(slave)
 r=subprocess.run(cmd,stdout=slave,stderr=subprocess.PIPE,timeout=10)
 assert r.returncode==42,(r.returncode,r.stderr)
 assert os.read(master,64)==b'\x1b[0;93;44m'+'Aé😀'.encode()+b'\x1b[0m'
finally:
 os.close(slave);os.close(master)
print('PASS: WriteConsoleW PTY Unicode/count and redirection; SetStdHandle round trips/errors; TerminateProcess current/invalid')
