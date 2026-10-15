"""Verify a relocated ELF package uses its local runtime and graphics libraries."""
import json
import os
from pathlib import Path
import shutil
import subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'package-test-{os.getpid()}'
work.mkdir()
original=work/'initial'
subprocess.run([str(root/'target/release/peekgame'),str(root/'tests/testgame/testgame.exe'),'--package','--output',str(original)],check=True)
package=work/'moved package'
original.rename(package)
manifest=json.loads((package/'manifest.json').read_text())
assert len(manifest['imports'])==20
assert (package/'play').read_bytes()[:4]==b'\x7fELF'
assert (package/'game_imports.so').read_bytes()[:4]==b'\x7fELF'
with display() as d:
    env=dict(os.environ,DISPLAY=d,VK_DRIVER_FILES='/usr/share/vulkan/icd.d/lvp_icd.json')
    # The launcher must override stale paths from a developer shell.
    env['PEEKGAME_DXVK_DIR']='/nonexistent'
    result=subprocess.run([str(package/'play'),'--frames','6'],cwd='/home',env=env,timeout=30)
    log=(package/'logs/run.log').read_text()
    assert result.returncode==0,log
    assert 'RESULT PASS' in log and 'frames presented: 6' in log,log
    assert (package/'logs/dynamic-imports.tsv').exists()
    assert any((package/'logs').glob('*d3d11.log'))
    (package/'game_imports.so').rename(package/'saved-runtime.so')
    result=subprocess.run([str(package/'play'),'--frames','1'],cwd='/home',env=env,timeout=10)
    log=(package/'logs/run.log').read_text()
    assert result.returncode!=0 and 'game_imports.so' in log,log
    (package/'saved-runtime.so').rename(package/'game_imports.so')
    # Preserve a successful final log for inspection.
    result=subprocess.run([str(package/'play'),'--frames','3'],cwd='/home',env=env,timeout=30)
    assert result.returncode==0,(package/'logs/run.log').read_text()
refused=subprocess.run([str(root/'target/release/peekgame'),str(root/'tests/testgame/testgame.exe'),'--package','--output',str(package)],capture_output=True,text=True)
assert refused.returncode!=0
assert 'RESULT PASS' in (package/'logs/run.log').read_text()
print('PASS: relocated package, local shared runtime, real Vulkan frames, logs, missing-library failure, overwrite refusal')
print('Artifacts:',package)
