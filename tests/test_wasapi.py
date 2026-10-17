"""Actual Windows COM playback into an isolated PulseAudio/PipeWire null sink."""
import os
from pathlib import Path
import shutil
import struct
import subprocess
import time
root=Path(__file__).resolve().parents[1]
work=root/f'cache/wasapi-{os.getpid()}'
work.mkdir(parents=True)
sink=f'peekgame_test_{os.getpid()}'
module=None
try:
    exe=work/'wasapi.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-fno-builtin','-Wl,-e,entry',
        str(root/'tests/wasapi.c'),'-lole32','-luuid','-lkernel32','-o',str(exe)],check=True,env=dict(os.environ,TMPDIR=str(work)))
    module=subprocess.check_output(['pactl','load-module','module-null-sink',f'sink_name={sink}',
        'rate=48000','channels=2'],text=True).strip()
    for mode in [[],['--pcm16'],['--rate-adjust']]:
        capture=work/('pcm.raw' if mode else 'float.raw')
        with capture.open('wb') as output:
            record=subprocess.Popen(['parec',f'--device={sink}.monitor','--format=float32le','--rate=48000','--channels=2','--latency-msec=10'],stdout=output,stderr=subprocess.PIPE)
            try:
                time.sleep(.2)
                result=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run',*mode],
                    env=dict(os.environ,PULSE_SINK=sink),capture_output=True,text=True,timeout=15)
                time.sleep(.2)
            finally:
                record.terminate();record.communicate(timeout=3)
        assert result.returncode==0,(mode,result.returncode,result.stderr)
        data=capture.read_bytes();samples=struct.unpack('<'+'f'*(len(data)//4),data)
        nonzero=[v for v in samples[::2] if abs(v)>.02]
        assert len(nonzero)>20000,(mode,'no sustained rendered samples',len(nonzero))
        assert any(v>.05 for v in nonzero) and any(v<-.05 for v in nonzero),'missing square waveform'
        print('PASS: WASAPI',mode or 'float32','COM identity/properties, event playback, clock, reset, captured host samples')
finally:
    if module:subprocess.run(['pactl','unload-module',module],check=True)
    shutil.rmtree(work,ignore_errors=True)
