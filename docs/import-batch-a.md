# Relink/import batch A: common A-variants, atoms, DOS time, misc

Implemented proactively from the DOOM `--relink` unresolved list — the cheap,
widely-called entries. The rest of that list (GDI drawing, menus, dialogs,
resources, threadpool/timer queues, DoDragDrop, MiniDumpWriteDump) stays
on-demand: an unresolved static import only stops if a game actually calls it.

## Implemented

- **Filesystem A-variants** (wrap the existing W; ANSI code page is UTF-8):
  `GetFileAttributesExA`, `CopyFileA`, `GetVolumeInformationA`, `CreateDirectoryA`,
  `DeleteFileA`, `RemoveDirectoryA` (`src/api/filesystem/files.rs`).
- **SearchPathW/A**: finds a file in the given path list, or the EXE directory then
  the current directory, adding a default extension when the name has none.
- **Global/local atoms** (`src/api/system/atoms.rs`): `GlobalAddAtomA/W`,
  `GlobalFindAtomA/W`, `GlobalGetAtomNameA/W`, `GlobalDeleteAtom`, `GlobalFlags`,
  and `AddAtom`/`FindAtom`/`GetAtomName`/`DeleteAtom`. One process-wide,
  reference-counted string<->u16 table (strings from 0xC000); integer atoms
  (MAKEINTATOM) pass through unchanged.
- **DOS date/time** (`src/api/system/time.rs`): `FileTimeToDosDateTime`,
  `DosDateTimeToFileTime` (1980-2107).
- **`GetProcessId`**: the current-process handle only; other handles
  ERROR_INVALID_HANDLE (no other processes exist here).
- **`CompareStringA`**: decodes the ANSI (UTF-8) strings to wide and uses the
  existing ordinal comparison with optional case folding.

## Verification

`python3 tests/test_import_batch_a.py` **PASS**. files_sync, move_file, case_paths,
save_safety, crt_directory, locale_info, system_times and `cargo test` pass; DOOM
still runs fullscreen.
