"""APPDATA must exist before game code calls known-folder APIs; overrides persist."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/early-appdata';work.mkdir(parents=True,exist_ok=True)
exe=work/'early_appdata.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/early_appdata.c'),'-lshell32','-lkernel32','-lgcc','-o',str(exe)],check=True)
for custom in [None,work/'custom-é']:
 env={k:v for k,v in os.environ.items() if k.upper()!='APPDATA'}
 env['XDG_DATA_HOME']=str(work/'data')
 expected=custom or work/'data/peekgame'
 windows='Z:'+str(expected).replace('/','\\')
 if custom:env['APPDATA']=windows
 r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],env=env,capture_output=True,timeout=10)
 assert r.returncode==0,(r.returncode,r.stderr)
 assert r.stdout.decode()==windows,(r.stdout,windows)
 assert expected.is_dir()
print('PASS: early APPDATA, created host directory, UTF-16 known-folder consistency and preserved override')
