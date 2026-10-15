# Future development: optional decompilers

Recorded for later work. These tools are candidates for developer diagnostics;
no installation or integration is implemented by this note.

## ILSpy: managed .NET and Unity code

Project: https://github.com/icsharpcode/ILSpy

ILSpy browses .NET assemblies and decompiles managed code to C#. Its command-line
frontend, ILSpyCmd, could support an optional analysis step for Unity Mono builds.

Potential PeekGame uses:

- Inspect game assemblies under `*_Data/Managed/`.
- Investigate `PlayerNametag.LateUpdate` exceptions and replay save/cleanup logic.
- Inspect managed declarations of native imports (`DllImport`/PInvoke) and their
  argument types, calling conventions, and error-handling expectations.
- Follow managed call paths involved in networking, input, and filesystem access.

Managed import declarations can supplement our PE import inventory. Their presence
does not establish that a function was called during a particular session. ILSpy
is not the tool for ordinary native x86-64 code such as our C testgame or a native
UnityPlayer DLL. Unity IL2CPP builds need a different analysis workflow.

## RetDec: native PE code

Project: https://github.com/avast/retdec

RetDec supports x86-64 PE binaries, disassembly, reconstruction of functions and
C-like code, and call/control-flow graphs.

Potential PeekGame uses:

- Inspect native EXE/DLL callers around a failing API or crash location.
- Explore unfamiliar functions and indirect call paths.
- Compare reconstructed argument handling with Microsoft API contracts.
- Supplement native compiler, symbol, and binary metadata analysis.

Treat recovered types and code as hypotheses. Verify findings against machine
instructions and runtime observations. RetDec's output does not automatically
produce a correct Linux executable or recover the original source exactly.

## Possible integration

Keep optional tools in separate developer adapters, for example
`tools/analysis/ilspy/` and `tools/analysis/retdec/`, with their own documented
installation steps. Those directories do not exist yet.

Adapters could write supplementary reports beside `imports.json`/`imports.txt`.
Record the tool version, input binary identity, and original method token or
module-relative address so a finding can be reproduced on the same build.
Clearly distinguish static findings, decompiler inferences, runtime export
requests, and observed calls; keep recovered output separate from our API code.

Do not make either tool a requirement for launching packaged games. Prioritize
the native window, graphics, loader, and import-binding work before integration.
Use Microsoft documentation and headers to define Windows API behavior; use
these tools to investigate concrete caller behavior when that evidence is needed.
