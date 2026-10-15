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
shutil.copy2(runner,destination/'peekgame')
# Useful for the separate package/ABI workflow; the runner itself links the Rust runtime.
imports=root/'target/release/libgame_imports.so'
if imports.is_file():shutil.copy2(imports,destination/'libgame_imports.so')
native=root/'target/dxvk-x11/prefix/lib'
if native.is_dir():
    libraries=destination/'lib';libraries.mkdir(exist_ok=True)
    for source in native.glob('libdxvk_*.so*'):
        if source.is_file():shutil.copy2(source,libraries/source.name)
print('Distribution ready:',destination/'peekgame')
