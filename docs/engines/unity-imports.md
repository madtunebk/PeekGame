# Unity import ledger

## KERNEL32.dll!GetThreadTimes

- **Trigger:** TouhouFumoRacing / UnityPlayer.dll stops at caller 0x1816276f7.
- **Contract:** Win64 BOOL with four required FILETIME outputs, 100 ns units;
  creation/exit are absolute UTC since 1601, kernel/user are CPU durations.
  [Microsoft contract](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadtimes).
- **Implementation:** `api/threading/thread_times.rs`, `threads.rs`, registry
  `threading.rs`. Reads task stat fields 14/15/22 and host boot time; current
  thread uses getrusage(RUSAGE_THREAD) for finer CPU counters. Exiting runtime
  threads retain a snapshot after FLS/TLS/DLL cleanup and before signaling exit.
  Duplicate thread handles share the snapshot. Adapted from the earlier project's
  `src/modules/threading/threads.inc` (reference only).
- **Scope/errors:** current-thread pseudo handle and runtime known/open handles;
  no remote process threads. Live exit time is zero (undefined by Windows).
  Invalid handle: ERROR_INVALID_HANDLE; null output: ERROR_NOACCESS;
  unavailable accounting: ERROR_ACCESS_DENIED. Four-byte-aligned FILETIME outputs
  use unaligned writes. Thread access masks are not currently enforced.
- **Verification:** `python3 tests/test_thread_times.py` exercises real CPU growth,
  current/worker creation times, running/finished handles, duplicate lifetime,
  stable final counters, null/invalid handles and FILETIME ABI as PE and ELF.
  Unit parser checks task names with spaces/parentheses and tick conversion.
  **PASS** for parser, PE/ELF and exit/TLS regressions. A private Xvfb/lavapipe
  Touhou boot passes GetThreadTimes, loads GameAssembly.dll and baselib.dll, then
  stops at `GetLocaleInfo: information type 0x1004 is not mapped`. Log:
  `logs/touhou-thread-times.log`. This does not establish gameplay or all imports
  from the UnityPlayer unresolved report are implemented.
- **Impact:** shared kernel32 implementation, also usable by Godot. Thread exit
  now snapshots real accounting; no engine-specific return values.

## KERNEL32.dll!GetLocaleInfoW / GetLocaleInfoEx

- **Trigger:** Touhou's IL2CPP startup stops querying type 0x1004,
  LOCALE_IDEFAULTANSICODEPAGE. This type was also absent in the old implementation.
- **Contract:** locale-specific legacy ANSI code-page metadata as a NUL-terminated
  UTF-16 decimal string; LOCALE_RETURN_NUMBER writes a DWORD and returns two WCHAR
  units. Size-only queries do not write. Negative capacities/null positive buffers
  fail ERROR_INVALID_PARAMETER; short buffers fail ERROR_INSUFFICIENT_BUFFER.
- **Implementation:** `api/system/locale.rs` and `locale/metadata.rs`, existing registry exports. Mapping
  follows the existing language table, including Romanian 1250 and Japanese 932;
  Hindi is Unicode-only (zero). This metadata does not change GetACP=65001 or add
  converters for other code pages. Explicit foreign locales remain unsupported.
- **Verification:** `python3 tests/test_locale_info.py`: C/en-US, Romanian, Japanese,
  W/Ex string and DWORD forms, sizing/invalid arguments, preserved UTF-8 process
  ACP, and relinked ELF. Game startup probe result recorded after verification.
- **Impact:** shared NLS behavior, no per-game constant or unconditional success.
  Reference: Microsoft GetLocaleInfoW / LOCALE_IDEFAULT constants and installed
  MinGW winnls.h. Existing numeric requests now receive actual DWORDs.

### Locale English names

The next IL2CPP request is LOCALE_SENGLANGUAGE (0x1001). The shared implementation
now returns English names for all LANGIDs already in its locale table; 0x1002
LOCALE_SENGCOUNTRY supports common mapped regions and neutral locale names.
Unmapped regions remain explicit unsupported cases. These are locale metadata,
not game-specific answers. The same fixture checks English/Romanian/Japanese
language and region names, and rejects LOCALE_RETURN_NUMBER on textual names.

### Currency metadata (0x15 and related types)

GetLocaleInfoW/Ex now query host ICU data for LOCALE_SCURRENCY (0x14),
LOCALE_SINTLSYMBOL (0x15), LOCALE_SMONDECIMALSEP (0x16),
LOCALE_SMONTHOUSANDSEP (0x17), LOCALE_ICURRDIGITS (0x19),
LOCALE_IINTLCURRDIGITS (0x1a), LOCALE_SENGCURRNAME (0x1007) and
LOCALE_SNATIVECURRNAME (0x1008). The adapter lives in `host/nls/`;
no ICU objects enter Windows code. This adds a host `libicu-dev` build dependency
and ICU libraries to PE runtime/relinked ELF. Values follow installed ICU/CLDR,
which can differ from a particular Windows NLS release. Other formatting types
are not inferred. Failure stays explicit. The existing fixture now verifies
USD/RON/JPY differences, real symbols/names, zero JPY versus two USD/RON fraction
digits, text/numeric output and ELF linkage.
Reference: https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/unum_8h.html

The same backend supplies LOCALE_SGROUPING (0x10), LOCALE_SMONGROUPING (0x18)
and decimal/thousands separators (0x0e/0x0f). Group sizes are translated from ICU
primary/secondary grouping to Windows `3;0` / `3;2;0` syntax. The fixture verifies
`3;0` and Romanian comma versus English/Japanese decimal point. Replacing the
previous fixed separators also affects other engines using these shared exports.

LOCALE_SPOSITIVESIGN (0x50) and LOCALE_SNEGATIVESIGN (0x51) use ICU sign symbols;
the three-locale fixture verifies their values and Win64 string return ABI.

Currency patterns: LOCALE_ICURRENCY (0x1b), LOCALE_INEGCURR (0x1c),
LOCALE_IPOSSIGNPOSN (0x52), LOCALE_INEGSIGNPOSN (0x53),
LOCALE_IPOSSYMPRECEDES (0x54), LOCALE_IPOSSEPBYSPACE (0x55),
LOCALE_INEGSYMPRECEDES (0x56), LOCALE_INEGSEPBYSPACE (0x57) derive from ICU
formatted field positions in `host/nls/pattern.c`. Currency/number/sign fields
become canonical Windows patterns; spaces normalize from Unicode whitespace.
Patterns without an exact supported Windows representation fail explicitly.
Tests cover RON's trailing spaced currency and USD/JPY's leading currency,
plus supported positive/negative pattern indices; sign-position variants are
implemented but are not individually covered by this fixture yet.

Calendar strings: LOCALE_SDAYNAME1–7 (0x2a–0x30),
LOCALE_SABBREVDAYNAME1–7 (0x31–0x37), LOCALE_SMONTHNAME1–12 (0x38–0x43),
LOCALE_SABBREVMONTHNAME1–12 (0x44–0x4f), 13th month names (0x100e/0x100f),
and LOCALE_S1159/S2359 (0x28/0x29) use ICU calendar symbols in
`host/nls/calendar.c`. Monday-based Windows indices are translated to ICU's
Sunday-based indices. Gregorian 13th-month names are legitimately empty.
Fixture checks actual Monday/Sunday and January names, localized queries,
13th-month emptiness and AM text. Locale calendar differences follow host ICU;
this is not full Windows calendar API support.

Date patterns: LOCALE_SDATE (0x1d), STIME (0x1e), SSHORTDATE (0x1f),
SLONGDATE (0x20), IDATE/ILDATE (0x21/0x22), ITIME (0x23), ICENTURY (0x24),
ITLZERO (0x25), IDAYLZERO (0x26), IMONLZERO (0x27), STIMEFORMAT (0x1003),
ITIMEMARKPOSN (0x1005), SYEARMONTH (0x1006), SMONTHDAY (0x78) and SSHORTTIME
(0x79) derive from ICU patterns in `host/nls/date_pattern.c`. Weekday, AM/PM,
year and era field letters are translated to Windows syntax; unknown fields fail
explicitly. Locale formats follow installed CLDR, not exact Windows NLS presets.
The fixture checks short/long date, time AM/PM adaptation and year-month patterns.

Calendar numeric metadata: LOCALE_ICALENDARTYPE (0x1009),
IFIRSTDAYOFWEEK (0x100c) and IFIRSTWEEKOFYEAR (0x100d) come from the actual ICU
calendar type/attributes. Known calendar identifiers are mapped to Windows CALID;
unknown types or week rules fail. Tests check Gregorian ID and US/Japan Sunday
versus Romanian Monday using numeric output. Year-length metadata follows the
translated Windows pattern, including ICU single-y to Windows yyyy expansion.

### Verified NLS bring-up result

`python3 tests/test_locale_info.py` passes C/en-US, Romanian and Japanese PE runs
and an en-US relinked ELF with ICU linked. `logs/touhou-locale.log` now shows
Touhou passing its exercised GetLocaleInfo startup sequence; the next stop is
**ole32.dll!CoGetContextToken**. This verifies the NLS queries reached in that
startup, not all Windows locale APIs or all .NET cultures. Native locale data
requires the installed ICU libraries at runtime (Ubuntu `libicu-dev` to build).
Date/number user overrides, arbitrary foreign locale selection and complete
Windows-versus-CLDR parity remain outside this implementation's scope.

## ole32.dll!CoGetContextToken

- **Trigger:** Touhou GameAssembly.dll +0x294e5f queries its thread apartment.
  The caller branches on failed HRESULT; success dereferences the token and
  queries IComThreadingInfo. A numeric sentinel would crash.
- **Contract/reference:** Win64 HRESULT(ULONG_PTR*).
  https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cogetcontexttoken
  Success requires an actual IObjContext pointer; it is not an integer ID.
- **Implementation:** `api/com/context.rs`, current apartment state in
  `apartment.rs`, explicit partial registry entry in `registry/com.rs`.
- **Scope/errors:** null output E_POINTER; uninitialized calling thread
  CO_E_NOTINITIALIZED. Initialized apartments return E_NOTIMPL because no real
  context object/vtable is implemented yet. Output is cleared on failure. No
  fabricated S_OK, object, AddRef or IComThreadingInfo. Implicit MTA/context
  marshalling behavior is not implemented by this logical apartment model.
- **Verification:** `python3 tests/test_com_context.py` tests PE and ELF, null
  pointers, nested STA initialization, changed-mode errors, independent MTA worker
  state and OleInitialize/OleUninitialize. Other engines share the same COM state.
  **PASS** for PE/ELF fixture. Private Touhou startup passes this call, reaches
  WASAPI initialization (48000 Hz stereo PCM16), and next stops at
  KERNEL32.dll!GetThreadLocale. Log: `logs/touhou-locale.log`. FMOD logs an
  initial device error before its retry reaches WASAPI; audible game output
  and full startup are not established by this probe.

## KERNEL32.dll!GetThreadLocale / SetThreadLocale

- **Trigger:** Touhou IL2CPP calls GetThreadLocale after COM/NLS/audio startup.
- **Contract:** current-thread LCID and setter; getter/success preserve last error.
  New Windows threads inherit the creator's LCID, captured at CreateThread rather
  than ResumeThread. Reference:
  https://learn.microsoft.com/en-us/windows/win32/api/winnls/nf-winnls-getthreadlocale
- **Implementation:** small `api/system/locale/thread.rs` module, creation hook in
  `api/threading/threads.rs`; x64 TEB.CurrentLocale at 0x108 initialized and updated.
  No process-global locale mutation or dependency on the earlier project.
- **Scope:** existing known LCID table, invariant, user/system-default aliases;
  unknown LCIDs fail ERROR_INVALID_PARAMETER, custom LCIDs ERROR_NOT_SUPPORTED.
  This tracks thread locale, not UI language. Arbitrary foreign-locale formatting
  and resource selection are not added; CP_THREAD_ACP retains the existing UTF-8
  runtime policy. It does not expand GetLocaleInfo's explicit locale coverage.
- **Verification:** `python3 tests/test_thread_locale.py` checks host defaults,
  last error, TEB agreement, independent changes, nested/suspended inheritance,
  default aliases/errors and PE/ELF. Thread changes are shared across engines;
  existing TLS/CRT-exit regressions are required before structural refactoring.

Thread-locale verification **PASS** as PE/ELF, including inherited LCID in a
suspended child and nested child. Existing TLS and _endthreadex regressions pass.
Touhou on private lavapipe now passes GetThreadLocale and stops on integer division
by zero at DXGI +0xe44b, recorded in `logs/touhou-locale.log`; a later user hardware
run creates the swapchain, loads scene data and reaches RtlCaptureStackBackTrace.
The declarations moved unchanged to `registry/system/nls.rs` during stage 1.

## KERNEL32.dll!RtlCaptureStackBackTrace / ntdll.dll!RtlCaptureStackBackTrace

- **Trigger:** user hardware Touhou run reaches scene loading, then GameAssembly
  requests stack capture. Neither project had an implementation.
- **Implementation:** `seh/backtrace.rs`, existing x64 capture-context assembly,
  PE/JIT function-table lookup and RtlVirtualUnwind. The Win64 assembly entry
  captures the actual Windows caller before entering Rust; CONTEXT is zeroed and
  aligned, nonvolatile state remains preserved. No host backtrace pointers are
  substituted for Windows frames. Registry: `system/exceptions.rs`.
- **Scope:** skip/count, optional wrapping address-sum DWORD hash, caller stack
  bounds, leaf frames and PE/JIT metadata. Stops at native host boundary or invalid
  progress, bounded to 4096 walked frames; not a mixed Linux/Windows unwinder.
  Existing unwinder assumes valid PE unwind metadata. No context/exception changes.
- **Verification:** `python3 tests/test_stack_backtrace.py` compiles without frame
  pointers and checks exact parent return addresses, frame ordering, skip/count,
  hash, zero capture, excessive skip, main/worker stacks, PE and ELF.
  Full hardware Touhou replay remains user verification.

Stack-capture verification **PASS** as PE and relinked ELF. Existing SEH regression
and export catalog uniqueness/resolution test also pass. Release rebuilt and
staged in `dist/`; hardware Touhou continuation is not yet verified.
