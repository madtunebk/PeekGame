"""Stage PeekGame's retained Windows DXVK DLLs beside a local runner."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--destination',type=Path,default=root/'target/release/windows-dlls')
a=p.parse_args()
source=root/'deps/dxvk/windows-x64'
metadata=json.loads((source/'PROVENANCE.json').read_text())
for name in ('d3d11.dll','dxgi.dll'):
    actual=hashlib.sha256((source/name).read_bytes()).hexdigest()
    if actual!=metadata['files'][name]['sha256']:raise SystemExit(f'Checksum mismatch: {source/name}')
a.destination.mkdir(parents=True,exist_ok=True)
for name in ('d3d11.dll','dxgi.dll','DXVK-LICENSE','PROVENANCE.json'):
    shutil.copy2(source/name,a.destination/name)
print('Windows DXVK runtime staged:',a.destination)
