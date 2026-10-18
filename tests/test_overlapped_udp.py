"""Overlapped UDP with OVERLAPPED.hEvent (PlayFab Party QoS/transport pattern): pending and immediate WSARecvFrom/WSASendTo, WSAGetOverlappedResult, truncation, cancel on closesocket."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'overlapped-udp-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
exe=work/'overlapped_udp.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/overlapped_udp.c'),'-lkernel32','-lws2_32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=60)
assert r.returncode==0,f'overlapped_udp.exe failed at line {r.returncode}\n{r.stderr}'
print('PASS: dual-stack overlapped WSARecvFrom pending/immediate, WSASendTo, WSAGetOverlappedResult (incomplete/bytes/flags/v4-mapped source), WSAEMSGSIZE, cancel on closesocket, completion routine rejected; GetAddrInfoExW sync/async event/HOST_NOT_FOUND')
