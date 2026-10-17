"""POST a small body to a private local server through PeekGame WinHTTP."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading

root = Path(__file__).resolve().parents[1]
work = root / "cache/winhttp-post"
work.mkdir(parents=True, exist_ok=True)

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        self.server.seen_path = self.path
        self.server.seen_type = self.headers.get("Content-Type")
        self.server.seen_body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.send_response(201)
        self.send_header("Content-Type", "text/plain")
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"OK")
    def log_message(self, *_args): pass

server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
try:
    for mode in ("inline", "stream", "async"):
        exe = work / f"winhttp_post_{mode}.exe"
        subprocess.run([
            "x86_64-w64-mingw32-gcc", "-O2", "-nostdlib", "-nostartfiles",
            "-fno-builtin", "-Wl,-e,entry", f"-DPORT={server.server_port}",
            *(["-DSTREAM"] if mode == "stream" else []),
            str(root / ("tests/winhttp_post_async.c" if mode == "async" else "tests/winhttp_post.c")),
            "-lwinhttp", "-lkernel32", "-o", str(exe),
        ], check=True)
        result = subprocess.run([
            str(root / "dist/peekgame"), str(exe), "--output", str(work / f"logs-{mode}"), "--run",
        ], capture_output=True, text=True, timeout=20)
        assert result.returncode == 0, f"{mode} POST fixture exited {result.returncode}\n{result.stderr}"
        assert server.seen_path == "/post"
        assert server.seen_type == "application/json"
        assert server.seen_body == b"peek-post-body"
    print("PASS: WinHTTP inline, streaming and async POST bodies, callbacks, response")
finally:
    server.shutdown()
    server.server_close()
