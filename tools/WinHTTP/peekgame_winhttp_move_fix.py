from pathlib import Path
from difflib import unified_diff

repo = Path('/home/nobus/projects/PeekGame')
changes = {}
for path in ('tools/build_winhttp_probe.py', 'docs/progress/winhttp-probe.md'):
    old = (repo / path).read_text()
    new = old.replace('tools/winhttp_probe.rs', 'tools/WinHTTP/winhttp_probe.rs')
    changes[path] = (old, new)
out = []
for path, (old, new) in changes.items():
    out.append(f'diff --git a/{path} b/{path}\n')
    out.extend(unified_diff(old.splitlines(keepends=True), new.splitlines(keepends=True), fromfile='a/' + path, tofile='b/' + path))
Path('/tmp/peekgame-winhttp-move-fix.patch').write_text(''.join(out))
