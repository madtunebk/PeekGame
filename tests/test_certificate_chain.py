"""Offline TLS validation tests: trusted, untrusted, expiry, names, EKU and tampering."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'cert-chain-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
def openssl(*args):subprocess.run(['openssl',*map(str,args)],cwd=work,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
for name in ['root','unrelated']:
    openssl('req','-x509','-newkey','ec','-pkeyopt','ec_paramgen_curve:P-256','-nodes','-days','2','-subj',f'/CN=PeekGame {name}','-keyout',f'{name}.key','-out',f'{name}.pem')
openssl('req','-new','-newkey','ec','-pkeyopt','ec_paramgen_curve:P-256','-nodes','-subj','/CN=fixture.test','-keyout','leaf.key','-out','leaf.csr')
(work/'leaf.ext').write_text('basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:fixture.test,IP:127.0.0.1\n')
openssl('x509','-req','-in','leaf.csr','-CA','root.pem','-CAkey','root.key','-set_serial','123','-days','1','-extfile','leaf.ext','-out','leaf.pem')
openssl('x509','-in','leaf.pem','-outform','DER','-out','leaf.der')
for name,defines,trust in [('trusted',[],'root.pem'),('untrusted',['-DEXPECT_UNTRUSTED'],'unrelated.pem')]:
    exe=work/f'{name}.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Wextra','-Werror','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry','-DCERT_CHAIN_PARA_HAS_EXTRA_FIELDS',*defines,str(root/'tests/certificate_chain.c'),'-lkernel32','-lcrypt32','-o',str(exe)],check=True)
    result=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/name),'--run'],cwd=work,env={**os.environ,'SSL_CERT_FILE':str(work/trust),'SSL_CERT_DIR':str(work/'no-default-roots')},capture_output=True,text=True,timeout=20)
    (root/f'logs/certificate-chain-{name}.log').write_text(result.stdout+result.stderr)
    assert result.returncode==0,f'{name} fixture exit {result.returncode}\n{result.stderr}'
    print(f'PASS: {name} chain verification and Windows policy ABI')
for key in work.glob('*.key'):key.unlink()
