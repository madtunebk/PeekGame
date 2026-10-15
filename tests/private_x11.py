"""Private display context for native graphics tests; never uses the desktop."""
from contextlib import contextmanager
import os
import select
import subprocess
@contextmanager
def display():
    read,write=os.pipe();server=None
    try:
        server=subprocess.Popen(["Xvfb","-displayfd",str(write),"-screen","0","1024x768x24","-nolisten","tcp"],
            pass_fds=(write,),stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
        os.close(write);write=None
        if not select.select([read],[],[],10)[0]:raise RuntimeError("Xvfb did not start")
        number=os.read(read,64).decode().strip()
        if not number.isdigit():raise RuntimeError("invalid private Xvfb display")
        yield ":"+number
    finally:
        os.close(read)
        if write is not None:os.close(write)
        if server:
            server.terminate()
            try:server.wait(timeout=5)
            except subprocess.TimeoutExpired:server.kill();server.wait()
