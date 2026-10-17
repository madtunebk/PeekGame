"""Walk real PE unwind metadata without requiring frame pointers, also as ELF."""
import os, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
work=root/"cache"/("stack-backtrace-"+str(os.getpid()));work.mkdir(parents=True)
env={**os.environ,"TMPDIR":str(work)}
exe=work/"stack_backtrace.exe"
subprocess.run(["x86_64-w64-mingw32-gcc","-O2","-fomit-frame-pointer","-fno-optimize-sibling-calls","-Wall","-Wextra","-Werror","-nostdlib","-nostartfiles","-fno-builtin","-Wl,-e,entry",str(root/"tests/stack_backtrace.c"),"-lkernel32","-o",str(exe)],check=True,env=env)
runner=root/"dist/peekgame"
subprocess.run([str(runner),str(exe),"--output",str(work/"reports"),"--run"],check=True,timeout=10)
subprocess.run([str(runner),str(exe),"--relink","--output",str(work/"relinked")],check=True,timeout=120,env=env)
subprocess.run([str(work/"relinked/stack_backtrace")],check=True,timeout=10)
print("PASS: RtlCaptureStackBackTrace PE/ELF, frame order, skip/count/hash, no frame pointers and worker stack")
