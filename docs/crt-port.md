# CRT parity with the earlier peek

The local runtime now resolves all **183 named exports** registered by the earlier
`src/modules/crt/msvcrt.inc`. The missing 87 were ported or connected to existing
PeekGame implementations. This is export parity with that subset, not a complete
implementation of Microsoft's CRT. Existing unsupported operations remain errors.
There is no build/runtime dependency on the earlier repository.

The saved name list is `tests/crt_legacy_exports.txt`. A Windows fixture loads
`msvcrt.dll` and verifies every name through `GetProcAddress`, including data.

## Organization

Under `src/api/crt/`:

- `math.rs`: typed Windows ABI wrappers around libm.
- `strings.rs`, `numbers.rs`, `encoding.rs`: Windows 32-bit long, UTF-16 wchar_t,
  UTF-8 narrow strings, bounded copies, classification, parsing, Microsoft's
  thread-local 15-bit random generator.
- `algorithms.rs`: in-place heapsort and binary search; Windows ABI comparators.
- `time.rs`: Windows nine-int tm, 64-bit UTC times, 1000-tick wall clock.
- `streams.rs`, `stdio.rs`, `directory.rs`: registered Windows FILE objects,
  stream limits, descriptor metadata/truncation, cwd and Windows path mapping.
- `format.rs`: legacy printf interfaces reusing the Windows va_list formatter;
  assembly entry shims handle fprintf/fwprintf arguments.
- `control.rs`, `startup.rs`: initialization, cleanup, numbered recursive locks,
  assertions and floating-point reset.
- `ucrt.rs`: pre-existing Universal CRT implementation and shared formatting.

The inherited limitations include C-locale classification, incomplete printf
conversions/options, no newline translation for CRT text streams, and partial
invalid-parameter handling. Narrow encoding is UTF-8 as in the previous runtime;
locale selection remains limited. Clock starts on first query. Unsupported
COMMON_LVB, MMCSS, shell property stores and console screen buffers are separate
API scopes, not fixed by CRT export parity.

## Verification

```sh
cargo build --release --offline
python3 tools/stage_dist.py
python3 tests/test_crt_port.py
python3 tests/test_crt_core.py
python3 tests/test_imports_abi.py
python3 tests/test_tls_threads.py
```

The port test covers all export addresses, sort/search callbacks, 32-bit overflow,
UTF-16 conversions, formatting with integers/floats/stack arguments, real files,
UTC tm layout, wall-clock units and recursive locks across Windows threads.
It also runs the CRT fixture as a relinked ELF. Focused tests cover stream limits,
64-bit sparse-file lengths, cwd and wcstombs.

## Related boot fixes

`OpenMutexW` uses process-local names with independent handle lifetimes; closing
the original no longer destroys an opened/duplicated mutex. Other processes are
outside this namespace. `SetPriorityClass` maps supported classes to host nice
values for process threads and reports permission errors; realtime is unavailable.
AVRT recognizes A/W registration, revert and priority functions, but registration
returns NULL/ERROR_NOT_SUPPORTED because there is no MMCSS backend.
