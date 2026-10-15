"""C/Windows ABI -> Rust COM wrappers -> real DXVK -> Vulkan llvmpipe/X11."""
import os
from pathlib import Path
import subprocess
from private_x11 import display
root=Path(__file__).resolve().parents[1]
headers=root/'target/dxvk-x11/source/include/native'
subprocess.run(['cc','-std=c11','-O2','-Wall','-Wextra','-Werror',
    '-isystem',str(headers/'windows'),'-isystem',str(headers/'directx'),
    str(root/'tests/graphics_abi.c'),'-lX11','-ldl','-o',str(root/'target/graphics_abi')],check=True)
command=[str(root/'target/graphics_abi'),str(root/'target/release/libgame_imports.so')]
with display() as screen:
    env={**os.environ,'DISPLAY':screen,'VK_DRIVER_FILES':'/usr/share/vulkan/icd.d/lvp_icd.json','DXVK_LOG_PATH':'none','DXVK_WSI_DRIVER':'PeekX11'}
    subprocess.run(command+['missing-backend'],env={**env,'PEEKGAME_DXVK_DIR':str(root/'target/nonexistent-backend')},check=True,timeout=20)
    subprocess.run(command,env=env,check=True,timeout=60)
