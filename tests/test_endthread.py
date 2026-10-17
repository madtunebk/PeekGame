"""CRT thread exit signals handles and runs FLS cleanup before join completes."""
import os
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[1]
work = root / "cache" / ("endthread-" + str(os.getpid()))
work.mkdir(parents=True)
env = {**os.environ, "TMPDIR": str(work)}
exe = work / "endthread.exe"
subprocess.run(["x86_64-w64-mingw32-gcc", "-nostdlib", "-nostartfiles",
    "-fno-builtin", "-Wall", "-Wextra", "-Werror", "-Wl,-e,entry",
    str(root / "tests/endthread.c"), "-lkernel32", "-lmsvcrt", "-o", str(exe)], check=True, env=env)
runner = root / "dist/peekgame"
subprocess.run([str(runner), str(exe), "--output", str(work / "reports"), "--run"], check=True, timeout=15)
print("PASS: _beginthreadex/_endthreadex, exit code, signaled join and FLS cleanup")
out = work / "relinked"
subprocess.run([str(runner), str(exe), "--relink", "--output", str(out)], check=True, timeout=120, env=env)
subprocess.run([str(out / exe.stem)], check=True, timeout=15, env=env)
print("PASS: CRT thread cleanup and joins in relinked ELF")
