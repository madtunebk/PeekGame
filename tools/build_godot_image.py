"""Build a small Windows engine fixture and verify a native Linux image reference."""
import os
from pathlib import Path
import subprocess
import sys
root=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(root/'tests'))
from private_x11 import display
project=root/'tests/godot_image'
logs=root/'logs/godot-image'
output=root/'cache/godot-image/windows'
logs.mkdir(parents=True,exist_ok=True)
output.mkdir(parents=True,exist_ok=True)
def run(name,args,env=None):
    with (logs/(name+'-console.log')).open('w') as log:
        result=subprocess.run(args,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=120)
    if result.returncode:
        raise RuntimeError(f'{name} failed; see {logs/(name+"-console.log")}')
run('import',['godot','--headless','--path',str(project),'--import','--log-file',str(logs/'import.log')])
run('export',['godot','--headless','--path',str(project),'--export-release','Windows Image','--log-file',str(logs/'export.log')])
subprocess.run([str(root/'target/release/peekgame'),'--analyze',str(output/'godot-image.exe'),'--output',str(root/'reports/godot-image')],cwd=root,check=True)
with display() as d:
    env=dict(os.environ,DISPLAY=d,LIBGL_ALWAYS_SOFTWARE='1',XDG_DATA_HOME=str(root/'cache/godot-image/user'),XDG_CACHE_HOME=str(root/'cache/godot-image/xdg'))
    run('native-reference',['godot','--path',str(project),'--display-driver','x11','--audio-driver','Dummy','--log-file',str(logs/'native-godot.log'),'--','--capture',str(logs/'reference.png')],env)
if 'GODOT_IMAGE_PASS' not in (logs/'native-reference-console.log').read_text():
    raise RuntimeError('native reference did not verify its image')
print('Windows fixture:',output/'godot-image.exe')
print('PASS: native Linux reference pixels; screenshot:',logs/'reference.png')
