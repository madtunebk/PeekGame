# Palworld Launcher Bringup (`Palworld.exe`)

**Data**: Octombrie 2026  
**Executabil**: `~/.steam/debian-installation/steamapps/common/Palworld/Palworld.exe`  
**Comandă**:
```bash
SteamAppId=1623730 SteamGameId=1623730 ./dist/peekgame /home/nobus/.steam/debian-installation/steamapps/common/Palworld/Palworld.exe --run
```

---

## 1. Rolul Executabilului `Palworld.exe`

`Palworld.exe` este launcher-ul oficial de Steam / bootstrapper-ul Unreal Engine:
1. Steam lansează direct `Palworld.exe` când utilizatorul dă „Play”.
2. `Palworld.exe` citește din secțiunea proprie de resurse (`.rsrc`, tip `RT_RCDATA` = 10) calea către binarul de producție:
   - Nume binar: `\Binaries\Win64\Palworld-Win64-Shipping.exe`
   - Subdirector: `Pal`
3. Folosește API-urile din `SHLWAPI.dll` pentru a construi calea canonică completă.
4. Inițializează sesiunea de Steam și lansează binarul shipping via `CreateProcessW`.

---

## 2. Modificări Implementate în PeekGame

### A. Adăugarea Funcțiilor de Manipulare Căi SHLWAPI (`src/api/shell/paths.rs`)
Implementate în modulul nou [`src/api/shell/paths.rs`](file:///home/nobus/projects/PeekGame/src/api/shell/paths.rs):
- **`PathCanonicalizeW(pszBuf, pszPath)`**:
  Normalizează separatoarele (`/` -> `\`), colapsează componentele relative `.` și `..`, și scrie rezultatul în buffer-ul de destinație.
- **`PathCombineW(pszDest, pszDir, pszFile)`**:
  Concatenează directorul de bază cu fișierul/subcalea și canonizează rezultatul.
- **`PathRemoveFileSpecW(pszPath)`**:
  Elimină numele fișierului și separatorul final dintr-o cale pentru a lăsa doar folderul părinte.

### B. Înregistrare Exporturi în Registru (`src/registry/shell.rs`)
Înregistrate sub `shlwapi.dll`:
```rust
export!("shlwapi.dll", "PathCanonicalizeW", "shell", "implemented", "canonicalizes path, collapses . and .. components", paths::canonicalize_w),
export!("shlwapi.dll", "PathCombineW", "shell", "implemented", "concatenates dir and file and canonicalizes", paths::combine_w),
export!("shlwapi.dll", "PathRemoveFileSpecW", "shell", "implemented", "removes trailing file name component", paths::remove_file_spec_w),
```

---

## 3. Rezultate și Următorul Pas

La re-testare:
- `PathCanonicalizeW`, `PathCombineW` și `PathRemoveFileSpecW` au rezolvat 3 din cele 9 importuri statice lipsă (scăzute de la 9 la 6).
- `Palworld.exe` a avansat în execuție până la încărcarea resurselor embedded:
```
STOP: missing Windows API KERNEL32.dll!FindResourceW, called from 0x1400013fe
```
- Următorul set de API-uri necesare pentru a permite launcher-ului să citească calea binarului shipping din `.rsrc`:
  - `KERNEL32.dll!FindResourceW`
  - `KERNEL32.dll!LoadResource`
  - `KERNEL32.dll!LockResource`
  - `KERNEL32.dll!SizeofResource`
