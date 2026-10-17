# Valve Steam DRM Embedded Payload (`steamdrmp.dll`) Analysis & Resolution

## 1. Executive Summary

When launching `Palworld-Win64-Shipping.exe` under PeekGame with live Linux Steam client connectivity, the process progressed past initial IPC synchronization but terminated abruptly with exit code **51** (`0x33` in ASCII = `'3'`, corresponding to Steam's error message `Application load error 3:0000065432`).

By instrumenting memory allocations and disassembling the execution path, we discovered and dumped the hidden embedded Valve DRM DLL: **`steamdrmp.dll`**.

This document details:
1. The discovery, extraction, and structure of `steamdrmp.dll`.
2. The verification sequence performed by the payload.
3. The root causes of the exit code 51 (`'3'`).
4. The fixes in PeekGame's runtime.

---

## 2. Discovery & Extraction of `steamdrmp.dll`

### 2.1 The Allocation Sequence
During early startup within the `.bind` section of `Palworld-Win64-Shipping.exe`, SteamStub unpacks a 228 KB (`0x39000` bytes) PE32+ module into memory:
```
VirtualAlloc(address=0x180000000, size=0x39000, kind=0x2000, prot=0x4) -> 0x0 (collided with tbb.dll)
VirtualAlloc(address=0x0, size=0x39000, kind=0x2000, prot=0x4) -> dynamic base (e.g. 0x7d99e0e40000)
VirtualAlloc(address=base, size=0x39000, kind=0x1000, prot=0x4)
VirtualAlloc(address=base+0x1000, size=0x21000, kind=0x1000, prot=0x4) ; .text
VirtualAlloc(address=base+0x22000, size=0xe400, kind=0x1000, prot=0x4)  ; .rdata
VirtualAlloc(address=base+0x31000, size=0xe00, kind=0x1000, prot=0x4)   ; .data
VirtualAlloc(address=base+0x35000, size=0x1a00, kind=0x1000, prot=0x4)  ; .pdata
VirtualAlloc(address=base+0x37000, size=0x200, kind=0x1000, prot=0x4)   ; .fptable
VirtualAlloc(address=base+0x38000, size=0x800, kind=0x1000, prot=0x4)   ; .reloc
```

### 2.2 PE Characteristics
- **Binary Type**: PE32+ (64-bit DLL), 6 sections.
- **PDB Path**: `C:\buildworker\steam_rel_rack_win64\build\src\codetools\drmpayload\win64\Release\steamdrmp.pdb`
- **Internal Name**: `steamdrmp.dll`
- **Exports**: Exactly 1 export:
  - Function name: `steam`
  - Ordinal: 1
  - RVA: `0x52c0`

---

## 3. Disassembly of the Payload Verification Pipeline

At `0x149fd9c21` in `Palworld-Win64-Shipping.exe`:
```asm
149fd9c0e: mov $0xf0, %r8d          ; Size of DRM parameters struct
149fd9c14: mov 0x38(%rsp), %rdx     ; Pointer to DRM parameters struct
149fd9c19: mov 0xd60(%rsp), %rcx    ; Main executable image base / entry pointer
149fd9c21: call *0x6d8(%rsp)        ; Call steamdrmp.dll!steam (RVA 0x52c0)
149fd9c28: mov %al, 0x30(%rsp)      ; Store return status
149fd9c2c: movsbl 0x30(%rsp), %eax
149fd9c31: cmp $0x30, %eax          ; Is return code == '0' (0x30 = SUCCESS)?
149fd9c34: je 149fd9c40             ; Success -> Proceed to OEP jump!
149fd9c36: jmp 149fda0ef            ; Failure -> TerminateProcess(code)
```

Inside `steamdrmp.dll!steam` (RVA `0x52c0`):
1. **Sanity Check**:
   - Validates that `r8 >= 0xf0` and struct magic at `0x4(rdx) == 0xc0dec0df`.
2. **Anti-Debug / Thread Configuration**:
   - Checks `IsDebuggerPresent()`.
   - Resolves `ntdll.dll!NtSetInformationThread` dynamically and sets `ThreadHideFromDebugger` (`0x11`).
3. **Steam Client Handshake**:
   - Attempts to locate and verify `steamclient64.dll`.
   - Initializes `SteamAPI_Init` / `CreateInterface`.
4. **Integrity & Origin Check Function (`0x4040`)**:
   ```asm
   7c14eea753b1: mov %r14, %rcx
   7c14eea753b4: mov %r14, %rdx
   7c14eea753b7: sub 0x10(%rdi), %rcx
   7c14eea753bb: call 0x4040         ; Check Module Integrity
   7c14eea753c0: test %al, %al
   7c14eea753c2: jne 0x53cb          ; Success -> Continue
   7c14eea753c4: mov $0x33, %al      ; Error '3' (0x33)!
   7c14eea753c6: jmp 0x561b          ; Exit with error '3' -> TerminateProcess(51)
   ```

---

## 4. Root Causes of Exit Code 51 (`'3'`)

Reverse engineering function `0x4040` revealed two distinct issues that caused it to fail:

### Issue 1: Truncated Read in Synchronous `ReadFile`
- In `0x4040`:
  1. `GetModuleFileNameW` retrieves the Windows path to `Palworld-Win64-Shipping.exe`.
  2. `CreateFileW` opens the binary for reading.
  3. `GetFileSize` gets the exact size (`155 MB` = `162,996,224` bytes).
  4. `malloc(file_size)` allocates a buffer for the entire binary.
  5. `ReadFile(hFile, buf, file_size, &bytes_read, NULL)` reads the file.
  6. **Integrity Check**:
     ```asm
     cmp %r14d, bytes_read  ; Must equal full file size!
     jne error_exit
     ```
- **PeekGame Flaw**:
  PeekGame's `ReadFile` implementation called Rust's standard `(&file).read(slice)` once. On Linux, a single `read(2)` syscall on a large file yields at most 8–32 MB, leaving `bytes_read < file_size`.
- **Solution**:
  Updated `src/api/filesystem/files.rs` so synchronous disk file reads loop until all requested bytes are read or EOF is reached.

### Issue 2: `GetModuleHandleExA(FROM_ADDRESS)` on Builtin / Bridge Exports
- In `0x4040` (and `SteamAPI_Init`):
  ```asm
  mov $0x6, %ecx                     ; GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | UNCHANGED_REFCOUNT
  lea 0x20(%rsp), %r8                ; phModule
  call GetModuleHandleExA            ; rdx = address of function
  test %eax, %eax
  je error_exit                      ; Returns false -> Error '3'!
  ```
- **Address Passed**:
  The address passed was inside PeekGame's `native/steam_bridge` (`steamclient64.dll`).
- **PeekGame Flaw**:
  `GetModuleHandleExA` with `FROM_ADDRESS` only checked `images::by_address(address)`, which only looks up loaded external PE files.
- **Solution**:
  1. Mapped the genuine Windows `steamclient64.dll` PE image at `0x138000000` with full PE headers and exports.
  2. Implemented `steam_bridge_contains_address` to resolve `GetModuleHandleExA(FROM_ADDRESS)` accurately for all bridge thunks and vtables.
  3. Mapped Windows `C:\Program Files (x86)\Steam\` drive paths to the host Linux Steam client directory.

---

## 5. Exit Code 53 ('5') Resolution: `SteamUtils007` & `SteamUser017`

- **Symptom**:
  `steamdrmp.dll` invoked `ISteamClient::GetISteamUtils(hPipe, "SteamUtils007")`. Because `steam_bridge.c` only bridged `SteamUtils010` and `SteamUtils009`, it returned `NULL`. SteamStub interpreted this as Steam client communication failure, launched `ShellExecuteA("steam://run/1623730//")`, and terminated with code 53 ('5').
- **Resolution**:
  1. Added `UTILS007` and `UTILS008` (26 and 28 slots, exact prefixes of `ISteamUtils010`) to `steam_bridge.c`.
  2. Added `USER017`, `USER018`, `USER019` (24, 25, 29 slots, exact prefixes of `ISteamUser021`) to `steam_bridge.c`.
  3. Linux `steamclient.so` directly implements `CAdapterSteamUtils007` and `SteamUser017`. Verified via host probe that live Linux Steam answers both queries natively with matching 1:1 vtables.

---

## 6. Exit Code 54 ('6') Resolution: `STEAMAPPTICKET_INTERFACE_VERSION001`

- **Symptom**:
  After `SteamUtils007`, `steamdrmp.dll` invoked `ISteamClient::GetISteamGenericInterface(hUser, hPipe, "STEAMAPPTICKET_INTERFACE_VERSION001")` to retrieve the cryptographic app ownership ticket for Palworld (`1623730`). Returning `NULL` triggered exit code 54 ('6').
- **Resolution**:
  1. Bridged `ISteamAppTicket001` (`STEAMAPPTICKET_INTERFACE_VERSION001`) with its single slot:
     ```c
     uint32_t GetAppOwnershipTicketData(uint32_t nAppID, void *pvBuffer, uint32_t cubBuffer,
                                        uint32_t *piLength, uint32_t *piAppId,
                                        uint32_t *piSteamId, uint32_t *piSignature);
     ```
  2. Forwarded the call to Linux `steamclient.so`'s `CAdapterSteamAppTicket001`.
  3. Live Linux Steam returned a genuine 196-byte ownership ticket (`cubBuffer=4096`, `piLength=16`, `piAppId=8`, `piSteamId=68`, `piSignature=128`).
  4. SteamStub verified the cryptographic ticket signature against Valve's public key.

---

## 7. Complete Verification & Transfer to Game OEP

With `SteamUtils007`, `SteamUser017`, and `STEAMAPPTICKET_INTERFACE_VERSION001` bridged:
1. `[S_API] SteamAPI_Init(): Loaded 'C:\Program Files (x86)\Steam\steamclient64.dll' OK.` succeeded.
2. In-memory vs on-disk PE integrity checks passed.
3. Steam ownership ticket verified legally against live Linux Steam client PID 114075.
4. SteamStub fully decrypted the `.text` section of `Palworld-Win64-Shipping.exe`.
5. Execution jumped directly into Palworld's entry point (`0x14350595e` in `.text`), successfully launching Unreal Engine 5!

---

## 8. Artifacts and Preservation

- Decrypted DRM Payload dump: `/tmp/steamstub_payload.dll` / `/tmp/steamstub_mem.dll`
- PDB: `steamdrmp.pdb`
- Export: `steam` (Ordinal 1, RVA `0x52c0`)
- Key documentation linked from:
  - `docs/progress/steamstub-ipc.md`
  - `docs/progress/steamstub/steamstub-investigation.md`

