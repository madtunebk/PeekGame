# Import tracking for engine bring-up

For every new engine, create `docs/engines/<engine>-imports.md` before adding APIs.
Update it in the same change as every added or modified import implementation.
The registry remains the source of callable exports; this ledger records why an
API was needed and what evidence supports its behavior.

## Engine baseline

Record the engine/version, executable identity (name and hash), renderer, runtime
revision, launch command and host test environment. Keep proprietary executables,
binaries, raw logs and secrets outside version control.

Save the static analysis report and dynamic-import trace under project cache/logs.
Static imports are not proof of execution; dynamic lookups are not proof that a
function was called. A runtime trace covers only the exercised paths.

## One entry for each import

Use the exact `DLL!Name` or `DLL!#ordinal`, including dynamically resolved APIs.
For a family implemented together, list every export explicitly. Record:

- **Trigger:** startup, rendering, input, audio, networking, saves, replay or exit;
  relevant sanitized error, caller module/RVA and local log reference.
- **Contract:** Windows signature/ABI, structure layout, flags and error behavior;
  source/documentation reference used to implement it.
- **Implementation:** source and registry paths, native backend, ownership,
  lifetime and threading rules. State whether shared or engine-specific.
- **Scope:** implemented arguments and unsupported cases with their actual
  errors. Mark partial/unsupported behavior explicitly; never infer success.
- **Verification:** exact command, fixture and observed result. Separate lookup,
  isolated API behavior and game-level evidence; identify software versus hardware
  rendering. Link the change's commit/PR when available.
- **Impact:** other engines using the shared API, focused regression results,
  remaining risks and follow-up work.

### Entry template

```markdown
## KERNEL32.dll!ExampleW
- Trigger / caller / evidence:
- Contract / reference:
- Implementation / registry / backend:
- Ownership / threading:
- Supported arguments / unsupported behavior:
- Verification command / result / environment:
- Other engines / regression evidence:
- Commit or PR / remaining work:
```

## Completion rule

An import is documented when its registry entry, implementation scope and test
result agree. Record untested behavior as untested. Update existing entries when
semantics change, including reversions. An engine reaching its menu does not
establish gameplay, save/replay, audio or shutdown support.

This rule applies to new work. Existing subsystem notes are useful evidence, but
they are not a complete historical per-import ledger.
