"""Clipboard formats share stable process-local atom IDs with window messages."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/clipboard-formats';work.mkdir(parents=True,exist_ok=True)
exe=work/'clipboard_formats.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/clipboard_formats.c'),'-luser32','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=10)
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: clipboard A/W and window-message atom identity, ASCII case-insensitivity, Unicode and invalid names')
