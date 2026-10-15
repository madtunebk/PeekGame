"""PE TLS callbacks/template isolation plus real synchronization and thread handles."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
exe=root/'target/tls_threads.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry','-Wl,--disable-dynamicbase',str(root/'tests/tls_threads.c'),'-lkernel32','-o',str(exe)],check=True)
result=subprocess.run([str(root/'target/release/peekgame'),str(exe),'--output',str(root/'logs/tls-threads'),'--run'],timeout=15)
assert result.returncode==0,f'TLS/thread fixture exit {result.returncode}'
print('PASS: PE process/thread TLS, fresh templates, zero fill, callbacks, dynamic TLS isolation, recursive locks, event waits, duplicated thread handles')
# The same program linked at base 0 cannot sit at its preferred base (Windows keeps
# the first 64 KiB free), so it is mapped elsewhere and its .reloc entries applied.
moved=root/'target/tls_threads_relocated.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry','-Wl,--dynamicbase','-Wl,--image-base=0',str(root/'tests/tls_threads.c'),'-lkernel32','-o',str(moved)],check=True)
result=subprocess.run([str(root/'target/release/peekgame'),str(moved),'--output',str(root/'logs/tls-threads-relocated'),'--run'],capture_output=True,text=True,timeout=15)
assert result.returncode==0,f'relocated fixture exit {result.returncode}: {result.stderr}'
assert 'TLS callback 0x1035' not in result.stderr and 'TLS callback 0x' in result.stderr,result.stderr
print('PASS: relocated PE image: base relocations, TLS template/index/callbacks at the new base')
