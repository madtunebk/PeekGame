"""Windows ABI: IOCP queues, file completion, wait/close and DLL thread exit."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'target'/f'iocp-{os.getpid()}'
work.mkdir(parents=True,exist_ok=True)
cc=['x86_64-w64-mingw32-gcc','-O2','-Wall','-Wextra','-Werror','-nostdlib','-fno-builtin']
subprocess.run(cc+['-shared','-Wl,-e,DllMain',str(root/'tests/pe_dlls/exit_worker.c'),'-lkernel32','-o',str(work/'exit_worker.dll')],check=True)
subprocess.run(cc+['-nostartfiles','-Wl,-e,entry',str(root/'tests/iocp.c'),'-lkernel32','-o',str(work/'iocp.exe')],check=True)
result=subprocess.run([str(root/'target/release/peekgame'),str(work/'iocp.exe'),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=15)
(root/'logs/iocp-test.log').write_text(result.stdout+result.stderr)
assert result.returncode==0,f'IOCP fixture exit {result.returncode}\n{result.stderr}'
assert 'FreeLibrary: unloaded exit_worker.dll' in result.stderr,result.stderr
print('PASS: IOCP FIFO/batch/key width/timeout/APC/duplicate/file completion/close wake; ExitThread and self-unloading DLL')
