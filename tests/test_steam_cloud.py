"""Steam bridge: ISteamRemoteStorage014 (cloud saves) and ISteamUserStats011 (achievements) against fake_steamclient.so."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/'cache/steam-cloud-test';work.mkdir(parents=True,exist_ok=True)
native=root/'native/steam_bridge'
fake=work/'fake_steamclient.so'
subprocess.run(['cc','-shared','-fPIC','-O2','-Wall','-Wextra',native/'fake_steamclient.c','-o',fake],check=True)
subprocess.run(['cc','-O2','-Wall','-Wextra','-Werror',native/'steam_bridge.c',native/'test_cloud.c',fake,
    f'-Wl,-rpath,{work}','-ldl','-pthread','-o',work/'test_cloud'],check=True)
r=subprocess.run([work/'test_cloud',fake],capture_output=True,text=True,timeout=30)
assert r.returncode==0,r.stdout+r.stderr
print(r.stdout.strip())
