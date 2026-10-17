"""CRT parity groups and named-mutex lifetime, exercised by Win64 programs."""
from pathlib import Path
import subprocess,os,re
root=Path(__file__).resolve().parents[1]
work=root/'cache/crt-port'
work.mkdir(parents=True,exist_ok=True)
names=sorted(set(re.findall(r'crt!\(\s*"([^"]+)"',(root/'src/registry/crt.rs').read_text())))
definition=work/'msvcrt.def'
definition.write_text('LIBRARY msvcrt.dll\nEXPORTS\n'+'\n'.join(names)+'\n')
library=work/'libcrt_fixture.a'
subprocess.run(['x86_64-w64-mingw32-dlltool','-d',str(definition),'-l',str(library)],check=True)
catalog=(root/'tests/crt_legacy_exports.txt').read_text().splitlines()
(work/'crt_export_names.h').write_text('static const char *names[]={'+','.join('"'+n+'"' for n in catalog)+'};')
for name,libs in [('crt_port',[str(library)]),('mutex_open',[]),('crt_export_catalog',[])]:
 exe=work/(name+'.exe')
 subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-fno-builtin','-nostdlib','-nostartfiles','-Wl,-e,entry','-I',str(work),str(root/'tests'/ (name+'.c')), *libs,'-lkernel32','-lgcc','-o',str(exe)],check=True,env={**os.environ,'TMPDIR':str(work)})
 r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=20)
 assert r.returncode==0,(name,r.returncode,r.stderr)
 print('PASS:',name)

# Check the same wider runtime through a generated ELF, including native libm.
import tempfile
with tempfile.TemporaryDirectory(prefix='relink-',dir=work) as directory:
 subprocess.run([str(root/'dist/peekgame'),str(work/'crt_port.exe'),'--relink','--output',directory],check=True,capture_output=True,timeout=120)
 r=subprocess.run([str(Path(directory)/'crt_port')],cwd=work,capture_output=True,text=True,timeout=20)
 assert r.returncode==0,(r.returncode,r.stderr)
print('PASS: CRT port through relinked ELF')
