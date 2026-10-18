"""FindFirstChangeNotification backed by inotify: directory changes signal the handle, re-arm and close."""
import os
from pathlib import Path
import shutil
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'target/change-notify-{os.getpid()}'
work.mkdir(parents=True)
try:
    exe=work/'change_notify.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
        str(root/'tests/change_notify.c'),'-lkernel32','-o',str(exe)],check=True)
    r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],
        capture_output=True,text=True,timeout=30,cwd=work)
    assert r.returncode==0,f'change_notify.exe failed at line {r.returncode}\n{r.stderr}'
    print('PASS: FindFirstChangeNotification signals on a real directory change, re-arms, closes, rejects a bad handle')
finally:
    shutil.rmtree(work,ignore_errors=True)
