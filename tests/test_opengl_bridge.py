"""Real GL3.3 rendering across Win64 ABI, on a private software GLX display."""
from pathlib import Path
import os, subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1]
work=root/'cache/opengl-bridge';work.mkdir(parents=True,exist_ok=True)
exe=work/'opengl_bridge.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/opengl_bridge.c'),'-lopengl32','-lgdi32','-luser32','-lkernel32','-o',str(exe)],check=True,env={**os.environ,'TMPDIR':str(work)})
with display() as d:
 env={**os.environ,'DISPLAY':d,'LIBGL_ALWAYS_SOFTWARE':'1','APPDATA':'Z:'+str(work/'appdata').replace('/','\\')}
 result=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],env=env,capture_output=True,text=True,timeout=30)
 print(result.stdout);print(result.stderr)
 assert result.returncode==0,result.returncode
 assert 'OPENGL_BRIDGE_PASS' in result.stdout
 out=work/f'relinked-{os.getpid()}'
 subprocess.run([str(root/'dist/peekgame'),str(exe),'--relink','--output',str(out)],check=True,capture_output=True,text=True,timeout=60)
 result=subprocess.run([str(out/exe.stem)],env=env,capture_output=True,text=True,timeout=30)
 assert result.returncode==0,(result.returncode,result.stderr)
 assert 'OPENGL_BRIDGE_PASS' in result.stdout,result.stdout
 print('PASS: same OpenGL shader/readback fixture as relinked ELF')
print('PASS: native GLX, GL3.3 shader rendering/readback, float/double ABI, resize and context lifetime')
