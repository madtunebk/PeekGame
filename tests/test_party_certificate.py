"""CRYPT32 for PlayFab Party's DTLS identity: X509_NAME DER, self-signed certificate (checked with the openssl CLI), SHA256 hash, time validity, My/Root collection search by SHA1."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'party-certificate-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
exe=work/'party_certificate.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
    str(root/'tests/party_certificate.c'),'-lkernel32','-lcrypt32','-o',str(exe)],check=True)
r=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],cwd=work,capture_output=True,text=True,timeout=60)
assert r.returncode==0,f'party_certificate.exe failed at line {r.returncode}\n{r.stderr}'
der=work/'selfsigned.der'
text=subprocess.run(['openssl','x509','-inform','der','-in',str(der),'-noout','-subject','-issuer','-dates','-text'],capture_output=True,text=True,check=True).stdout
assert 'subject=CN = PeekGame Party' in text and 'issuer=CN = PeekGame Party' in text,text
assert 'notBefore=Jan  1 00:00:00 2026 GMT' in text and 'notAfter=Jan  1 00:00:00 2027 GMT' in text,text
assert 'sha1WithRSAEncryption' in text and 'Public-Key: (2048 bit)' in text,text
pem=subprocess.run(['openssl','x509','-inform','der','-in',str(der)],capture_output=True,check=True).stdout
(work/'self.pem').write_bytes(pem)
v=subprocess.run(['openssl','verify','-no_check_time','-CAfile',str(work/'self.pem'),str(work/'self.pem')],capture_output=True,text=True)
assert v.returncode==0 and ': OK' in v.stdout,v.stdout+v.stderr
print('PASS: X509_NAME DER (size/MORE_DATA/alloc), self-signed RSA-2048 sha1RSA certificate verifies under openssl, SHA256 hash, time validity, My/Root collection SHA1 find')
