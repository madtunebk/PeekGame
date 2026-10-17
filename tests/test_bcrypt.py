"""bcrypt.dll hashes, HMAC and AES-CBC/ECB/GCM checked against hashlib/hmac and the openssl CLI."""
from pathlib import Path
import subprocess,struct,hashlib,hmac
root=Path(__file__).resolve().parents[1]
work=root/'cache/bcrypt';work.mkdir(parents=True,exist_ok=True)
exe=work/'bcrypt.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/bcrypt.c'),'-lbcrypt','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,timeout=15)
assert r.returncode==0,(r.returncode,r.stderr)
data=r.stdout;records=[]
while data:
    status,n=struct.unpack('<II',data[:8]);records.append((status,data[8:8+n]));data=data[8+n:]
it=iter(records)
def ok(expected=None):
    status,value=next(it)
    assert status==0,hex(status)
    if expected is not None:assert value==expected,(value.hex(),expected.hex())
    return value
def fails(code):
    status,_=next(it);assert status==code,(hex(status),hex(code))
def openssl(mode,data,iv=None,pad=True,decrypt=False):
    command=['openssl','enc','-'+mode,'-K','2b7e151628aed2a6abf7158809cf4f3c']
    if iv is not None:command+=['-iv',iv.hex()]
    if not pad:command.append('-nopad')
    if decrypt:command.append('-d')
    return subprocess.run(command,input=data,capture_output=True,check=True).stdout
fox=b'The quick brown fox jumps over the lazy dog'
ok();ok(struct.pack('<I',32));ok()
ok(hashlib.sha256(b'abc').digest());ok(hashlib.sha256(b'a').digest());fails(0xC000000D)
ok()
mac=hmac.new(b'key',fox,'sha256').digest();ok(mac);ok(mac)
ok(hmac.new(bytes(range(100)),fox,'sha256').digest())
plain=bytes((i*7+1)&255 for i in range(48));iv=bytes(range(16))
ok();ok();ok()
ok(struct.pack('<I',32))
cipher=ok(openssl('aes-128-cbc',plain[:20],iv))
ok(cipher[-16:])
ok(plain[:20])
whole=openssl('aes-128-cbc',plain,iv,pad=False)
ok(whole[:16]);ok(whole[16:])
fails(0xC0000206);fails(0xC0000023)
status,_=next(it);assert status==0,'bad padding was accepted'
ok();ok(openssl('aes-128-ecb',plain[:32],pad=False))
ok();sealed=ok();tag=ok()
assert len(sealed)==24 and len(tag)==16
ok(bytes(range(24)));fails(0xC000A002)
fails(0xC0000225);fails(0xC00000BB)
assert next(it,None) is None
print('PASS: SHA-256 and duplicate, HMAC-SHA256 (reuse, long key), AES-CBC padding/IV chaining/in-place,'
      ' ECB, GCM tag check, buffer/size errors, unknown algorithm and property refusals')
