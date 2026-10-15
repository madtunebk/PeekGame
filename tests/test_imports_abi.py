"""Exercise the built Rust library through a C caller using ms_abi pointers."""
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[1]
subprocess.run([
    "cc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
    str(root / "tests/imports_abi.c"), "-ldl", "-pthread",
    "-o", str(root / "target/imports_abi"),
], check=True)
subprocess.run([
    str(root / "target/imports_abi"),
    str(root / "target/release/libgame_imports.so"),
], check=True)
