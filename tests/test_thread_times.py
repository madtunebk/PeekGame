"""Real per-thread CPU time, Win64 FILETIME alignment and post-exit retention."""
import os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[1]
work = root / "cache" / ("thread-times-" + str(os.getpid()))
work.mkdir(parents=True)
env = {**os.environ, "TMPDIR": str(work)}
exe = work / "thread_times.exe"
subprocess.run(["x86_64-w64-mingw32-gcc", "-O2", "-Wall", "-Wextra", "-Werror", "-nostdlib", "-nostartfiles", "-fno-builtin", "-Wl,-e,entry", str(root/"tests/thread_times.c"), "-lkernel32", "-o", str(exe)], check=True, env=env)
runner = root / "dist/peekgame"
subprocess.run([str(runner), str(exe), "--output", str(work/"reports"), "--run"], check=True, timeout=10)
subprocess.run([str(runner), str(exe), "--relink", "--output", str(work/"relinked")], check=True, timeout=120, env=env)
subprocess.run([str(work/"relinked/thread_times")], check=True, timeout=10)
print("PASS: GetThreadTimes PE/ELF, real CPU counters, creation/exit, 4-byte alignment, duplicate lifetime and errors")
