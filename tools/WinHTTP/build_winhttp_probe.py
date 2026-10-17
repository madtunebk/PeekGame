#!/usr/bin/env python3
"""Build the minimal Windows Rust WinHTTP probe; optionally run it under PeekGame."""
import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / 'cache/winhttp-probe'
EXE = WORK / 'winhttp_probe.exe'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', action='store_true', help='run the PE through dist/peekgame')
    args = parser.parse_args()
    WORK.mkdir(parents=True, exist_ok=True)
    subprocess.run([
        'rustc', '--edition=2021', '--target', 'x86_64-pc-windows-gnu',
        '-C', 'panic=abort', '-C', 'linker=x86_64-w64-mingw32-gcc',
        '-C', 'link-arg=-nostartfiles', '-C', 'link-arg=-Wl,-e,entry',
        str(ROOT / 'tools/WinHTTP/winhttp_probe.rs'), '-o', str(EXE),
    ], check=True)
    print(f'Built {EXE}', flush=True)
    if args.run:
        result = subprocess.run([
            str(ROOT / 'dist/peekgame'), str(EXE), '--output', str(WORK / 'logs'), '--run',
        ], timeout=30)
        print(f'Probe exit: {result.returncode} (0=HTTP 200 and body read; 29=WinHTTP CANNOT_CONNECT)', flush=True)
        return result.returncode
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
