"""Optional locally supplied D3D compiler must produce actual DXBC."""
from pathlib import Path
import subprocess,os,shutil,sys
root=Path(__file__).resolve().parents[1]
work=root/'cache/d3dcompiler'
work.mkdir(parents=True,exist_ok=True)
source=os.environ.get('PEEKGAME_D3DCOMPILER')
if not source:
 print('SKIP: experimental D3D compiler probe; set PEEKGAME_D3DCOMPILER to a local PE DLL')
 sys.exit(0)
libraries=work/'windows-dlls';libraries.mkdir(exist_ok=True)
shutil.copy2(source,libraries/'d3dcompiler_47.dll')
exe=work/'d3dcompiler.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/d3dcompiler.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20,env={**os.environ,'PEEKGAME_WINDOWS_DLLS':str(libraries)})
assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: local d3dcompiler_47.dll D3DCompile produces DXBC through the PE runtime')
