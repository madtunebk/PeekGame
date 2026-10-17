# SteamStub DRM wrapper bringup

Palworld (`Palworld-Win64-Shipping.exe`) originally stopped on entry with:
```
STOP: unhandled exception: exception 0x80000003 at Palworld-Win64-Shipping.exe+0x9fda0ec
```

## Reverse Engineering the `.bind` Section

1. **Header and OEP Discovery**:
   The PE entry point at `0x149fd7310` calls `0x149fd73d0`, which copies and decrypts 240 bytes (`0xf0`) of metadata from `EntryPoint - 0xf0` using CBC XOR.
   - Magic: `0xc0dec0df` verified at `0x149fd7540`.
   - Real Original Entry Point (OEP) RVA: `0x065562e0`.
   - Section size: `0x00039248`.

2. **String Table and IPC Protocol**:
   SteamStub decrypts 534 bytes of strings from the base of `.bind`, containing:
   - CRT memory and formatting: `calloc`, `free`, `vsprintf`.
   - Shared memory and event primitives: `Local\SteamStart_SharedMemFile`, `Local\SteamStart_SharedMemLock`, `OpenFileMappingA`, `OpenEventA`, `MapViewOfFile`, `UnmapViewOfFile`, `CloseHandle`.
   - Anti-debug / TEB checks: `gs:[0x30]` TEB verification and `PEB.BeingDebugged`.

3. **Abort / Bailout Handler**:
   `0x149fda0ec` (`int3` followed by `jmp short -1`) was the failure termination point of `0x149fd9fde`. If any required import returned NULL during dynamic discovery, SteamStub scrubbed sensitive stack buffers (`0xf0`, `0x1d0`, `0x280` bytes) and executed `int3`.

## Changes Made

### 1. `msvcrt.dll!vsprintf` (`src/api/crt/cxx_support.rs`, `src/registry/crt.rs`)
- Added standard `vsprintf` routing to `ucrt::common_vsprintf(0, buffer, usize::MAX, format, 0, args)`.
- Registered `vsprintf` as an export in `msvcrt.dll`.
- Cleared the initial `msvcrt.dll` loop failure.

### 2. `kernel32.dll!OpenFileMappingA` and `IsBadReadPtr` (`src/api/memory/mapping.rs`, `src/api/memory/virtual_memory.rs`, `src/registry/system/memory.rs`)
- Added `mapping::open_a` to look up named shared memory mappings with ANSI strings, matching `open_w`.
- Added `virtual_memory::is_bad_read_ptr` returning 0 for valid addresses.
- Registered both in `src/registry/system/memory.rs`.

## Verification and Breakthrough

Following the addition of `OpenFileMappingA` and `IsBadReadPtr`, rerunning `Palworld-Win64-Shipping.exe` eliminated the `0x80000003` crash completely!
All 16 kernel32 dynamic imports resolved cleanly:
- `TerminateProcess`, `GetLastError`, `OpenEventA`, `OpenFileMappingA`, `MapViewOfFile`, `WaitForSingleObject`, `CreateEventA`, `UnmapViewOfFile`, `CloseHandle`, `GetCurrentProcessId`, `VirtualAlloc`, `VirtualFree`, `VirtualProtect`, `IsBadReadPtr`, `OutputDebugStringA`, `FreeLibrary`.

SteamStub advanced into its active runtime logic:
- Checked debugger state (`IsDebuggerPresent`).
- Handled command line and file attributes.
- Attempted to locate the Steam client shared memory session `Local\SteamStart_SharedMemFile`.
- Displayed the official Steam DRM error modal dialog via `MessageBoxA`:
  ```
  MessageBox [Steam Error]: Application load error B:0000065559
  ```
- Error `B:0000065559` is the standard SteamStub code when the Steam DRM client IPC handshake cannot find an active Steam client session communicating on the shared memory block.
- Logged `FlushInstructionCache` as requested (`not_found 1`), confirming preparation for in-place section decryption once Steam authentication responds.
