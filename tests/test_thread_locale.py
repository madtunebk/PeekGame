"""Per-thread LCID state, creation-time inheritance and x64 TEB agreement."""
import os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[1]
work = root / "cache" / ("thread-locale-" + str(os.getpid()))
work.mkdir(parents=True)
env = {**os.environ, "TMPDIR": str(work)}
exe = work / "thread_locale.exe"
subprocess.run(["x86_64-w64-mingw32-gcc", "-O2", "-Wall", "-Wextra", "-Werror", "-nostdlib", "-nostartfiles", "-fno-builtin", "-Wl,-e,entry", str(root/"tests/thread_locale.c"), "-lkernel32", "-o", str(exe)], check=True, env=env)
runner = root / "dist/peekgame"
for locale in ("C", "ro_RO.UTF-8"):
 subprocess.run([str(runner), str(exe), "--output", str(work/locale), "--run"], check=True, timeout=10, env={**env, "LC_ALL":locale})
subprocess.run([str(runner), str(exe), "--relink", "--output", str(work/"relinked")], check=True, timeout=120, env=env)
subprocess.run([str(work/"relinked/thread_locale")], check=True, timeout=10, env={**env,"LC_ALL":"C"})
print("PASS: thread locale PE/ELF, host default, independent LCIDs, suspended/nested inheritance, TEB and errors")
