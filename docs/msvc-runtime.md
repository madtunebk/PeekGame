# MSVC runtime bring-up

MSVCRT export parity with the earlier project is not full MSVC support. Modern
MSVC executables use UCRT API-set contracts plus VCRUNTIME140, including C++ EH.
Do not redirect every VCRUNTIME export to MSVCRT or return fake startup success.

## Observed applications

Static reports: `cache/msvc-audit/{arbgeom,marching}/imports.{json,txt}`.
These reports describe the actual local binaries, not a universal MSVC ABI list.
Both initially stop at `_set_app_type`; after the fix both stop at `_set_fmode`.
Private Xvfb startup results: `cache/msvc-audit/*/startup.log`.

## Implemented import

- **Name:** api-ms-win-crt-runtime-l1-1-0.dll!_set_app_type.
- **Implementation:** registry/crt.rs maps the UCRT name to the existing
  startup::app_type state setter. _query_app_type reads that state; legacy
  __set_app_type shares it under the current single-CRT runtime model.
- **Behavior:** unknown/console/GUI types 0/1/2; invalid values retain the existing
  explicit STOP policy. No display, console or dialog backend is added.
- **Verification:** tests/test_msvc_app_type.py builds an actual API-set import
  library and PE fixture, checks state and legacy alias, then runs relinked ELF.
  Both pass. Existing tests/test_crt_port.py passes, including its ELF fixture.
- **Reference:** https://learn.microsoft.com/en-us/cpp/c-runtime-library/set-app-type

## Remaining work from the import audit

1. UCRT stdio configuration: _set_fmode and __p__commode, including application
   of text/binary defaults by real stream/descriptor operations.
2. UCRT startup/environment and teardown: initial environment accessor,
   thread-local EXE exit callbacks, _c_exit, termination and exception filtering.
   Audit existing argv/environment helpers before claiming complete startup.
3. Thread-locale CRT configuration and remaining math functions.
4. VCRUNTIME140: separately expose compatible memory operations and the existing
   C SEH handler. C++ throw/frame handlers and current-exception state require
   actual MSVC EH semantics; no unconditional success stubs.

Neither game is certified to reach main/rendering yet. Distribution is staged;
this is a verified first startup binding correction, not a full MSVC port.

## File-mode startup configuration

- **Exports:** _set_fmode, _get_fmode, __p__fmode, __p__commode, via UCRT API sets.
- **Source:** api/crt/file_mode.rs and registry/crt.rs. File mode shares the
  existing exported _fmode storage; its initial value is now _O_TEXT (0x4000).
  Accepted settings are _O_TEXT, _O_BINARY and _O_WTEXT. Invalid values/null get
  output return EINVAL and set errno without changing the prior mode. The runtime
  does not yet expose the UCRT invalid-parameter callback mechanism.
- **Scope:** real configuration storage and pointer access only. Existing stdio
  does not implement CRLF/Unicode stream translation, and _open/_pipe are not
  implemented. __p__commode shares existing storage; automatic commit-on-flush is
  unsupported. No claim of complete Windows text I/O behavior.
- **Verification:** tests/test_msvc_fmode.py passes actual API-set PE imports and
  relinked ELF, defaults, accepted modes, invalid input, errno preservation and
  shared pointer writes. CRT-port regressions pass. Release rebuilt/staged.
- **Game result:** both arbgeom and marching_ants_game pass these calls and now
  stop at api-ms-win-crt-locale-l1-1-0.dll!_configthreadlocale. Neither reaches main
  yet; logs are cache/msvc-audit/*/fmode-startup.log.
- **Reference:** https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/set-fmode

## CRT thread-locale policy

- **Export:** api-ms-win-crt-locale-l1-1-0.dll!_configthreadlocale.
- **Implementation:** api/crt/locale.rs. Thread-local state starts disabled (2);
  mode 0 queries, 1 enables, 2 disables. Returns previous policy. Invalid inputs
  return -1/set errno EINVAL, preserving policy. Successful calls preserve errno.
- **Scope:** existing CRT locale contents remain C/POSIX only; both policies select
  that supported locale. Unknown locale names still fail setlocale. No host-global
  setlocale call and no claim of broader locale support. Undocumented global/new-
  thread flag variants and invalid-parameter callbacks are not implemented.
- **Verification:** tests/test_msvc_thread_locale.py passes PE and ELF, query,
  transitions, invalid input, errno, worker defaults and isolation. CRT parity
  regressions pass. Release rebuilt/staged.
- **Game result:** both binaries pass this API and now stop at
  _get_initial_narrow_environment. Logs: cache/msvc-audit/*/locale-startup.log.
- **Reference:** https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/configthreadlocale

## Initial environment, math and explicit VCRUNTIME subset

- **Exports:** _get_initial_narrow_environment returns the existing CRT argument
  state's stable, NUL-terminated UTF-8 environment snapshot. No placeholder array.
  trunc/truncf, round/roundf and exp2/exp2f use typed Win64-to-host-libm adapters.
  Math errno/fenv parity remains incomplete, as for existing libm adapters.
- **VCRUNTIME:** registry/vcruntime.rs explicitly exposes memcpy, memmove, memset,
  memcmp and the existing __C_specific_handler. The namespace is not globally
  aliased to MSVCRT. C++ throw/frame handlers remain absent; this is not complete
  VCRUNTIME support. C SEH behavior is shared with the existing tested unwinder.
- **Verification:** tests/test_msvc_runtime_basics.py passes PE/ELF environment
  contents/pointer stability, signed rounding, powers and signed zero.
  tests/test_vcruntime_memory.py passes PE/ELF memory return values, overlap,
  comparison signs and runtime module/C-handler discovery. Existing CRT-port,
  SEH and catalog uniqueness tests pass. Release and dist rebuilt.
- **Game result:** both actual executables now pass the environment and VCRUNTIME
  memcpy calls and stop at ntdll!NtWriteFile. Logs are
  cache/msvc-audit/*/vcruntime-startup.log. Neither game is certified playable.

## Native I/O and window-menu follow-up

- **NtWriteFile:** api/filesystem/nt_io.rs supplies synchronous current-position
  writes using existing file/pipe/standard-stream WriteFile support. IO_STATUS_BLOCK
  has the Windows x64 layout; status and actual bytes are reported, with Win32
  last-error preserved. NULL offset or FILE_USE_FILE_POINTER_POSITION (-2) works.
  Explicit offsets, append sentinel, event/APC/context and key parameters return
  STATUS_NOT_SUPPORTED. No asynchronous/IOCP behavior is claimed.
- **Menus:** api/window/menus.rs handles GetSystemMenu (NULL/ERROR_NOT_SUPPORTED
  for a valid window), GetMenu (valid windows have no application menu), and
  EnableMenuItem (-1/ERROR_INVALID_MENU_HANDLE; no menu handles are allocated).
  Invalid windows report ERROR_INVALID_WINDOW_HANDLE. No fake native menu handle.
- **Tests:** tests/test_nt_write.py passes data readback, byte counts, status,
  last-error, invalid handles/arguments and stdout in PE and ELF. Native window
  ABI/private-X11 menu checks and existing files_sync regressions pass.
- **Reference:** https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntwritefile
