# DirectWrite system fonts

`src/api/text/dwrite/` provides `DWriteCreateFactory` through a Rust export and
an independent local C/fontconfig backend adapted from the owner's earlier peek.
It does not include files or load libraries from the previous project.

Implemented: factory versions 0–2, system collection, family/name enumeration,
closest face selection, font-file paths and local loaders, character coverage
fallback using fontconfig, and number-substitution objects. COM callbacks use the
Windows x64 ABI; strings use UTF-16. Collection objects are retained for the
process lifetime. Isolated factories have distinct identities and share the
read-only system collection.

Not implemented: text layout/shaping/rendering, glyph metrics/tables, custom font
collections/loaders, and newer factory interfaces. Unsupported methods stop with
the interface and method name; unsupported IIDs return `E_NOINTERFACE`.

Build requires `cc`, `ar`, `pkg-config`, and fontconfig development headers and
libraries. Cargo builds the local adapter through `build.rs`. The ELF relinker
also links `libfontconfig`.

Verification:

```sh
cargo build --release --offline
python3 tools/stage_dist.py
python3 tests/test_dwrite_fonts.py
```

The test walks actual host font files through Windows COM vtables, checks buffer
errors, number substitution, factory isolation and the fallback interface, both
under the PE loader and as a relinked ELF. Gundam Survivors passed font discovery
(247 families on this host); its next missing import was `msvcrt!_setmaxstdio`.
