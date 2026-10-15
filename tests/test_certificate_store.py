"""CRYPT32 store ABI, parsed metadata and ownership using an ephemeral CA and host roots."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache'/f'cert-store-{os.getpid()}';work.mkdir(parents=True,exist_ok=True)
subprocess.run(['openssl','req','-x509','-newkey','ec','-pkeyopt','ec_paramgen_curve:P-256','-nodes','-days','1','-subj','/CN=PeekGame fixture','-set_serial','0x1234','-keyout',str(work/'key.pem'),'-out',str(work/'ca.pem')],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
for name,defines in [('fixture',[]),('host',['-DHOST_BUNDLE'])]:
    exe=work/f'{name}.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-Wall','-Wextra','-Werror','-Wno-unused-function','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',*defines,str(root/'tests/certificate_store.c'),'-lkernel32','-lcrypt32','-o',str(exe)],check=True)
    env={**os.environ}
    if name=='fixture':env['SSL_CERT_FILE']=str(work/'ca.pem')
    else:env.pop('SSL_CERT_FILE',None)
    result=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/name),'--run'],env=env,cwd=work,capture_output=True,text=True,timeout=15)
    (root/f'logs/certificate-store-{name}.log').write_text(result.stdout+result.stderr)
    assert result.returncode==0,f'{name} exit {result.returncode}\n{result.stderr}'
    print(f'PASS: {name} ROOT enumeration, CERT_INFO ABI, DER metadata, reference lifetime, memory store and invalid DER')
(work/'key.pem').unlink()
