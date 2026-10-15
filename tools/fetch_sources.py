"""Fetch pinned source inputs into PeekGame; no reference to another checkout."""
import json
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
metadata=json.loads((root/'deps/manifest.json').read_text())
for name,info in metadata.items():
    dest=root/'sources'/name
    if dest.exists():
        print('Keeping existing source:',dest)
        continue
    subprocess.run(['git','clone','--no-checkout',info['upstream'],str(dest)],check=True)
    subprocess.run(['git','-C',str(dest),'checkout','--detach',info['revision']],check=True)
    subprocess.run(['git','-C',str(dest),'submodule','update','--init','--recursive'],check=True)
    if info.get('local_changes'):
        subprocess.run(['git','-C',str(dest),'apply',str(root/info['local_changes'])],check=True)
    (dest/'PEEKGAME-PROVENANCE.json').write_text(json.dumps(info,indent=2)+'\n')
    print('Pinned source ready:',dest)
