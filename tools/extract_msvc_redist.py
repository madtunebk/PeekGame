"""Extract Microsoft's x64 C++ standard library DLLs (msvcp140*.dll) from a local
VC++ 2015-2022 redistributable installer into windows-dlls/ (local only, gitignored).

PeekGame implements VCRUNTIME140 and the UCRT itself but not the C++ standard
library; programs built with MSVC (Unreal Engine games) import MSVCP140.dll and
expect it installed. Steam keeps the official installer under
Steamworks Shared/_CommonRedist/vcredist/2022. Nothing is downloaded and nothing
is committed: the DLLs stay beside the runner, like the DXVK replacements.

Usage: python3 tools/extract_msvc_redist.py [--with-vcruntime] [path/to/VC_redist.x64.exe]
--with-vcruntime also extracts vcruntime140.dll/vcruntime140_1.dll, replacing
PeekGame's own VCRUNTIME140 (C++ EH with __CxxFrameHandler4, RTTI) for every program.
Needs 7z (p7zip) to unpack the cabinet archives. Then run tools/stage_dist.py.
"""
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
default = Path.home() / '.steam/debian-installation/steamapps/common/Steamworks Shared/_CommonRedist/vcredist/2022/VC_redist.x64.exe'
arguments = [a for a in sys.argv[1:] if a != '--with-vcruntime']
with_vcruntime = '--with-vcruntime' in sys.argv[1:]
prefixes = ('msvcp140', 'vcruntime140') if with_vcruntime else ('msvcp140',)
installer = Path(arguments[0]) if arguments else default
if not installer.is_file():
    raise SystemExit(f'VC++ redistributable not found: {installer}')
if not shutil.which('7z'):
    raise SystemExit('7z is required (p7zip-full)')
destination = root / 'windows-dlls'
destination.mkdir(exist_ok=True)

data = installer.read_bytes()
found = {}
with tempfile.TemporaryDirectory() as work:
    work = Path(work)
    # The Burn bundle's payload: cabinet archives (MSCF), some holding further cabs.
    pending, index = [data], 0
    while pending:
        blob = pending.pop()
        offset = 0
        while (offset := blob.find(b'MSCF\0\0\0\0', offset)) >= 0:
            size = struct.unpack_from('<I', blob, offset + 8)[0]
            if not 0 < size <= len(blob) - offset:
                offset += 4
                continue
            cab = work / f'{index}.cab'
            cab.write_bytes(blob[offset:offset + size])
            out = work / str(index)
            subprocess.run(['7z', 'x', '-y', f'-o{out}', str(cab)], check=True, capture_output=True)
            index += 1
            offset += size
            for member in out.iterdir():
                name = member.name.lower()
                if name.startswith(prefixes) and name.endswith('.dll_amd64'):
                    found[member.name[:-len('_amd64')]] = member.read_bytes()
                elif member.read_bytes()[:4] == b'MSCF':
                    pending.append(member.read_bytes())

if 'msvcp140.dll' not in found:
    raise SystemExit('msvcp140.dll (x64) not found in the installer')
provenance = {
    'description': 'Microsoft C++ standard library (x64) extracted from the local VC++ redistributable; not committed',
    'source': str(installer),
    'source_sha256': hashlib.sha256(data).hexdigest(),
    'files': {},
}
for name, content in sorted(found.items()):
    (destination / name).write_bytes(content)
    provenance['files'][name] = {'sha256': hashlib.sha256(content).hexdigest(), 'bytes': len(content)}
    print('extracted', destination / name)
(destination / 'MSVC-REDIST-PROVENANCE.json').write_text(json.dumps(provenance, indent=2) + '\n')
print('Run tools/stage_dist.py to copy them beside dist/peekgame.')
