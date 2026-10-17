# Palworld Launcher: Visual C++ Runtime Prerequisite Check

## 1. Problem Overview
When running `Palworld.exe` via PeekGame, the launcher halts and displays a Win32 message box:
```
MessageBox []: The following component(s) are required to run this program:

Microsoft Visual C++ Runtime
```
and then prompts the user whether to run `Engine\Extras\Redist\en-us\UEPrereqSetup_x64.exe`.

---

## 2. Reverse Engineering the Prerequisite Check

Disassembly of `Palworld.exe` between `0x140001060` and `0x140001200` reveals the exact check:

### A. Registry Key Lookup
The launcher queries `HKEY_LOCAL_MACHINE` (`0xffffffff80000002`):
- **Key Path**: `SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64`
- **Values Queried**:
  - `Major` (DWORD)
  - `Minor` (DWORD)
  - `Bld` (DWORD)
  - `Rbld` (DWORD)

### B. Version Requirements
At `0x14000119b` - `0x1400011b0`:
```asm
14000119b:  cmp $0xe, %esi        ; Major >= 14 (0x0e)
14000119e:  ja  pass
1400011a0:  jne fail
1400011a2:  cmp $0x1f, %edi       ; Minor >= 31 (0x1f)
1400011a5:  ja  pass
1400011a7:  jne fail
1400011a9:  cmp $0x797f, %r14d    ; Bld >= 31103 (0x797f)
1400011b0:  jb  fail
```
The launcher requires Microsoft Visual C++ 2015-2022 Redistributable (x64) version >= `14.31.31103`.

### C. DLL Verification
After validating the registry version, the launcher calls helper `0x1400014a0` to verify that the following runtime DLLs can be loaded:
- `msvcp140_2.dll`
- `vcruntime140_1.dll`

If the registry key is missing, or the version is below `14.31.31103`, or the DLLs cannot be loaded, the launcher appends `Microsoft Visual C++ Runtime\n` to the missing component list and displays the message box.

---

## 3. Implementation Plan

1. **Seed Registry Key**:
   In `src/api/system/registry_keys.rs`, add `tree.seed_vc_runtime()` under `HKEY_LOCAL_MACHINE`:
   - Path: `SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64`
   - `Major`: `14` (DWORD)
   - `Minor`: `34` (DWORD)
   - `Bld`: `33810` (DWORD)
   - `Rbld`: `0` (DWORD)
   - `Installed`: `1` (DWORD)
   - `Version`: `v14.34.33810` (REG_SZ)

2. **DLL Availability**:
   Both `msvcp140_2.dll` and `vcruntime140_1.dll` already exist in `dist/windows-dlls/`. PeekGame's loader locates DLLs in `--dll-dir` / `windows-dlls`.

3. **Verification**:
   Run `Palworld.exe` and confirm the runtime check passes cleanly without displaying the missing component prompt.

---

## 4. Verification & Results

Running `Palworld.exe`:
```
SteamAppId=1623730 SteamGameId=1623730 ./dist/peekgame ~/.steam/debian-installation/steamapps/common/Palworld/Palworld.exe --run
```

Output:
```
PeekGame: entering Palworld.exe at 0x140001ca8
LoadLibrary: mapped .../msvcp140_2.dll
LoadLibrary: mapped .../msvcp140.dll
LoadLibrary: mapped .../vcruntime140.dll
LoadLibrary: mapped .../vcruntime140_1.dll
vcruntime140.dll: DLL_PROCESS_ATTACH
vcruntime140_1.dll: DLL_PROCESS_ATTACH
msvcp140.dll: DLL_PROCESS_ATTACH
msvcp140_2.dll: DLL_PROCESS_ATTACH
CreateProcess: - "Z:\home\nobus\.steam\debian-installation\steamapps\common\Palworld\Pal\Binaries\Win64\Palworld-Win64-Shipping.exe" Pal : child processes are not supported
```

### Result:
- **Prerequisite Check**: Fully passed! The launcher verified `SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64` (`Major=14`, `Minor=34`, `Bld=33810`), loaded `msvcp140_2.dll` and `vcruntime140_1.dll`, and proceeded directly to launch `Palworld-Win64-Shipping.exe`.
- **Next Frontier**: The launcher reached `CreateProcessW` attempting to spawn `Palworld-Win64-Shipping.exe Pal`.
