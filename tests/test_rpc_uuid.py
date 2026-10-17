"""Validate Windows UUID ABI and error behavior through a small PE fixture."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
work = root / "cache/rpc-uuid"
work.mkdir(parents=True, exist_ok=True)
exe = work / "rpc_uuid.exe"
subprocess.run([
    "x86_64-w64-mingw32-gcc", "-O2", "-nostdlib", "-nostartfiles", "-fno-builtin",
    "-Wl,-e,entry", str(root / "tests/rpc_uuid.c"), "-lrpcrt4", "-lkernel32",
    "-o", str(exe),
], check=True)
result = subprocess.run([
    str(root / "dist/peekgame"), str(exe), "--output", str(work / "logs"), "--run",
], capture_output=True, text=True, timeout=20)
assert result.returncode == 0, f"UUID fixture exited {result.returncode}\n{result.stderr}"
print("PASS: RPCRT4 UuidFromStringA layout, invalid input and nil UUID")
