"""CRYPT32 Base64/PEM results compared independently with Python base64."""
from pathlib import Path
import subprocess,base64,struct
root=Path(__file__).resolve().parents[1]
work=root/'cache/crypt-strings';work.mkdir(parents=True,exist_ok=True)
exe=work/'crypt_strings.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',str(root/'tests/crypt_strings.c'),'-lcrypt32','-lkernel32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],capture_output=True,timeout=15)
assert r.returncode==0,(r.returncode,r.stderr)
rest=r.stdout
for flags in [1,0,0x80000001,0x40000001,0x40000000]:
 for n in [0,1,2,3,48,49]:
  size=struct.unpack('<I',rest[:4])[0];actual=rest[4:4+size];rest=rest[4+size:]
  eol=b'' if flags&0x40000000 else b'\n' if flags&0x80000000 else b'\r\n'
  raw=base64.b64encode(bytes(range(n)))
  expected=b''.join(raw[i:i+64]+eol for i in range(0,len(raw),64))
  if flags&0xffff==0:expected=b'-----BEGIN CERTIFICATE-----'+eol+expected+b'-----END CERTIFICATE-----'+eol
  assert actual==expected,(flags,n,actual,expected)
assert not rest
print('PASS: CryptBinaryToString A/W Base64/PEM, padding, 64-column wrapping, newline flags and buffer errors')
