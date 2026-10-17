"""Install the peek native loader as a self-contained Steam Play compatibility tool.

The Steam side must not depend on this source checkout (which can move, rebuild or be
cleaned), so this copies the compat-tool scripts plus the staged loader and its
Windows DLLs into a standalone directory under Steam's compatibilitytools.d. Steam
then shows "peek native loader (experimental)" and runs the installed copy.

Run `python3 tools/stage_dist.py` first (it builds dist/peekgame + dist/windows-dlls).
Usage: python3 tools/install_compat_tool.py [--steam DIR] [--uninstall]
"""
import argparse
import os
import shutil
from pathlib import Path

root = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--steam', type=Path, help='Steam root (default: autodetect)')
p.add_argument('--uninstall', action='store_true', help='remove the installed tool')
a = p.parse_args()

def steam_roots():
    if a.steam:
        return [a.steam]
    found = []
    for c in ['~/.steam/root', '~/.steam/steam', '~/.steam/debian-installation',
              '~/.local/share/Steam']:
        d = Path(c).expanduser()
        if (d / 'compatibilitytools.d').is_dir() or d.is_dir():
            found.append(d)
    return found or [Path('~/.steam/root').expanduser()]

name = 'peek_native'
for steam in steam_roots():
    target = steam / 'compatibilitytools.d' / name
    # A stale symlink (older installs pointed into the source tree) or old dir goes first.
    if target.is_symlink() or target.exists():
        if target.is_symlink():
            target.unlink()
        else:
            shutil.rmtree(target)
    if a.uninstall:
        print('removed', target)
        continue
    binary = root / 'dist/peekgame'
    if not binary.is_file():
        raise SystemExit('dist/peekgame missing; run python3 tools/stage_dist.py first')
    target.mkdir(parents=True)
    src = root / 'tools/peek_compat_tool'
    for f in ['peek-launch', 'compatibilitytool.vdf', 'toolmanifest.vdf', 'README.md']:
        shutil.copy2(src / f, target / f)
    (target / 'peek-launch').chmod(0o755)
    shutil.copy2(binary, target / 'peekgame')
    (target / 'peekgame').chmod(0o755)
    dlls = root / 'dist/windows-dlls'
    if dlls.is_dir():
        shutil.copytree(dlls, target / 'windows-dlls')
    print('installed', target)
    print('  loader:', target / 'peekgame', '(self-contained; set PEEK_BIN to override)')
