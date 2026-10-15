"""Opt-in real Windows Discord SDK probe; no activity updates or chat messages."""
import argparse
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--dll',type=Path,required=True)
p.add_argument('--header-dir',type=Path,default=root/'cache/discord-sdk/c')
a=p.parse_args()
work=root/'cache/discord-probe';work.mkdir(parents=True,exist_ok=True)
# The official header spells Windows.h in upper case; MinGW uses windows.h.
(work/'Windows.h').write_text('#include <windows.h>\n')
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',f'-I{work}',f'-I{a.header_dir.resolve()}',f'-DDISCORD_DLL=L"{a.dll.resolve()}"',str(root/'tests/discord_probe.c'),'-lkernel32','-o',str(work/'probe.exe')],check=True)
try:
    result=subprocess.run([str(root/'dist/peekgame'),str(work/'probe.exe'),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=20)
    text=result.stdout+result.stderr
    (root/'logs/discord-probe.log').write_text(text)
    print(text[-4000:]);print('Probe exit:',result.returncode)
    if result.returncode:raise SystemExit(result.returncode)
    if 'DiscordCreate result: 0\n' not in text:
        raise SystemExit('Discord SDK returned a failure result; see logs/discord-probe.log')
except subprocess.TimeoutExpired as error:
    (root/'logs/discord-probe.log').write_bytes((error.stdout or b'')+(error.stderr or b''))
    raise SystemExit('Discord probe timed out; see logs/discord-probe.log')
