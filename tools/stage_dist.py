"""Assemble a local distribution; games and generated binaries stay out of Git."""
from pathlib import Path
import shutil
import subprocess
import sys
root=Path(__file__).resolve().parents[1]
runner=root/'target/release/peekgame'
if not runner.is_file():raise SystemExit('Build first: cargo build --release --offline')
subprocess.run([sys.executable,str(root/'tools/stage_windows_dlls.py')],check=True)
destination=root/'dist';destination.mkdir(exist_ok=True)
staged_runner=destination/'peekgame.new'
shutil.copy2(runner,staged_runner)
staged_runner.replace(destination/'peekgame')
# Useful for the separate package/ABI workflow; the runner itself links the Rust runtime.
for name in ('libgame_imports.so', 'libgame_imports.a'):
    imports=root/'target/release'/name
    if not imports.is_file():
        raise SystemExit(f'Missing {imports}; run cargo build --release --offline')
    shutil.copy2(imports,destination/name)
native=root/'target/dxvk-x11/prefix/lib'
if native.is_dir():
    libraries=destination/'lib';libraries.mkdir(exist_ok=True)
    for source in native.glob('libdxvk_*.so*'):
        if source.is_file():shutil.copy2(source,libraries/source.name)
print('Distribution ready:',destination/'peekgame')
