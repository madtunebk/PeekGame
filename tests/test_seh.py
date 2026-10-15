"""Windows x64 structured exception handling from Windows code."""
import os
from pathlib import Path
import shutil
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/f'target/seh-{os.getpid()}'
work.mkdir(parents=True)
try:
    exe=work/'seh.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
        str(root/'tests/seh.c'),'-lkernel32','-o',str(exe)],check=True)
    result=subprocess.run([str(root/'target/release/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],
        capture_output=True,text=True,timeout=20,cwd=work)
    assert result.returncode==0,f'seh.exe exit {result.returncode}\n{result.stderr}'
    print('PASS: SEH: vectored handlers, hardware fault resumed, frame handler via .pdata with RtlUnwindEx,'
          ' run-time function tables, unhandled-exception filter')
finally:
    shutil.rmtree(work,ignore_errors=True)
