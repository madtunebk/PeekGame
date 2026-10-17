"""Win64 CPU-time ABI, optional outputs and host counter agreement."""
import os
from pathlib import Path
import shutil
import struct
import subprocess
root=Path(__file__).resolve().parents[1]
work=root/f"cache/system-times-{os.getpid()}"
work.mkdir(parents=True)
def counters():
    fields=list(map(int,Path('/proc/stat').read_text().splitlines()[0].split()[1:]))
    idle=fields[3]+fields[4]
    return [idle,idle+fields[2]+fields[5]+fields[6],fields[0]+fields[1]]
try:
    exe=work/'system_times.exe'
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-nostartfiles','-Wl,-e,entry',
        str(root/'tests/system_times.c'),'-lkernel32','-o',str(exe)],check=True)
    before=counters()
    result=subprocess.run([str(root/'dist/peekgame'),str(exe),'--output',str(work/'logs'),'--run'],
        capture_output=True,timeout=10)
    after=counters()
    assert result.returncode==0,result.stderr.decode(errors='replace')
    values=struct.unpack('<QQQ',result.stdout)
    hz=os.sysconf('SC_CLK_TCK')
    for low,value,high in zip(before,values,after):
        assert low*10_000_000//hz <= value <= high*10_000_000//hz,(before,values,after)
    print('PASS: GetSystemTimes Win64 ABI, monotonic counters, nullable outputs, host CPU accounting')
finally:
    shutil.rmtree(work)
