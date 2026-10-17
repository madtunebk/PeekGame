"""Godot UI hit testing after native move/resize, on private Xvfb only.
Requires Godot and its Windows x86_64 release export template; override the
latter with PEEKGAME_GODOT_TEMPLATE to test a different engine executable.
"""
import os,sys,subprocess,time,shutil
from pathlib import Path
root=Path(__file__).resolve().parents[1];sys.path.insert(0,str(root/'tests'))
from private_x11 import display
work=root/f'cache/godot-input-{os.getpid()}'
work.mkdir(parents=True)
project=work/'project'
shutil.copytree(root/'tests/godot_input',project,ignore=shutil.ignore_patterns('.godot'))
env=dict(os.environ,TMPDIR=str(work),XDG_DATA_HOME=str(work/'data'))
with open(work/'export.log','w') as output:
 subprocess.run(['godot','--headless','--path',str(project),'--export-pack','Windows Desktop',str(work/'input.pck')],env=env,stdout=output,stderr=subprocess.STDOUT,check=True,timeout=60)
version=subprocess.check_output(['godot','--version'],text=True).strip().split('.')
version='.'.join(version[:3])+'.'+version[3]
template=Path(os.environ.get('PEEKGAME_GODOT_TEMPLATE',str(Path.home()/'.local/share/godot/export_templates'/version/'windows_release_x86_64.exe')))
shutil.copyfile(template,work/'input.exe')
with display() as d,open(work/'run.log','w') as log:
 env=dict(os.environ,DISPLAY=d,LIBGL_ALWAYS_SOFTWARE='1',LP_NUM_THREADS='2',PEEKGAME_TRACE_INPUT='1',XDG_DATA_HOME=str(work/'data'),TMP=str(work),TEMP=str(work))
 p=subprocess.Popen([str(root/'dist/peekgame'),str(work/'input.exe'),'--output',str(work/'reports'),'--run','--audio-driver','Dummy'],env=env,stdout=log,stderr=subprocess.STDOUT)
 try:
  for _ in range(100):
   if 'INPUT_PROBE_READY' in (work/'run.log').read_text():break
   if p.poll() is not None:raise RuntimeError((work/'run.log').read_text()[-2500:])
   time.sleep(.1)
  else:raise RuntimeError('Godot startup timed out')
  win=subprocess.check_output(['xdotool','search','--name','PeekGame Godot Input Probe'],env=env,text=True).split()[-1]
  for width,height,x,y in [(640,360,23,41),(800,600,117,83),(960,540,11,19)]:
   subprocess.run(['xdotool','windowmove',win,str(x),str(y),'windowsize',win,str(width),str(height),'windowfocus',win],env=env,check=True)
   time.sleep(.3)
   subprocess.run(['xdotool','mousemove','--window',win,str(width//2),str(height//2)],env=env,check=True)
   time.sleep(.2)
   subprocess.run(['xdotool','click','1'],env=env,check=True)
   time.sleep(.3)
 finally:
  p.terminate();p.wait(timeout=5)
text=(work/'run.log').read_text()
assert text.count('INPUT_PROBE_PRESSED')==3, f'Expected three button clicks; inspect {work}/run.log'
for size in ['(640, 360)','(800, 600)','(960, 540)']:
 assert any('position=(320.0, 180.0)' in line and 'window='+size in line for line in text.splitlines()), size
print(f'PASS: Godot button hit testing at three moved/resized window sizes; log {work}/run.log')
