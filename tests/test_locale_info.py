"""Locale code-page metadata, text/DWORD forms and buffer contracts."""
import os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[1]
work = root / "cache" / ("locale-info-" + str(os.getpid()))
work.mkdir(parents=True)
env = {**os.environ, "TMPDIR": str(work)}
exe = work / "locale_info.exe"
subprocess.run(["x86_64-w64-mingw32-gcc", "-O2", "-Wall", "-Wextra", "-Werror", "-nostdlib", "-nostartfiles", "-fno-builtin", "-Wl,-e,entry", str(root/"tests/locale_info.c"), "-lkernel32", "-o", str(exe)], check=True, env=env)
runner = root / "dist/peekgame"
for locale in ("C", "ro_RO.UTF-8", "ja_JP.UTF-8"):
 subprocess.run([str(runner), str(exe), "--output", str(work/locale), "--run"], check=True, timeout=10, env={**env, "LC_ALL": locale})
subprocess.run([str(runner), str(exe), "--relink", "--output", str(work/"relinked")], check=True, timeout=120, env=env)
subprocess.run([str(work/"relinked/locale_info")], check=True, timeout=10, env={**env,"LC_ALL":"C"})
print("PASS: GetLocaleInfoW/Ex text/numeric code pages, locale differences, sizing/errors, UTF-8 ACP and ELF")
