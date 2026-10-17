"""Case-insensitive path resolution (open, attributes, find, create under mis-cased dirs, exact match wins) and device names (NUL, nul.txt, COM1)."""
from pathlib import Path
import shutil,subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/case-paths';work.mkdir(parents=True,exist_ok=True)
tree=Path('/tmp/peekgame-case');shutil.rmtree(tree,ignore_errors=True)
(tree/'Data/Textures').mkdir(parents=True)
(tree/'Data/Textures/Wall.dds').write_text('wall')
(tree/'Both.txt').write_text('upper');(tree/'both.txt').write_text('lower')
exe=work/'case_paths.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/case_paths.c'),'-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,text=True,timeout=20)
assert r.returncode==0,f'case_paths.exe failed at step {r.returncode}\n{r.stderr}'
assert (tree/'Data/Textures/New.txt').read_text()=='new' and not (tree/'DATA').exists(),'new file not created in the existing directory'
assert not (tree/'nul.txt').exists() and not (root/'NUL').exists(),'a NUL device name created a file'
shutil.rmtree(tree)
print('PASS: mis-cased open/attributes/find, create under mis-cased dirs, exact match wins, missing stays not-found, NUL/nul.txt swallow writes, COM1 absent')
