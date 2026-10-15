"""Reject active code paths/symlinks that depend on the earlier checkout."""
from pathlib import Path
import os
root=Path(__file__).resolve().parents[1]
legacy=root.parent/('sdl3'+'-window')
problems=[]
for folder in ('src','native','tools','tests'):
    for base,dirs,files in os.walk(root/folder,followlinks=False):
        dirs[:]=[d for d in dirs if d not in ('.git','__pycache__','.godot','retdec','ilspy')]
        for name in files:
            path=Path(base)/name
            if path.suffix not in ('.rs','.py','.sh','.cpp','.h','.toml') and name!='Makefile':continue
            if name.endswith('.exe.c'):continue
            text=path.read_text(errors='replace')
            if str(legacy) in text or repr(legacy.name) in text:
                problems.append(f'legacy path in code: {path.relative_to(root)}')
for folder in ('src','native','tools','tests','deps','sources','games','target'):
    for base,dirs,files in os.walk(root/folder,followlinks=False):
        dirs[:]=[d for d in dirs if d!='.git']
        for name in dirs+files:
            path=Path(base)/name
            if path.is_symlink() and path.resolve().is_relative_to(legacy):
                problems.append(f'legacy symlink: {path.relative_to(root)}')
if problems:raise SystemExit('\n'.join(problems))
print('PASS: build/run code and dependency symlinks are independent of the earlier checkout')
