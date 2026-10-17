# Palworld Launcher: CreateProcess Implementation

## 1. Context & Motivation
Following the resolution of the Visual C++ Runtime prerequisite check, `Palworld.exe` (Unreal Engine's `BootstrapPackagedGame.exe`) successfully executes all setup logic and calls:
```
CreateProcessW(
    lpApplicationName = NULL,
    lpCommandLine = "\"Z:\home\nobus\.steam\debian-installation\steamapps\common\Palworld\Pal\Binaries\Win64\Palworld-Win64-Shipping.exe\" Pal",
    ...,
    lpProcessInformation = &process_info
)
```
PeekGame's stub returned `ERROR_NOT_SUPPORTED` (50 / `0x32`), causing `Palworld.exe` to display:
```
Couldn't start:
"Z:\home\nobus\.steam\debian-installation\steamapps\common\Palworld\Pal\Binaries\Win64\Palworld-Win64-Shipping.exe" Pal 
CreateProcess() returned 32.
```

---

## 2. Palworld Launcher Execution Flow
From disassembling `Palworld.exe` at `0x1400017fb` - `0x1400018d0`:
1. `call CreateProcessW`
2. If `CreateProcessW` succeeds:
   - `WaitForSingleObject(process_info.hProcess, INFINITE)` (`0x14000187f`)
   - `GetExitCodeProcess(process_info.hProcess, &exit_code)` (`0x140001897`)
   - `CloseHandle(process_info.hThread)` (`0x1400018a2`)
   - `CloseHandle(process_info.hProcess)` (`0x1400018ad`)
   - Returns child exit code.
3. If `CreateProcessW` fails:
   - `GetLastError()`
   - `wsprintfW()` formatting error message
   - `MessageBoxW()` displaying error dialog

---

## 3. Architecture for Child Process Execution in PeekGame

To support game launchers like `Palworld.exe`:
1. **Command Line Parsing**:
   Use `crate::host::arguments::split` to extract the executable target and arguments from `lpCommandLine` (or `lpApplicationName`).
2. **Path Resolution**:
   Translate Windows path (e.g. `Z:\home\nobus\...`) to host path using `crate::host::paths::host`.
3. **Spawning via PeekGame**:
   Spawn `std::process::Command::new(std::env::current_exe())` with:
   - Argument: `<resolved_target_exe>`
   - Argument: `--run`
   - Arguments: `<trailing_arguments>`
   - Working Directory: directory of target EXE or `lpCurrentDirectory`.
   - Environment: Inherit host environment (`SteamAppId`, `SteamGameId`, etc.).
4. **Process Lifetime & Synchronization**:
   - Store `ChildProcess` entry mapped to an allocated `hProcess` handle.
   - Support `WaitForSingleObject(hProcess, timeout)` to block or poll child exit.
   - Support `GetExitCodeProcess(hProcess, &code)` returning `STILL_ACTIVE` (259) or the exit status.
   - Support `CloseHandle` for process and thread handles.

---

## 4. Verification & Results

Running `Palworld.exe`:
```
SteamAppId=1623730 SteamGameId=1623730 ./dist/peekgame ~/.steam/debian-installation/steamapps/common/Palworld/Palworld.exe --run
```

Output:
```
PeekGame: entering Palworld.exe at 0x140001ca8
LoadLibrary: mapped msvcp140_2.dll, msvcp140.dll, vcruntime140.dll, vcruntime140_1.dll
CreateProcess: spawning "/home/nobus/.steam/debian-installation/steamapps/common/Palworld/Pal/Binaries/Win64/Palworld-Win64-Shipping.exe" with args ["Pal"]
LoadLibrary: mapped tbb.dll, dxgi.dll, d3d11.dll, msvcp140.dll, vcruntime140.dll, vcruntime140_1.dll
PeekGame: entering .../Palworld-Win64-Shipping.exe at 0x149fd7310
tbb.dll: DLL_PROCESS_ATTACH
dxgi.dll: DLL_PROCESS_ATTACH
d3d11.dll: DLL_PROCESS_ATTACH
vcruntime140.dll: DLL_PROCESS_ATTACH
vcruntime140_1.dll: DLL_PROCESS_ATTACH
msvcp140.dll: DLL_PROCESS_ATTACH
TLS callback 0x146555164: DLL_PROCESS_ATTACH
TLS callback 0x146555300: DLL_PROCESS_ATTACH
MessageBox [Steam Error]: Application load error B:0000065559
```

### Milestone Achieved:
1. `Palworld.exe` launcher succeeded completely from entry to child spawn.
2. `Palworld.exe` successfully called `CreateProcessW` which launched `Palworld-Win64-Shipping.exe Pal` natively under PeekGame.
3. The parent process received valid `hProcess` and waited synchronously on `WaitForSingleObject`.
4. The child process `Palworld-Win64-Shipping.exe` initialized DLLs, TLS, and entered the decrypted SteamStub DRM without crashing (`0x80000003` is gone).
