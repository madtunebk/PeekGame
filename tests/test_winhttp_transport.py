"""Exercise a Rust Windows WinHTTP GET against a private local HTTP server."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os
import subprocess
import threading

root = Path(__file__).resolve().parents[1]
work = root / "cache/winhttp-transport"
work.mkdir(parents=True, exist_ok=True)
exe = work / "winhttp_probe.exe"

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        assert self.path == "/"
        body = b"peekgame-winhttp-transport-ok"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_HEAD(self):
        assert self.path == "/"
        self.server.connection_header = self.headers.get("Connection")
        self.send_response(200)
        self.send_header("Content-Length", "0")
        self.send_header("Content-Type", "application/json")
        self.end_headers()

    def log_message(self, *_args):
        pass

server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    env = os.environ.copy()
    env["WINHTTP_PROBE_PORT"] = str(server.server_port)
    subprocess.run([
        "rustc", "--edition=2021", "--target", "x86_64-pc-windows-gnu",
        "--cfg", "local_http", "-C", "panic=abort",
        "-C", "linker=x86_64-w64-mingw32-gcc",
        "-C", "link-arg=-nostartfiles", "-C", "link-arg=-Wl,-e,entry",
        str(root / "tools/WinHTTP/winhttp_probe.rs"), "-o", str(exe),
    ], check=True, env=env)
    result = subprocess.run([
        str(root / "dist/peekgame"), str(exe), "--output", str(work / "logs"), "--run",
    ], capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, f"probe exited {result.returncode}\n{result.stderr}"
    async_exe = work / "winhttp_async.exe"
    subprocess.run([
        "x86_64-w64-mingw32-gcc", "-O2", "-nostdlib", "-nostartfiles",
        "-fno-builtin", "-Wl,-e,entry", f"-DPORT={server.server_port}",
        str(root / "tests/winhttp_async.c"), "-lwinhttp", "-lkernel32",
        "-o", str(async_exe),
    ], check=True)
    result = subprocess.run([
        str(root / "dist/peekgame"), str(async_exe),
        "--output", str(work / "async-logs"), "--run",
    ], capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, f"async fixture exited {result.returncode}\n{result.stderr}"
    assert server.connection_header == "close", f"Connection header not forwarded: {server.connection_header!r}"
    print("PASS: Rust GET body and async HEAD callbacks, numeric status, Content-Type, and Connection header")
finally:
    server.shutdown()
    server.server_close()
