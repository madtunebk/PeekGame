# Investigare și Progres SteamStub (Palworld-Win64-Shipping.exe)

**Data analizei**: Octombrie 2026  
**Fișier analizat**: `Palworld-Win64-Shipping.exe`  
**Crash inițial**:
```
PeekGame: entering .../Palworld-Win64-Shipping.exe at 0x149fd7310
STOP: unhandled exception: exception 0x80000003 at Palworld-Win64-Shipping.exe+0x9fda0ec (flags 0x0, 0 parameters)
```

---

## 1. Modificări Efectuate în Codebase

Conform înțelegerii, modificările aduse în PeekGame sunt documentate aici:

1. **[`src/api/crt/cxx_support.rs`](file:///home/nobus/projects/PeekGame/src/api/crt/cxx_support.rs)**:
   - Adăugat wrapper-ul clasic de C `vsprintf`:
     ```rust
     /// vsprintf(buffer, format, argptr): standard C vsprintf for msvcrt.dll.
     pub unsafe extern "win64" fn vsprintf(
         buffer: *mut c_char,
         format: *const c_char,
         args: *const u64,
     ) -> i32 {
         unsafe { super::ucrt::common_vsprintf(0, buffer, usize::MAX, format, 0, args) }
     }
     ```

2. **[`src/registry/crt.rs`](file:///home/nobus/projects/PeekGame/src/registry/crt.rs)**:
   - Înregistrat exportul pentru `msvcrt.dll`:
     ```rust
     crt!("vsprintf", "standard vsprintf", crate::api::crt::cxx_support::vsprintf),
     ```

---

## 2. Structura Executabilului și Secțiunea `.bind`

Executabilul conține un wrapper SteamStub standard adăugat de Steam DRM:
- **ImageBase**: `0x140000000`
- **EntryPoint RVA**: `0x09fd7310` (VA: `0x149fd7310`)
- **Secțiuni**:
  - `.text` (VA: `0x1000`, VSize: `0x67134f7`)
  - `.pdata` (VA: `0x9736000`, VSize: `0x5874cc`)
  - `.bind` (VA: `0x9fd7000`, VSize: `0x39248`, RawPtr: `0x9a15600`) — Secțiunea wrapper SteamStub.

---

## 3. Punctul de Intrare (`0x149fd7310`) și Decriptarea Header-ului

Wrapper-ul începe cu salvarea registrelor și apelul către `0x149fd73d0`:
```asm
149FD7310  call 0x149fd7315
149FD7315  push rax ... push r15
149FD732C  mov rcx, [rsp+0x78]    ; return address
149FD7331  sub rcx, 0x5           ; rcx = 0x149fd7310 (EntryPoint)
149FD7338  mov rax, rsp
149FD733B  and rsp, -16
149FD7341  sub rsp, 0x20
149FD7345  call 0x149fd73d0       ; <--- Funcția principală SteamStub
149FD7353  mov [rsp+0x78], rax    ; Suprascrie adresa de retur cu OEP returnat în RAX!
149FD7358  pop r15 ... pop rax
ret                               ; Salt către OEP (jocul decriptat)
```

### Header-ul de Metadate (`EntryPoint - 0xf0`)
Decriptat via CBC XOR:
- Magic la offset `+0x04`: `0xc0dec0df` (verificat la `0x149fd7540`).
- Original Entry Point (OEP) la offset `+0x20`: **`0x065562e0`** (RVA).
- Dimensiune `.bind` la offset `+0x40`: `0x00039248`.

---

## 4. Tabela de String-uri Decriptată din `.bind` (534 octeți)

La adresa `0x149fd76e1`, SteamStub decriptează din baza secțiunii `.bind` o tabelă de 34 de string-uri:
- `0x0`: `calloc`
- `0x1`: `free`
- `0x2`: `vsprintf`
- `0x3`: `TerminateProcess`
- `0x4`: `GetLastError`
- `0x5`: `OpenEventA`
- `0x6`: `OpenFileMappingA`
- `0x7`: `MapViewOfFile`
- `0x8`: `WaitForSingleObject`
- `0x9`: `CreateEventA`
- `0xa`: `UnmapViewOfFile`
- `0xb`: `CloseHandle`
- `0xc`: `GetCurrentProcessId`
- `0xd`: `VirtualAlloc`
- `0xe`: `VirtualFree`
- `0xf`: `VirtualProtect`
- `0x10`: `IsBadReadPtr`
- `0x11`: `OutputDebugStringA`
- `0x12`: `FreeLibrary`
- `0x13`: `afterimports`
- `0x14`: `kernel32.dll`
- `0x15`: `msvcrt.dll`
- `0x16`: `user32.dll`
- `0x17`: `GetProcAddress`
- `0x18`: `GetModuleHandleA`
- `0x19`: `LoadLibraryA`
- `0x1a`: `MessageBoxA`
- `0x1b`: `Local\SteamStart_SharedMemFile`
- `0x1c`: `Local\SteamStart_SharedMemLock`
- `0x1d`: `Steam Error`
- `0x1e`: `Application load error X:XXXXXXXXXX`
- `0x1f`: `Payload routine failed with %u ('%c')`
- `0x20`: `Unpack step %u`
- `0x21`: `steam`

---

## 5. Rezultatul După Repararea `vsprintf` și Următorul Pas

După adăugarea `vsprintf`:
1. Bucla pentru `msvcrt.dll` a trecut cu succes:
   - `msvcrt.dll!calloc` -> `implemented`
   - `msvcrt.dll!free` -> `implemented`
   - `msvcrt.dll!vsprintf` -> `implemented` (Rezolvat!)
2. Execuția a avansat direct în bucla următoare (`0x149fd80cf` - `0x149fd8172`), care iterează prin primele 16 funcții din `kernel32.dll` (string-urile de la `0x3` la `0x12`):
   - `kernel32.dll!TerminateProcess` -> `implemented`
   - `kernel32.dll!GetLastError` -> `implemented`
   - `kernel32.dll!OpenEventA` -> `implemented`
   - **`kernel32.dll!OpenFileMappingA`** -> **`not_found 1`** ❌

La adresa `0x149fd8149`, codul verifică:
```asm
cmp qword [rax], 0    ; pointerul funcției OpenFileMappingA
jnz 0x149fd8159       ; dacă e valid, continuă bucla
jmp 0x149fd9fde       ; DACĂ E NULL -> ABORT la 0x149fda0ec (int 3)!
```

### Concluzie:
Pasul 1 a funcționat conform predicției. Următorul simbol cerut de SteamStub pentru a trece de inițializarea importurilor este **`kernel32.dll!OpenFileMappingA`** (urmat potențial de `MapViewOfFile`, `UnmapViewOfFile`, etc., adică API-urile de shared memory folosite de `Local\SteamStart_SharedMemFile`).
