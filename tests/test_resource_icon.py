"""Resource icon decode/lifetime and actual native X11 ARGB icon pixels."""
from pathlib import Path
import os,subprocess,zlib,struct,binascii,time,re
from private_x11 import display
root=Path(__file__).resolve().parents[1]
work=root/'cache/resource-icon';work.mkdir(parents=True,exist_ok=True)
def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',binascii.crc32(kind+data)&0xffffffff)
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b'\0\x11\x22\x33\xff'))+chunk(b'IEND',b'')
(work/'icon_png.h').write_text('static unsigned char icon_png[]={'+','.join(map(str,png))+'};')
exe=work/'resource_icon.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-I',str(work),'-Wl,-e,entry',str(root/'tests/resource_icon.c'),'-luser32','-lkernel32','-o',str(exe)],check=True)
with display() as d:
 env={**os.environ,'DISPLAY':d,'APPDATA':'Z:'+str(work/'appdata').replace('/','\\')}
 p=subprocess.Popen([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 try:
  window=None
  for _ in range(40):
   r=subprocess.run(['xdotool','search','--name','^PeekGame resource icon test$'],env=env,capture_output=True,text=True)
   if r.returncode==0:window=r.stdout.splitlines()[0];break
   if p.poll() is not None:break
   time.sleep(.04)
  assert window,p.communicate(timeout=5)
  r=subprocess.run(['xprop','-id',window,'-f','_NET_WM_ICON','32c','_NET_WM_ICON'],env=env,capture_output=True,text=True)
  if r.returncode!=0:
   _,stderr=p.communicate(timeout=5)
   raise AssertionError((r.stderr,p.returncode,stderr))
  values=list(map(int,re.findall(r'\d+',r.stdout.split('=',1)[1])))
  assert values==[1,1,0xff112233],r.stdout
  _,stderr=p.communicate(timeout=5);assert p.returncode==0,(p.returncode,stderr)
 finally:
  if p.poll() is None:p.kill();p.communicate()
print('PASS: PNG/32-bit DIB resource icons, scaling/lifetime, WM_SETICON and actual X11 ARGB pixels')
