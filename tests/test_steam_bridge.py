"""Exercise Windows PE -> integrated Steam bridge -> explicitly selected test backend."""
from pathlib import Path
import os, subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/steam-bridge-test'; work.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,TMPDIR=str(work))
native=root/'native/steam_bridge'
def run(args,**kw): subprocess.run(list(map(str,args)),check=True,env=kw.pop('env',env),timeout=120,**kw)
fake=work/'fake_steamclient.so'
run(['cc','-shared','-fPIC','-O2',native/'fake_steamclient.c','-o',fake])
run(['cc','-O2','-Wall','-Wextra','-Werror',native/'steam_bridge.c',native/'test_bridge.c','-ldl','-pthread','-o',work/'test_bridge'])
run([work/'test_bridge',fake])
exe=work/'steam-bridge.exe'
run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',root/'tests/steam_bridge.c','-lkernel32','-o',exe])
for missing in (False,True):
 e=dict(env,SteamAppId='952060',STEAM_BRIDGE_CLIENT=str(work/'absent.so' if missing else fake),PEEK_STEAM_TEST_MISSING=str(int(missing)))
 run([root/'dist/peekgame',exe,'--output',work/'reports','--run'],env=e)
print('PASS: Steam ABI suite and PE loader integration, including missing client and missing exports')
