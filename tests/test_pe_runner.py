"""Execute owned Windows PE fixtures, with actual graphics on private Xvfb."""
import os
from pathlib import Path
import subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1]
runner=root/'target/release/peekgame'
fixture=root/'target/dynamic_imports.exe'
subprocess.run(['x86_64-w64-mingw32-gcc','-nostdlib','-nostartfiles','-Wl,-e,entry','-Wl,--disable-dynamicbase',str(root/'tests/dynamic_imports.c'),'-lkernel32','-o',str(fixture)],check=True)
report=root/'logs/dynamic-fixture'
subprocess.run([str(runner),str(fixture),'--output',str(report),'--run'],check=True,timeout=10)
trace=(report/'dynamic-imports.tsv').read_text()
assert 'kernel32.dll\tGetLastError\timplemented\t1' in trace,trace
assert 'kernel32.dll\tPeekGameMissingExport\tnot_found\t1' in trace,trace
assert 'peekgamemissingmodule.dll\t<LoadLibrary>\tmodule_not_found\t1' in trace,trace
print('PASS: real PE dynamic lookup capture and documented missing-export/module errors')
with display() as d:
    env=dict(os.environ,DISPLAY=d,VK_DRIVER_FILES='/usr/share/vulkan/icd.d/lvp_icd.json',DXVK_LOG_PATH='none')
    with (root/'logs/testgame-run.log').open('w') as log:
        result=subprocess.run([str(runner),str(root/'tests/testgame/testgame.exe'),'--run','--frames','6'],env=env,stdout=log,stderr=subprocess.STDOUT,timeout=30)
    text=(root/'logs/testgame-run.log').read_text()
    assert result.returncode==0,text
    assert 'RESULT PASS' in text and 'frames presented: 6' in text,text
    assert 'readback rgba=' in text,text
    trace=(root/'logs/testgame/dynamic-imports.tsv').read_text()
    assert trace=='dll\texport\tstatus\trequests\n',trace
print('PASS: original testgame PE executed, six Vulkan frames, GPU readback, clean exit')
