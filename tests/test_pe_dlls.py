"""PE DLL loader: program DLLs loaded statically and with LoadLibrary, relocated,
with dependencies, static TLS, DllMain notifications, forwarders and FreeLibrary."""
import os
from pathlib import Path
import shutil
import subprocess
root=Path(__file__).resolve().parents[1]
src=root/'tests/pe_dlls'
work=root/f'target/pe-dlls-{os.getpid()}'
work.mkdir(parents=True)
def cc(*args):
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2','-nostdlib','-fno-builtin',*map(str,args)],check=True,cwd=work)
try:
    # dep.dll and plugin.dll share a preferred base: the second one must be relocated.
    dll=['-shared','-Wl,-e,DllMain','-Wl,--image-base=0x180000000']
    cc(*dll,src/'dep.c','-Wl,--out-implib,libdep.a','-o','dep.dll')
    cc(*dll,src/'plugin.c',src/'plugin.def','libdep.a','-lkernel32','-Wl,--out-implib,libplugin.a','-o','plugin.dll')
    cc('-nostartfiles','-Wl,-e,entry',src/'host.c','libplugin.a','-lkernel32','-Wl,--out-implib,libhost.a','-o','host.exe')
    cc(*dll,src/'late.c','libhost.a','-o','late.dll')
    result=subprocess.run([str(root/'target/release/peekgame'),str(work/'host.exe'),'--output',str(work/'logs'),'--run'],
        capture_output=True,text=True,timeout=20)
    assert result.returncode==0,f'host.exe exit {result.returncode}\n{result.stderr}'
    log=result.stderr
    # Dependencies attach first; each DLL is mapped once; late.dll is unloaded.
    assert log.index('dep.dll: DLL_PROCESS_ATTACH')<log.index('plugin.dll: DLL_PROCESS_ATTACH'),log
    assert log.count('LoadLibrary: mapped')==3,log
    assert 'FreeLibrary: unloaded late.dll' in log,log
    print('PASS: PE DLLs: static and dynamic loading, relocation, dependency order, TLS per module and thread,'
          ' thread notifications, exports by name/ordinal/forwarder, API sets, FreeLibrary with DLL_PROCESS_DETACH')
    # The same EXE relinked into a native ELF beside its DLLs: imports from
    # plugin.dll are bound at startup by the PE loader, the rest at link time.
    subprocess.run([str(root/'target/release/peekgame'),str(work/'host.exe'),'--relink','--output',str(work)],
        check=True,timeout=120,stdout=subprocess.DEVNULL)
    result=subprocess.run([str(work/'host')],capture_output=True,text=True,timeout=20)
    assert result.returncode==0,f'relinked host exit {result.returncode}\n{result.stderr}'
    assert result.stderr.index('dep.dll: DLL_PROCESS_ATTACH')<result.stderr.index('plugin.dll: DLL_PROCESS_ATTACH'),result.stderr
    print('PASS: relinked ELF loads its program DLLs at startup and passes the same checks')
finally:
    if not os.environ.get("KEEP"): shutil.rmtree(work,ignore_errors=True)
