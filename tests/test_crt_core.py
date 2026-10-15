"""kernel32 core used by MSVC CRT startup, exercised from Windows code across threads."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
exe=root/'target/crt_core.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/crt_core.c'),'-lkernel32','-lsynchronization','-o',str(exe)],check=True)
result=subprocess.run([str(root/'target/release/peekgame'),str(exe),'--output',str(root/'logs/crt-core'),'--run'],
    capture_output=True,text=True,timeout=20)
assert result.returncode==0,f'crt_core.exe exit {result.returncode}\n{result.stderr}'
print('PASS: heap, SList, FLS with thread-exit callbacks, SRW and condition variables, InitOnce,'
      ' WaitOnAddress, pointer encoding, time, environment, locale and strings')
