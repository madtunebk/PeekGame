"""AppUserModelID reaches the actual X11 WM_CLASS on a private display."""
from pathlib import Path
import os,subprocess,time
from private_x11 import display
root=Path(__file__).resolve().parents[1]
work=root/'cache/app-identity'
work.mkdir(parents=True,exist_ok=True)
exe=work/'app_identity.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-finput-charset=UTF-8','-Wl,-e,entry',str(root/'tests/app_identity.c'),'-lshell32','-lole32','-luser32','-lkernel32','-o',str(exe)],check=True)
with display() as d:
 env={**os.environ,'DISPLAY':d}
 p=subprocess.Popen([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 try:
  window=None
  for _ in range(40):
   r=subprocess.run(['xdotool','search','--name','^PeekGame identity test$'],env=env,capture_output=True,text=True)
   if r.returncode==0: window=r.stdout.splitlines()[0];break
   if p.poll() is not None: break
   time.sleep(.05)
  assert window, p.communicate(timeout=6)
  r=subprocess.run(['xprop','-id',window,'WM_CLASS'],env=env,capture_output=True,text=True,check=True)
  assert r.stdout.count('PeekGame.Test.Unicodeé')==2 or r.stdout.count(r'PeekGame.Test.Unicode\303\251')==2,r.stdout
  _,stderr=p.communicate(timeout=7)
  assert p.returncode==0,stderr
 finally:
  if p.poll() is None:p.kill();p.communicate()
print('PASS: AppUserModelID validation and actual UTF-8 X11 WM_CLASS grouping identity')
