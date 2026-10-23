# World of Warcraft Classic Bringup Progress

**Date**: 2026-10-19  
**Component**: World of Warcraft Classic (`WowClassic.exe`, `WowClassic_loader.dll`), PEB Module Loader (`PEB->Ldr`), Synthetic PE Builtin Images  
**Branch**: `doom-eternal-bringup`

---

## 1. Symptom

Launching `WowClassic.exe` with PeekGame:
```bash
PEEKGAME_TRACE_PEB_LDR=1 ./dist/peekgame '/media/nobus/7CB22B82B22B3FCE/Games/World of Warcraft/_classic_/WowClassic.exe' --run
```
immediately crashed during `DLL_PROCESS_ATTACH` in `WowClassic_loader.dll`:
```text
SEH dispatch: exception 0xc0000005 at WowClassic_loader.dll+0x1b6527c (flags 0x0, 2 params: [0000000000000000, 0000000000000000])
[peb] fault registers: rax=0x0 rcx=0xec01e6bd
  exception_frame[0] = 0x181b6527c (WowClassic_loader.dll+0x1b6527c)
  exception_frame[1] = 0x180d4ed83 (WowClassic_loader.dll+0xd4ed83)
STOP: unhandled exception: exception 0xc0000005 at WowClassic_loader.dll+0x1b6527c (flags 0x0, 2 parameters)
```

---

## 2. Reverse Engineering & Root Cause

### Crash Analysis
- Disassembly at `WowClassic_loader.dll+0x1b6527c`:
  ```asm
  cmpw $0x5a4d, (%rax)
  ```
  with `%rax == 0x0` causes an immediate null pointer dereference (`0xc0000005`).
- The call chain originates from the first TLS callback of `WowClassic_loader.dll` at `0x180d4e4c0` (registered in the TLS directory at `0x181fbf418`).
- The TLS callback invokes `0x181b61ab0(1, 0x24, 0x5c)`.

### Function `0x181b61ab0` Behavior
- Reads `gs:0x60` to obtain the `PEB` pointer.
- Reads `PEB->Ldr` (`PEB + 0x18`) and traverses `InLoadOrderModuleList` (`Ldr + 0x10`).
- For each `LDR_DATA_TABLE_ENTRY`, it reads `BaseDllName.Buffer` (`+0x60`) and `BaseDllName.Length` (`+0x58`).
- Converts the UTF-16 wide string to ASCII and lowercases it.
- Computes a hash and compares it against expected module hashes.
- Proved via standalone C harness: the target module being searched is **`kernel32.dll`**.

### Export Resolution inside `kernel32.dll`
- Once `kernel32.dll` is found, the loader verifies the DOS header (`'MZ'` / `0x5a4d`) and reads `IMAGE_NT_HEADERS64` -> `IMAGE_OPTIONAL_HEADER64.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT]`.
- It iterates through the exported names, computing **64-bit FNV-1a** hashes (`basis = 0xcbf29ce484222325`, `prime = 0x100000001b3`):
  - `0x578960f1fc7fff25` -> `GetProcAddress`
  - `0xfa55e32c9d72a921` -> `VirtualAlloc`
- If `kernel32.dll` is not present in `PEB->Ldr`, the loop finishes without matching any module. The fallback path at `0x181b68392` sets the module pointer to 0 and jumps to `0x181b6527c`, which immediately checks `cmpw $0x5a4d, (%rax)` with `%rax == 0`, causing the crash.
- After resolving `GetProcAddress`, it calls:
  ```c
  GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                     (LPCSTR)0x180110e90,
                     &hModule);
  ```
  to retrieve its own base address.

### PeekGame Architecture Gap
PeekGame implements Win32 system libraries (`kernel32.dll`, `ntdll.dll`, etc.) purely in host Rust code without mapping Windows PE binaries.
As a result:
1. `PEB->Ldr->InLoadOrderModuleList` only contained the PE binaries loaded from disk (`WowClassic.exe` and `WowClassic_loader.dll`).
2. PeekGame's `GetModuleHandleA("kernel32.dll")` and `builtin_handle` return dummy pseudo-handles (`0x50470000 + idx*16`), which are unmapped and invalid as PE bases.
3. Games using anti-cheat, packers, or manual PEB walkers that inspect `PEB->Ldr` for `kernel32.dll` / `ntdll.dll` and parse their export directories directly will fail or crash.

---

## 3. Implementation & Resolution

1. **Synthetic Builtin PE Image Generator ([`src/images/synthetic.rs`](file:///home/nobus/projects/PeekGame/src/images/synthetic.rs))**:
   - Implemented `build_builtin_pe()` to generate in-memory PE32+ images with:
     - DOS header (`0x5a4d`, `e_lfanew = 0x80`).
     - PE32+ header with `IMAGE_EXPORT_DIRECTORY`.
     - Lexicographically sorted export name table (`AddressOfNames`), ordinal table, and function RVA table.
     - `.text` section containing 16-byte x86_64 indirect jump thunks (`ff 25 00 00 00 00 <64-bit absolute fn ptr>`).
   - Implemented `create_builtin_module()` which maps and protects the synthetic image via [`Mapping::new`](file:///home/nobus/projects/PeekGame/src/images/map.rs#L33) and wraps it in a standard [`images::Module`](file:///home/nobus/projects/PeekGame/src/images/mod.rs#L27).

2. **PEB Module Loader List Ordering ([`src/images/mod.rs`](file:///home/nobus/projects/PeekGame/src/images/mod.rs))**:
   - Implemented `ensure_builtin_modules()` to synthesize `ntdll.dll`, `kernel32.dll`, and `KERNELBASE.dll`.
   - Placed them immediately after the main executable in standard Windows loader order:
     `[WowClassic.exe, ntdll.dll, kernel32.dll, KERNELBASE.dll, WowClassic_loader.dll]`.

3. **Dynamic Export and Handle Lookups ([`src/api/system/dynamic.rs`](file:///home/nobus/projects/PeekGame/src/api/system/dynamic.rs), [`src/api/system/modules.rs`](file:///home/nobus/projects/PeekGame/src/api/system/modules.rs))**:
   - [`builtin_handle`](file:///home/nobus/projects/PeekGame/src/api/system/dynamic.rs#L47) and [`named`](file:///home/nobus/projects/PeekGame/src/api/system/modules.rs#L22) return the mapped synthetic image base.
   - [`get_proc_address`](file:///home/nobus/projects/PeekGame/src/api/system/dynamic.rs#L182) returns the thunk address inside the synthetic PE image (preserving module range integrity for anti-cheat).

---

## 4. Verification & Milestone 2

- `cargo test --offline`: All 35 tests pass.
- All 4 previously missing libraries (`sfc.dll`, `tbs.dll`, `MSACM32.dll`, `ncrypt.dll`) and all 6 unresolved static imports (`IPHLPAPI.DLL!FreeMibTable`, `MSACM32.dll!acmStreamUnprepareHeader`, `ncrypt.dll!NCryptFreeObject`, `ntdll.dll!NtClose`, `sfc.dll!SfcIsFileProtected`, `tbs.dll!Tbsi_Context_Create`) now resolve cleanly without any warnings or traps.
- In addition, key dynamic APIs were implemented: `advapi32.dll!CryptGenRandom`, `ntdll.dll!NtFlushInstructionCache`, `ntdll.dll!LdrLockLoaderLock`, `ntdll.dll!LdrUnlockLoaderLock`, `ntdll.dll!NtQueryVirtualMemory`, `ntdll.dll!NtSetInformationProcess`, `kernel32.dll!GetSystemDefaultLangID`, `kernel32.dll!GetCurrentPackageId`, and activation context functions (`CreateActCtxW`, `ActivateActCtx`, `DeactivateActCtx`).
- New dynamic requests observed from the loader's TPM subsystem:
  - `tbs.dll!Tbsip_Context_Close`, `tbs.dll!Tbsip_Submit_Command`
  - `ntdll.dll!NtCreateSection`, `ntdll.dll!NtContinue`, `ntdll.dll!NtGetNextThread`
  - `crypt32.dll!CertFreeCRLContext`, `crypt32.dll!CertFreeCTLContext`
  - `kernel32.dll!StartThreadpoolIo`
- Architecture Check:
  - Game binaries are strictly 64-bit PE32+ (`WowClassic.exe`, `WowClassic_loader.dll`, `GFSDK_Aftermath_Lib.x64.dll`, `libxell.dll`, `UTILS/BlizzardBrowser.exe`, etc.).
  - The only 32-bit executables are `World of Warcraft Launcher.exe` (root) and `BlizzardError.exe` (crash reporter helper).
  - 32-bit support is NOT required to run the game directly.

---

## 5. Milestone 3: Unpacker State Machine & Entry Point Analysis

### APC Dispatch & Eidolon Unpacker Flow
1. `QueueUserAPC` is called with routine `0x180384550` on the main thread during DLL attach.
2. Routine `0x180384550` implements a state machine loop:
   - Evaluates state from `0x18203b5e4`.
   - When entering state `0x47d5a11c`, it calls `0x180342a90` (environment / anti-tamper validation) followed by `0x181d1e840` (`int3` trap).
   - The SEH filter at `0x180384d20` only handles `0xE06D7363` (`STATUS_MSC_EXCEPTION`) and treats `0x80000003` as fatal, spawning `BlizzardError.exe`.
3. Game Entry Point Handoff:
   - `WowClassic.exe` entry point at `0x1401d0c40` checks `0x143d78738`:
     ```asm
     1401d0c40: sub    $0x28, %rsp
     1401d0c44: mov    0x143d78738, %rax
     1401d0c4b: test   %rax, %rax
     1401d0c4e: je     0x1401d0c52
     1401d0c50: call   *%rax
     1401d0c52: xor    %eax, %eax
     1401d0c54: add    $0x28, %rsp
     1401d0c58: ret
     ```
   - Once the unpacker loop finishes unpacking, it writes the actual game entry point address into `0x143d78738`.
   - Bypassing the assertion or satisfying the environment validation allows the unpacker to complete and execute the main game engine.

---

## 6. Milestone 4: SSPI DTLS/TLS 1.2, 10MHz Timers, & Worker State Machines

### 1. SSPI Schannel TLS 1.2 Handshake & Session Teardown
- **Negotiation**: Implemented and verified the complete client-server TLS 1.2 handshake over OpenSSL in [`src/api/network/schannel.rs`](file:///home/nobus/projects/PeekGame/src/api/network/schannel.rs) and [`src/api/network/sspi.rs`](file:///home/nobus/projects/PeekGame/src/api/network/sspi.rs).
- **ALPN & Attributes**: Implemented `QueryContextAttributesW(SECPKG_ATTR_APPLICATION_PROTOCOL)` returning `SecApplicationProtocolNegotiationStatus_None` (0).
- **Packet Encryption / Decryption**: Decrypted live Battle.net challenges (`DecryptMessage total=321`), encrypted client hardware tokens (`EncryptMessage len=2067`), and received decryption payload (`DecryptMessage total=277`).
- **Control Token (Slot 9)**: Implemented `apply_control_token` handling `SCHANNEL_SHUTDOWN` (`0x1`), driving clean SSL shutdown token production and context teardown (`DeleteSecurityContext`).

### 2. High-Resolution Hardware Timing (Win 10/11 10MHz Standard)
- **Root Cause**: The unpacker validated `QueryPerformanceFrequency` with `cmp $0x989680, %rax` (`0x989680` == 10,000,000 counts/sec / 10 MHz standard for Win 10/11) and scaled QPC counters via `imul $0x64, counter, %rax` (converting 100ns units to nanoseconds). PeekGame previously returned 1 GHz (1,000,000,000), causing deadline comparisons (`sub %rax, %r8; jle 0x181139ee0`) to miscalculate.
- **Resolution**:
  - Configured `performance_frequency` to return `10_000_000` and `performance_counter` to `time.sec * 10_000_000 + time.nsec / 100` in [`src/api/system/startup.rs`](file:///home/nobus/projects/PeekGame/src/api/system/startup.rs).
  - Populated `QpcFrequency = 10_000_000` at offset `0x3c0` in `KUSER_SHARED_DATA` in [`src/host/user_shared_data.rs`](file:///home/nobus/projects/PeekGame/src/host/user_shared_data.rs).
  - Confirmed via GDB that QPC deadline arithmetic triggers accurately and the deadline sleep check exits cleanly.

### 3. Reverse Engineered Child Worker Thread Topology
- **Master Orchestrator (`0x181d53b50`, tid 67482)**: Waits on event `0x65000000`. Released by PeekGame startup signaling; exits cleanly with code 0.
- **Worker 2 (`0x181deb130`, tid 67484)**: Waits on event `0x65000040`. Released by PeekGame startup signaling; completes its unpacker stage and exits with code 0.
- **Worker 3 (`0x181a629b0`, tid 67485)**: Waits on event `0x65000050`. Released by PeekGame startup signaling; completes its unpacker stage and exits with code 0.
- **Worker 1 (`0x1804e0d50`, tid 67486)**:
  - Woken from its initial `INFINITE` wait on `0x65000030`, then enters a 100ms polling loop:
    ```asm
    1812d2363: call *%rsi ; WaitForSingleObject(0x65000030, 100)
    1812d2365: cmp $0x102, %eax ; 0x102 == 258 (WAIT_TIMEOUT)
    1812d236a: jne 0x1812d384a ; IF NOT TIMEOUT -> abort/exception handler!
    ```
  - Proved via testing that signaling `0x65000030` a second time triggers `0x1812d384a`, which deliberately raises non-continuable exception `0xc00000e5` (`STATUS_INTERNAL_ERROR`, arguments `[0xe1d04e02, 0x908]`) and spawns `BlizzardError.exe`.
  - Confirmed that Worker 1 is intentionally designed as an active heartbeat worker that expects timeout `0x102`.
- **Worker 4 (`0x1804e0d50`, tid 67483)**:
  - Primary network & crypto unpacker coordinator.
  - After TLS communication teardown, enters coordinator loop `0x180814846`.
  - In state `0x79968f90`, calls C++ virtual method `*0x10(%rax)` (`0x1813be6f0` in vtable `0x181f6c918`), which delegates to sub-task processor `0x181a0f5c0`.
  - Next calls polling routine `0x18140e8e0`, which computes a 3.5-second deadline via `0x180a9ef40` and calls `wait_for_deadline` (`0x181137bbc` to `0x181139f62`).
  - GDB tracing confirmed `wait_for_deadline` finishes its 3-second sleep cycle and returns to `0x18140ea18`, dispatching state `0x778709d9` into outer unpacker state machine `0x180814846`.
  - **Coordinator Epilogue & Exit**:
    - Dispatches state `0x6fae5428` (`0x1808147dc`) -> computes `0x104(%rbp)`.
    - Dispatches state `0x50c4e68a` (`0x180814e5f`) -> computes `0x100(%rbp)`.
    - Hits exit branch at `0x180814d25` -> `0x180815cc3`:
      ```asm
      180815cc3: mov 0x100(%rbp), %eax
      180815cc9: sub 0x11c(%rbp), %eax
      180815ccf: xor 0x160(%rbp), %eax
      180815cd5: add $0x1f8, %rsp
      180815cdc: pop %rbx ...
      180815ce8: ret
      ```
    - Returns cleanly with exit code `0` directly into `peekgame_thread_call`.
    - Worker 4 finishes its coordinator lifecycle completely without unhandled traps or errors.

---

## 7. Milestone 6: 32-Bit Subsystem Architecture for Blizzard Services

### Target Executables & Component Breakdown
Blizzard's client environment consists of two architectures:
1. **64-Bit Core Game Engine (`x86_64` / `PE32+` / `0x8664`)**:
   - `WowClassic.exe` (main game client).
   - `WowClassic_loader.dll` (engine unpacker and crypto runtime).
   - `UTILS/BlizzardBrowser.exe` (Chromium UI subprocess).
   - `UTILS/WowVoiceProxy.exe` (voice chat helper).
2. **32-Bit Service & Launcher Subsystem (`i386` / `PE32` / `0x14c`)**:
   - `Agent.exe` (local HTTP REST update daemon, listening on `127.0.0.1:1119` / `1120`).
   - `World of Warcraft Launcher.exe` (legacy desktop launcher).
   - `Battle.net.exe` (Battle.net Desktop App).
   - `BlizzardError.exe` (32-bit crash reporter).

### Architecture Roadmap for 32-Bit Support
- **Dual-Binary Architecture (`peekgame` + `peekgame32`)**:
  - `peekgame` (native 64-bit host) inspects the target PE machine type.
  - If `machine == 0x14c` (`IMAGE_FILE_MACHINE_I386`), it delegates execution to companion binary `peekgame32`.
- **PE32 Header Parsing in [`src/pe/mod.rs`](file:///home/nobus/projects/PeekGame/src/pe/mod.rs)**:
  - Added support for `IMAGE_FILE_MACHINE_I386` (`0x14c`) and optional header magic `0x10b` (`PE32`).
  - Implemented 32-bit image base extraction (`u32` at offset 28), data directories offset mapping (starting at 96 vs 112 in PE32+), and 32-bit IAT/ILT thunk parsing in [`src/pe/imports.rs`](file:///home/nobus/projects/PeekGame/src/pe/imports.rs) (4-byte stride, `1 << 31` ordinal mask).
  - Added support for 32-bit TLS directory (24 bytes) and `IMAGE_REL_BASED_HIGHLOW` (type 3) base relocations in [`src/pe/directories.rs`](file:///home/nobus/projects/PeekGame/src/pe/directories.rs).
  - Updated reports in [`src/report/text.rs`](file:///home/nobus/projects/PeekGame/src/report/text.rs) and [`src/report/json.rs`](file:///home/nobus/projects/PeekGame/src/report/json.rs) to distinguish `i386 PE32` vs `x86-64 PE32+`.
- **Dedicated & Isolated 32-Bit Module Architecture**:
  - [`src/pe32/mod.rs`](file:///home/nobus/projects/PeekGame/src/pe32/mod.rs): Fully independent 32-bit PE parser with dedicated `Image32`, `Section32`, `Import32`, `Export32`, `Tls32` structures.
  - [`src/images32/mod.rs`](file:///home/nobus/projects/PeekGame/src/images32/mod.rs): Dedicated 32-bit module manager and address space mapper (`Mapping32` using `MAP_32BIT` under 4GB).
  - [`src/loader32/mod.rs`](file:///home/nobus/projects/PeekGame/src/loader32/mod.rs): Dedicated 32-bit process loader cleanly decoupled from the 64-bit game loader.
  - Main loader automatically dispatches 32-bit PEs to `loader32` while keeping the 64-bit engine pipeline pristine.

