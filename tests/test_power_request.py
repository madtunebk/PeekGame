"""Power-request ABI and lifetime; unsupported inhibition must report failure."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/power-request'
work.mkdir(parents=True,exist_ok=True)
exe=work/'power_request.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/power_request.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=10)
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: power-request creation, validation, close/use-after-close and explicit unsupported inhibition')
