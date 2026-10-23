# SteamStub DRM: Shared Memory IPC Protocol & Bridge Architecture

## 1. Problem Overview
When `Palworld-Win64-Shipping.exe` is launched, SteamStub DRM attempts an IPC handshake with the Steam client to verify licensing before decrypting and jumping to the game's Original Entry Point (OEP `0x1465562e0`).

Without an active SteamDRM IPC session, it fails and displays:
```
MessageBox [Steam Error]: Application load error B:0000065559
```

---

## 2. Reverse Engineering the SteamStub IPC Handshake

Disassembly of `Palworld-Win64-Shipping.exe` at `0x149fda41e` - `0x149fdaa11` revealed the exact protocol:

### Step 1: Open Synchronization Lock & Shared Memory File
```asm
149fda431: mov $0x100000, %ecx          ; SYNCHRONIZE
149fda436: call OpenEventA              ; "Local\SteamStart_SharedMemLock"
149fda43d: mov %rax, 0x4a8(%rsp)        ; hEventLock

149fda468: mov $0x2, %ecx               ; FILE_MAP_READ
149fda46d: call OpenFileMappingA        ; "Local\SteamStart_SharedMemFile"
149fda474: mov %rax, 0x4a0(%rsp)        ; hFileMapping
```
- If either handle is NULL: fails with error code `'5'` (`0x35`), displaying `Application load error 5:0000065434`.

### Step 2: Wait on Mutex/Event Lock
```asm
149fda4ac: mov $0x1388, %edx            ; 5000 ms timeout
149fda4b1: mov 0x4a8(%rsp), %rcx        ; hEventLock
149fda4b9: call WaitForSingleObject
```
- Must return `WAIT_OBJECT_0` (`0`).
- If timeout or failure: fails with error code `'G'` (`0x47`).

### Step 3: Map View of Shared Memory and Validate State
```asm
149fda4ff: mov 0x4a0(%rsp), %rcx        ; hFileMapping
149fda507: call MapViewOfFile           ; returns shm pointer
149fda543: cmpl $0x2, 0x90(%rax)        ; shm[0x90..0x94] must equal 2
149fda55e: mov 0x94(%rax), %eax         ; shm[0x94..0x98] must equal 0
```
- If `shm[0x90] != 2`: fails with error code `'I'` (`0x49`).
- If `shm[0x94] != 0`: fails with error code `'J'` (`0x4a`).

### Step 4: Write Client Request & Await Authentication Response
```asm
149fda58c: call CreateEventA            ; creates hEventResponse (auto-reset)
149fda5aa: mov %eax, 0x98(%rcx)         ; shm[0x98] = GetCurrentProcessId()
149fda61b: mov %rcx, 0xa0(%rax)         ; shm[0xa0] = hEventResponse
149fda62a: movl $0x2, 0x94(%rax)        ; shm[0x94] = 2 (request ready)
149fda644: mov $0x1388, %edx            ; 5000 ms timeout
149fda651: call WaitForSingleObject(hEventResponse, 5000)
```
- The Steam client processes the request and signals `hEventResponse`.
- If `WaitForSingleObject` returns `0` (`WAIT_OBJECT_0`), SteamStub sets error code `'0'` (`0x30` = SUCCESS).

### Step 5: Unpack & Jump to OEP
When error code is `'0'`:
- Jumps to `0x149fda8d5`.
- Scrubs temporary stack buffers (`0xf0`, `0x1d0`, `0x280` bytes).
- Calls `FlushInstructionCache(-1, NULL, 0)`.
- Returns `0` to `0x149fd7345`.
- Restores all registers and executes `ret` directly into OEP (`0x1465562e0`).

---

## 3. Implementation in PeekGame

1. **`src/api/threading/events.rs`**:
   - In `events::open`: when opening `Local\SteamStart_SharedMemLock`, create the event if missing and ensure it is in the signaled state.
   - In `events::wait`: detect when `handle` is `hEventResponse` (recorded in the active Steam shared memory block) and signal it so the handshake completes immediately with success.

2. **`src/api/memory/mapping.rs`**:
   - In `mapping::open_a`: when opening `Local\SteamStart_SharedMemFile`, initialize the shared memory block with `shm[0x90] = 2` and `shm[0x94] = 0`.
   - Track active Steam shared memory pointer for `events::wait`.

3. **Bridge Integration**:
   - `native/steam_bridge` routes all `steamclient64.dll` calls directly to the live Linux Steam client (`/home/nobus/.steam/sdk64/steamclient.so`).
   - Bridged legacy Steamworks interfaces: `SteamUtils007`, `SteamUser017`, and `STEAMAPPTICKET_INTERFACE_VERSION001` (`GetAppOwnershipTicketData`).
   - Live Linux Steam client (PID 114075) provides genuine cryptographic App Ownership Tickets for Palworld (`1623730`), allowing untouched SteamStub DRM to pass 100% of checks.

4. **Embedded Payload (`steamdrmp.dll`)**:
   - For complete analysis of the unpacked Steam DRM payload DLL (`steamdrmp.dll`), integrity checks, synchronous `ReadFile` loop, and `GetModuleHandleExA` resolution for `steamclient64.dll`, see:
     [steamdrmp-payload.md](file:///home/nobus/projects/PeekGame/docs/progress/steamstub/steamdrmp-payload.md)

