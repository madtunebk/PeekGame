"""Stage retained Windows DXVK DLLs beside the distribution runner."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--destination',type=Path,help='override the default dist/windows-dlls directory')
a=p.parse_args()
source=root/'deps/dxvk/windows-x64'
metadata=json.loads((source/'PROVENANCE.json').read_text())
for name in ('d3d11.dll','dxgi.dll'):
    actual=hashlib.sha256((source/name).read_bytes()).hexdigest()
    if actual!=metadata['files'][name]['sha256']:raise SystemExit(f'Checksum mismatch: {source/name}')
destinations=([a.destination] if a.destination else
              [root/'dist/windows-dlls'])
for destination in destinations:
    destination.mkdir(parents=True,exist_ok=True)
    for name in ('d3d11.dll','dxgi.dll','DXVK-LICENSE','PROVENANCE.json'):
        shutil.copy2(source/name,destination/name)
    # Optional user-supplied PE dependencies stay local and are never taken from target/.
    local=root/'windows-dlls'
    if local.is_dir():
        for library in local.glob('*.dll'):
            if library.name.lower() not in ('d3d11.dll','dxgi.dll'):
                shutil.copy2(library,destination/library.name)
    print('Windows DXVK runtime staged:',destination)
