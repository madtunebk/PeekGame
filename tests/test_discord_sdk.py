"""Optional real Windows SDK + controlled IPC peer, never a real Discord account."""
import argparse
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import threading
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--dll',type=Path,required=True)
a=p.parse_args()
work=root/'cache'/f'discord-sdk-test-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
path=work/'discord-ipc-0'
server=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);server.bind(str(path));server.listen(1);server.settimeout(10)
errors=[];handshakes=[]
def receive(connection,length):
    data=b''
    while len(data)<length:
        part=connection.recv(length-len(data))
        if not part:raise RuntimeError('SDK closed before completing handshake')
        data+=part
    return data
def peer():
    try:
        connection,_=server.accept()
        with connection:
            connection.settimeout(5)
            opcode,length=struct.unpack('<II',receive(connection,8))
            assert opcode==0 and length<1024
            handshake=json.loads(receive(connection,length));assert handshake=={'v':1,'client_id':'1'}
            handshakes.append(handshake)
            # Explicit protocol fixture. These are test values, not account data.
            payload=json.dumps({'cmd':'DISPATCH','evt':'READY','data':{'v':1,'config':{'cdn_host':'cdn.discordapp.com','api_endpoint':'//discord.com/api','environment':'production'},'user':{'id':'1','username':'fixture','discriminator':'0000','avatar':None}}}).encode()
            connection.sendall(struct.pack('<II',1,len(payload))+payload)
            while connection.recv(8192):pass
    except Exception as error:errors.append(error)
worker=threading.Thread(target=peer,daemon=True);worker.start()
try:
    result=subprocess.run(['python3',str(root/'tools/probe_discord.py'),'--dll',str(a.dll.resolve())],env={**os.environ,'PEEKGAME_PIPE_DIR':str(work)},capture_output=True,text=True,timeout=25)
    (root/'logs/discord-sdk-test.log').write_text(result.stdout+result.stderr)
    assert result.returncode==0,result.stdout+result.stderr
    assert result.stdout.count('run_callbacks: 0\n')==10,result.stdout
    worker.join(3);assert not worker.is_alive(),'SDK did not close its IPC connection'
    assert len(handshakes)==1 and not errors,errors
    print('PASS: real Windows DiscordCreate/callbacks/destroy with a controlled Unix IPC peer')
finally:
    server.close();path.unlink(missing_ok=True)
