"""Compare Windows display APIs to actual XRandR monitor geometry."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
from private_x11 import display
root=Path(__file__).resolve().parents[1]
work=root/f'cache/display-topology-{os.getpid()}'
work.mkdir(parents=True)
try:
    binary=work/'probe'
    subprocess.run(['cc','-std=c11','-O2','-Wall','-Wextra','-Werror',str(root/'tests/display_topology.c'),
        '-lX11','-lXrandr','-ldl','-o',str(binary)],check=True,env=dict(os.environ,TMPDIR=str(work)))
    command=[str(binary),str(root/'dist/libgame_imports.so')]
    if '--host' in sys.argv:
        # Queries only: no window creation, focus, cursor movement or display changes.
        subprocess.run(command,check=True,timeout=10)
    else:
        with display() as screen:
            subprocess.run(command,env=dict(os.environ,DISPLAY=screen),check=True,timeout=10)
finally:shutil.rmtree(work)
