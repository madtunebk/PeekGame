"""WinHTTP without a transport: handle types, options, timeouts, sync CANNOT_CONNECT and async REQUEST_ERROR/HANDLE_CLOSING callbacks."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/winhttp-offline';work.mkdir(parents=True,exist_ok=True)
exe=work/'winhttp_offline.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/winhttp_offline.c'),'-lwinhttp','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20)
assert r.returncode==0,f'winhttp_offline.exe failed at step {r.returncode}\n{r.stderr}'
print('PASS: WinHTTP handle types, options/timeouts, sync CANNOT_CONNECT and response-state errors,'
      ' async HANDLE_CREATED, REQUEST_ERROR with context on the callback thread, HANDLE_CLOSING')
