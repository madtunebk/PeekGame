"""Native Xdnd selection transfer into a real Windows IDropTarget vtable."""
from pathlib import Path
import os,subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1];work=root/f'cache/ole-drop-{os.getpid()}';work.mkdir(parents=True)
compile_env={**os.environ,'TMPDIR':str(work)}
exe=work/'ole_drop.exe';source=work/'xdnd_source'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Wextra','-Werror','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/ole_drop.c'),'-lole32','-lshell32','-luser32','-lkernel32','-lgcc','-o',str(exe)],check=True,env=compile_env)
subprocess.run(['cc','-std=gnu11','-O2','-Wall','-Wextra','-Werror',str(root/'tests/xdnd_source.c'),'-lX11','-o',str(source)],check=True,env=compile_env)
with display() as d:
 env={**os.environ,'DISPLAY':d,'APPDATA':'Z:'+str(work/'appdata').replace('/','\\')}
 for reentrant in (False,True):
  trial={**env}
  if reentrant:trial['PEEKGAME_DROP_DESTROY']='1'
  p=subprocess.Popen([str(root/'dist/peekgame'),str(exe),'--output',str(work/'reports'),'--run'],env=trial,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  try:
   r=subprocess.run([str(source)]+(["--reentrant"] if reentrant else []),env=trial,capture_output=True,text=True,timeout=22)
   if r.returncode:raise AssertionError((r.returncode,r.stdout,r.stderr,p.poll()))
   out,err=p.communicate(timeout=8);assert p.returncode==0,(p.returncode,out,err)
   print(r.stdout.strip());print(err.strip())
  finally:
   if p.poll() is None:p.kill();p.communicate()
print('PASS: real Xdnd selection, UTF-16 IDataObject/CF_HDROP, enter/over/drop/leave, screen coordinates, owner-thread callbacks, COM lifetime, rejection and cleanup')
