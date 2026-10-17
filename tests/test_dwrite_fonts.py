"""Real fontconfig collection through the Windows DirectWrite COM ABI, also relinked."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/dwrite-fonts'
work.mkdir(parents=True,exist_ok=True)
exe=work/'dwrite_fonts.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/dwrite_fonts.c'),'-ldwrite','-lkernel32','-lgcc','-o',str(exe)],check=True)
runner=str(root/'dist/peekgame')
r=subprocess.run([runner,str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20)
assert r.returncode==0,(r.returncode,r.stderr)
# A fresh output directory avoids overwriting a previous generated ELF.
import tempfile
with tempfile.TemporaryDirectory(prefix='relink-',dir=work) as directory:
 subprocess.run([runner,str(exe),'--relink','--output',directory],check=True,capture_output=True,timeout=120)
 r=subprocess.run([str(Path(directory)/'dwrite_fonts')],capture_output=True,text=True,timeout=20)
 assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: DirectWrite fontconfig collection, family names, font files, number substitution, factory2 fallback interface; PE and relinked ELF')
