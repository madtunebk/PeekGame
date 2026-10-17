"""User UI language mapping and locale environment precedence through Win64 ABI."""
from pathlib import Path
import os,struct,subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/ui-language'
work.mkdir(parents=True,exist_ok=True)
exe=work/'ui_language.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/ui_language.c'),'-lkernel32','-o',str(exe)],check=True)
for all_,messages,lang,expected in [('ro_RO.UTF-8','de_DE.UTF-8','en_US.UTF-8',0x418),('','de_DE.UTF-8','ro_RO.UTF-8',0x407),('','','en_GB.UTF-8',0x809),('C','ro_RO.UTF-8','ro_RO.UTF-8',0x409),('POSIX','','',0x409),('xx_XX.UTF-8','','',0x409)]:
 env={**os.environ,'LC_ALL':all_,'LC_MESSAGES':messages,'LANG':lang}
 r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],env=env,capture_output=True,timeout=10)
 assert r.returncode==0,(r.returncode,r.stderr)
 assert struct.unpack('<H',r.stdout)[0]==expected,(all_,messages,lang,r.stdout)
print('PASS: GetUserDefaultUILanguage Win64 LANGID, host locale precedence, C/unknown fallback, preserved last error')
