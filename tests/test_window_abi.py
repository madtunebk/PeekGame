"""Run real X11 windows only on a private Xvfb display, never the desktop."""
import os
from pathlib import Path
import select
import subprocess
root=Path(__file__).resolve().parents[1]
subprocess.run(["cc","-std=c11","-O2","-Wall","-Wextra","-Werror",
    str(root/"tests/window_abi.c"),str(root/"tests/window_events.c"),str(root/"tests/window_input.c"),
    "-lX11","-lXtst","-ldl","-o",str(root/"target/window_abi")],check=True)
command=[str(root/"target/window_abi"),str(root/"target/release/libgame_imports.so")]
subprocess.run(command+["no-display"],env={**os.environ,"DISPLAY":":65534"},check=True,timeout=10)
read,write=os.pipe()
server=None
try:
    server=subprocess.Popen(["Xvfb","-displayfd",str(write),"-screen","0","1024x768x24","-nolisten","tcp"],
        pass_fds=(write,),stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    os.close(write);write=None
    if not select.select([read],[],[],10)[0]:raise RuntimeError("Xvfb did not start")
    display=os.read(read,64).decode().strip()
    if not display.isdigit():raise RuntimeError("invalid Xvfb display")
    subprocess.run(command,env={**os.environ,"DISPLAY":":"+display},check=True,timeout=20)
finally:
    os.close(read)
    if write is not None:os.close(write)
    if server is not None:
        server.terminate()
        try:server.wait(timeout=5)
        except subprocess.TimeoutExpired:server.kill();server.wait()
