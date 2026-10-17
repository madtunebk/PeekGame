"""Relink owned Windows PE fixtures into native ELF executables and run them."""
import os
from pathlib import Path
import shutil
import subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1]
runner=Path(os.environ.get('PEEKGAME_RUNNER',root/'target/release/peekgame'))
work=root/f'target/relink-test-{os.getpid()}'
work.mkdir(parents=True)
def mingw(source,output,*libs):
    subprocess.run(['x86_64-w64-mingw32-gcc','-nostdlib','-nostartfiles','-Wl,-e,entry','-Wl,--disable-dynamicbase',
        str(source),*libs,'-o',str(output)],check=True)
def relink(exe,name,strip=False):
    out=work/name
    cmd=[str(runner),str(exe),'--relink','--output',str(out)]+(['--strip'] if strip else [])
    subprocess.run(cmd,check=True,timeout=120,stdout=subprocess.DEVNULL)
    return out/exe.stem
try:
    # Dynamic lookups still go through the runtime registry; static imports are link-time.
    exe=work/'dynamic_imports.exe'
    mingw(root/'tests/dynamic_imports.c',exe,'-lkernel32')
    elf=relink(exe,'dynamic')
    assert elf.read_bytes()[:4]==b'\x7fELF'
    headers=subprocess.run(['readelf','-lW',str(elf)],check=True,capture_output=True,text=True).stdout
    assert 'EXEC (Executable file)' in headers and ' 0x0000000140000000 ' in headers,headers
    trace=work/'dynamic-imports.tsv'
    subprocess.run([str(elf)],check=True,timeout=10,env=dict(os.environ,PEEKGAME_IMPORT_TRACE=str(trace)))
    text=trace.read_text()
    assert 'kernel32.dll\tGetLastError\timplemented\t1' in text,text
    assert 'kernel32.dll\tPeekGameMissingExport\tnot_found\t1' in text,text
    assert 'peekgamemissingmodule.dll\t<LoadLibrary>\tmodule_not_found\t1' in text,text
    symbols=subprocess.run(['nm',str(elf)],check=True,capture_output=True,text=True).stdout
    assert 'peekgame.kernel32.dll.GetProcAddress' in symbols,'IAT not bound through the linker'
    print('PASS: relinked ELF, PE at preferred base, link-time IAT, runtime dynamic lookup')
    # --strip: no symbol table, but the link-time IAT and the program still work.
    stripped=relink(exe,'stripped',strip=True)
    assert stripped.read_bytes()[:4]==b'\x7fELF'
    nm=subprocess.run(['nm',str(stripped)],capture_output=True,text=True)
    assert 'no symbols' in (nm.stdout+nm.stderr).lower(),'stripped ELF still has a symbol table'
    subprocess.run([str(stripped)],check=True,timeout=10,env=dict(os.environ,PEEKGAME_IMPORT_TRACE=str(work/'stripped.tsv')))
    print('PASS: --strip relinked ELF has no symbols and still runs')
    # An import without an implementation links to a stop stub that names it.
    source=work/'missing.c'
    source.write_text('#include <windows.h>\nvoid entry(void){Beep(440,1);ExitProcess(0);}\n')
    exe=work/'missing.exe'
    mingw(source,exe,'-lkernel32')
    elf=relink(exe,'missing')
    result=subprocess.run([str(elf)],capture_output=True,text=True,timeout=10)
    assert result.returncode==78 and 'STOP: missing Windows API KERNEL32.dll!Beep' in result.stderr,result
    print('PASS: unresolved static import stops with its name')
    elf=relink(root/'tests/testgame/testgame.exe','testgame')
    dxvk=(elf.parent/'runtime/libdxvk_d3d11.so.0').is_file() or (elf.parent/'windows-dlls/d3d11.dll').is_file()
    with display() as d:
        env=dict(os.environ,DISPLAY=d,VK_DRIVER_FILES='/usr/share/vulkan/icd.d/lvp_icd.json',DXVK_LOG_PATH='none')
        result=subprocess.run([str(elf),'--frames','6'],env=env,capture_output=True,text=True,timeout=30)
    text=result.stdout+result.stderr
    assert 'window created' in text and 'WM_SIZE wParam=0' in text,text
    if dxvk:
        assert result.returncode==0 and 'RESULT PASS' in text and 'frames presented: 6' in text,text
        print('PASS: relinked testgame, six Vulkan frames, GPU readback, clean exit')
    else:
        assert 'FAIL D3D11CreateDeviceAndSwapChain' in text,text
        print('PASS: relinked testgame reaches D3D11 (DXVK not built: tools/build_dxvk_x11.py)')
finally:
    shutil.rmtree(work,ignore_errors=True)
