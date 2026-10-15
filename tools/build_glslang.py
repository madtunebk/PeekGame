"""Build the vendored shader compiler entirely inside PeekGame (no SDL)."""
import argparse
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--jobs',type=int,default=min(os.cpu_count() or 2,8))
a=p.parse_args()
build=root/'target/glslang'
prefix=root/'deps/shader-tools'
subprocess.run(['cmake','-S',str(root/'sources/glslang'),'-B',str(build),'-G','Ninja',
    '-DCMAKE_BUILD_TYPE=Release','-DCMAKE_INSTALL_PREFIX='+str(prefix),'-DCMAKE_INSTALL_LIBDIR=lib',
    '-DENABLE_OPT=OFF','-DGLSLANG_TESTS=OFF','-DBUILD_SHARED_LIBS=OFF','-DENABLE_GLSLANG_BINARIES=ON'],check=True)
subprocess.run(['cmake','--build',str(build),'-j',str(a.jobs)],check=True)
subprocess.run(['cmake','--install',str(build)],check=True)
