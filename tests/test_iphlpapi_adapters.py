"""Compile a Windows PE fixture and verify the IP Helper adapter ABI."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
work = root / "cache/iphlpapi-adapters"
work.mkdir(parents=True, exist_ok=True)
exe = work / "iphlpapi_adapters.exe"
subprocess.run([
    "x86_64-w64-mingw32-gcc", "-O2", "-nostdlib", "-nostartfiles",
    "-fno-builtin", "-Wl,-e,entry", str(root / "tests/iphlpapi_adapters.c"),
    "-liphlpapi", "-lkernel32", "-o", str(exe),
], check=True)
result = subprocess.run([
    str(root / "dist/peekgame"), str(exe), "--output", str(work / "logs"), "--run",
], capture_output=True, text=True, timeout=20)
assert result.returncode == 0, f"adapter fixture exited {result.returncode}\n{result.stderr}"
print("PASS: GetAdaptersAddresses sizing and Windows adapter/unicast ABI layout")
