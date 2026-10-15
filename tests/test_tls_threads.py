"""PE TLS callbacks/template isolation plus real synchronization and thread handles."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
exe=root/'target/tls_threads.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry','-Wl,--disable-dynamicbase',str(root/'tests/tls_threads.c'),'-lkernel32','-o',str(exe)],check=True)
result=subprocess.run([str(root/'target/release/peekgame'),str(exe),'--output',str(root/'logs/tls-threads'),'--run'],timeout=15)
assert result.returncode==0,f'TLS/thread fixture exit {result.returncode}'
print('PASS: PE process/thread TLS, fresh templates, zero fill, callbacks, dynamic TLS isolation, recursive locks, event waits, duplicated thread handles')
