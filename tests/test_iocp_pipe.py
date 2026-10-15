"""Private Unix peer exercises actual Windows named-pipe overlapped IOCP."""
import os
from pathlib import Path
import socket
import subprocess
import threading
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'iocp-pipe-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
server=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);server.bind(str(work/'peek-fixture'));server.listen(1);server.settimeout(10)
errors=[]
def peer():
    try:
        connection,_=server.accept()
        with connection:
            connection.settimeout(5)
            data=b''
            while len(data)<5:data+=connection.recv(5-len(data))
            assert data==b'hello'
            connection.sendall(data)
            # Keep the pipe alive until the caller cancels its second read and closes it.
            while connection.recv(16):pass
    except Exception as error:errors.append(error)
worker=threading.Thread(target=peer,daemon=True);worker.start()
try:
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Wextra','-Werror','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/iocp_pipe.c'),'-lkernel32','-o',str(work/'pipe.exe')],check=True)
    result=subprocess.run([str(root/'dist/peekgame'),str(work/'pipe.exe'),'--output',str(work/'logs'),'--run'],cwd=work,env={**os.environ,'PEEKGAME_PIPE_DIR':str(work)},capture_output=True,text=True,timeout=10)
    (root/'logs/iocp-pipe-test.log').write_text(result.stdout+result.stderr)
    assert result.returncode==0,f'pipe fixture exit {result.returncode}\n{result.stderr}'
    worker.join(3);assert not worker.is_alive(),'peer did not observe pipe close'
    assert not errors,errors
    print('PASS: real Unix pipe transport, pending read/write, IOCP keys, event, GetOverlappedResult, cancellation')
finally:
    server.close();(work/'peek-fixture').unlink(missing_ok=True)
