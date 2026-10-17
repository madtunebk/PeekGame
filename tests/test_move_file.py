"""MoveFileW must preserve an existing destination, including dangling symlinks."""
from pathlib import Path
import subprocess,os
root=Path(__file__).resolve().parents[1]
work=root/f'cache/move-file-{os.getpid()}';work.mkdir(parents=True)
(work/'dangling-link').symlink_to('missing-target')
exe=work/'move_file.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/move_file.c'),'-lkernel32','-lgcc','-o',str(exe)],check=True,env={**os.environ,'TMPDIR':str(work)})
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],cwd=work,capture_output=True,text=True,timeout=15)
assert r.returncode==0,(r.returncode,r.stderr)
assert (work/'dangling-link').is_symlink()
assert (work/'absolute-moved').read_text()=='B'
assert (work/'renamed-漢').read_text()=='C'
assert (work/'dir-漢').is_dir()
print('PASS: MoveFileW Unicode/relative/Z: paths, atomic no-replace including symlinks, directories and Win32 errors; MoveFileExW replacement preserved')
