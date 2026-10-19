# Unreal Engine import ledger

## Baseline

- **Game:** Palworld (Unreal Engine 5), Steam AppID 1623730,
  `Pal/Binaries/Win64/Palworld-Win64-Shipping.exe` (161802312 bytes). `Palworld.exe`
  is a launcher that starts the shipping executable as a child process (unsupported),
  so the shipping executable is run directly with `-dx11` (DXVK translates D3D11 only;
  the default is D3D12).
- **Launch:** `./dist/peekgame <Palworld>/Pal/Binaries/Win64/Palworld-Win64-Shipping.exe --run -dx11`
  Log: `logs/Palworld/pal.log`.
- **Where it stops:** `int3` at `+0x9fda0ec` inside the `.bind` section (the entry
  point): SteamStub DRM's launch check. **Stopped by policy**: DRM is never bypassed,
  patched or answered (same as re3.exe).

## MSVCP140 / VCRUNTIME140 (Microsoft's, local only)

- **Trigger:** 210 imports of MSVCP140.dll (C++ standard library), which games expect
  installed system-wide; PeekGame's own VCRUNTIME140 lacks __CxxFrameHandler4, RTTI and
  type_info helpers that UE5 needs.
- **Implementation:** `tools/extract_msvc_redist.py [--with-vcruntime]` carves the CABs
  out of Steam's local `VC_redist.x64.exe` and extracts the x64 `msvcp140*.dll` (and with
  the flag `vcruntime140*.dll`) into `windows-dlls/` (gitignored; checksums in
  `MSVC-REDIST-PROVENANCE.json`); `tools/stage_dist.py` copies them beside the runner.
  With `--with-vcruntime` Microsoft's VCRUNTIME replaces ours for every program that
  imports it.
- **UCRT additions for them** (`src/api/crt/cxx_support.rs`): _lock_locales/_unlock_locales
  (recursive), ___lc_locale_name_func, ___lc_collate_cp_func, __pctype_func (C ctype
  table), _wsetlocale (C only), isupper/islower, __strncnt, wcsnlen, _wcsdup,
  strtod/strtof/wcstod/wcstof, atol (32-bit), _set_new_handler/_callnewh,
  _seh_filter_dll, __stdio_common_vsprintf_s, _dclass/_fdclass/_dsign/_fdsign, copysign,
  frexp, lgamma/lgammaf/lgammal, _wremove/_wrename/_wrmdir, _fsopen,
  _get_stream_buffer_pointers (pointers into the FILE layout; host-buffered streams
  report an empty buffer).
- **Still unresolved in msvcp140 (stop if called):** time-name tables (_Getdays,
  _Getmonths, _Gettnames, _Strftime, _Wcsftime and W variants), threadpool work/wait/timer,
  CreateFile2, CopyFile2, CreateSymbolicLinkW, CreateHardLinkW.
- **Verification:** Palworld: vcruntime140, vcruntime140_1 and msvcp140 complete
  DLL_PROCESS_ATTACH; the program's TLS callbacks run; then SteamStub. MarioVsLuigi and
  DOOM (which do not load vcruntime140) unchanged; crt_core, crt_port,
  msvc_runtime_basics, seh, `cargo test` pass.

## USER32 GetQueueStatus (CEF in Palworld)

- **Trigger:** Palworld (UE5) now gets past SteamStub. The stub runs unmodified against the real Steam client, now that the command line names the exe by its Windows path. It loads Wwise and CEF, then stops at `STOP: missing Windows API USER32.dll!GetQueueStatus`, called from libcef.dll's message pump.
- **Contract:** the high word holds the QS_* kinds currently queued for the calling thread; the low word holds the kinds added since the last GetQueueStatus/GetMessage/PeekMessage. Both are limited to the flags, and the reported "added" bits are cleared. Unknown flags give ERROR_INVALID_FLAGS.
- **Implementation:** `src/api/window/queue.rs`. Posted messages record their kind (plus QS_ALLPOSTMESSAGE); sent messages record QS_SENDMESSAGE; due timers give QS_TIMER; windows needing paint give QS_PAINT. Retrieval clears the "added" bits.
- **Verification:** `tests/test_queue_status.py`.
- **Note:** libcef.dll has 398 unresolved static imports (GDI, menus, DDE, clipboard, ...). They are handled when a STOP shows they are reached, not ahead of time.

## KERNEL32.dll!CreateJobObjectW

- **Trigger:** `STOP: missing Windows API KERNEL32.dll!CreateJobObjectW`, called from
  `libcef.dll` (the CEF3 web helper bundled under
  `Engine/Binaries/ThirdParty/CEF3/Win64/`) during Palworld's Epic/EOS web view
  startup. Chromium uses a job object to group and sandbox its child processes.
- **Contract:** [CreateJobObjectW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createjobobjectw):
  returns a handle to a new or existing named job object; `lpName` names it,
  `lpJobAttributes` is a SECURITY_ATTRIBUTES. Differs from `CreateJobObjectA` only
  in the name's string encoding.
- **Implementation:** `process::create_job_object_w` in `src/api/system/process.rs`,
  registry `src/registry/system/process.rs`. Jobs are process-local tokens backed by
  an event handle; the name is ignored, so the W variant shares the ANSI variant's
  body. Pairs with the existing `SetInformationJobObject` / `AssignProcessToJobObject`.
- **Scope/errors:** returns an owned handle; job limits and process assignment are
  *accepted* but not enforced (no real cgroup/process grouping). Named reuse
  (ERROR_ALREADY_EXISTS) is not tracked. Honest `partial`, matching `CreateJobObjectA`.
- **Verification:** `./dist/peekgame --analyze libcef.dll` →
  `KERNEL32.dll!CreateJobObjectW [backend: partial]` (was `unknown`). The STOP no
  longer fires; game-level progress past the CEF init pending a re-run.
- **Impact:** shared with any CEF-based program (libcef) and any Unreal title using
  the Epic web helper. No behavior change to the A variant or the job stubs.

## CRYPT32.dll!CryptInstallOIDFunctionAddress

- **Trigger:** `STOP: missing Windows API CRYPT32.dll!CryptInstallOIDFunctionAddress`,
  called from `libcef.dll` during the same CEF3 web-helper startup (Chromium
  registers OID handler functions while setting up certificate processing). Not
  the Steam x64 overlay — the import is in libcef, which also pulls
  `CryptDecodeObjectEx`.
- **Contract:** [CryptInstallOIDFunctionAddress](https://learn.microsoft.com/en-us/windows/win32/api/wincrypt/nf-wincrypt-cryptinstalloidfunctionaddress):
  `(hModule, dwEncodingType, pszFuncName, cFuncEntry, rgFuncEntry, dwFlags)` →
  BOOL. `rgFuncEntry` is an array of `CRYPT_OID_FUNC_ENTRY { pszOID, pvFuncAddr }`,
  where `pszOID` may be a numeric OID id (high 16 bits zero) rather than a string.
- **Implementation:** `crypt_strings::install_oid_function_address` in
  `src/api/network/crypt_strings.rs`, registry `src/registry/network.rs`. Records
  each call (encoding type, function name, entry pointers) in a process-global
  table. `pszOID` is stored as a raw value and never dereferenced. Logs under
  `PEEKGAME_TRACE_CRYPTO`.
- **Scope/errors:** NULL `pszFuncName`, or a non-zero count with a NULL array,
  give ERROR_INVALID_PARAMETER (87) and FALSE; otherwise the entries are recorded
  and it returns TRUE. **Honest limit:** entries are recorded only — PeekGame's own
  CRYPT32 encode/decode paths (e.g. the `CryptDecodeObjectEx` stub) do **not** yet
  dispatch to installed handlers, so custom OID processing is not actually active.
  CEF does not import `CryptGetOIDFunctionAddress`, so nothing retrieves them here.
- **Verification:** `./dist/peekgame --analyze libcef.dll` →
  `CRYPT32.dll!CryptInstallOIDFunctionAddress [backend: partial]` (was `unknown`);
  build + clippy clean (no new warnings). Game-level progress past CEF pending a re-run.
- **Impact:** shared with any CEF/Chromium-based program. No change to existing
  CRYPT32 entries.

## CRYPT32.dll!CertGetCertificateChain — strong-signature policy (CEF TLS)

- **Trigger:** every TLS handshake in the CEF web helper failed. A
  `PEEKGAME_TRACE_CERTS` run showed `CertGetCertificateChain engine=0,
  flags=0xa0000000, parameter size=96` then `rejected (error 50): unsupported
  CERT_CHAIN_PARA field pStrongSignPara (offset 80) is set`, and Chromium mapped
  the Windows error 50 to `net::ERR_FAILED` (`cert_verify_proc_win.cc:105`),
  aborting the handshake (`ssl_client_socket_impl.cc:947`, SSL error 1).
- **Contract:** `CERT_CHAIN_PARA.pStrongSignPara` points to a `CERT_STRONG_SIGN_PARA`
  { cbSize, dwInfoChoice, union { pvInfo / pszOID } }; `dwInfoChoice==2`
  (`CERT_STRONG_SIGN_OID_INFO_CHOICE`) with `pszOID = szOID_CERT_STRONG_SIGN_OS_1`
  ("1.3.6.1.4.1.311.72.1.1", == OS_CURRENT) means "reject weak signatures" — SHA-2
  only, RSA >= 2048. `dwStrongSignFlags==0` enforces over the whole chain. Chromium
  sets exactly this on every verification.
- **Implementation:** previously the whole strong-sign area was blanket-rejected
  with ERROR_NOT_SUPPORTED (50). Now `chain::strong_sign` in
  `src/api/network/certificates/chain.rs` parses it: the well-known OS OID with
  `dwStrongSignFlags==0` returns "require strong", which `openssl::verify`
  (`certificates/openssl.rs`) honors by setting the `X509_STORE_CTX` auth level to
  2 (`X509_VERIFY_PARAM_set_auth_level`) — OpenSSL then rejects SHA-1/MD5 signatures
  and sub-2048-bit RSA during chain verification. The policy function
  (`CertVerifyCertificateChainPolicy`) passes `require_strong = false`.
- **Scope/errors:** honored only for the well-known OS strong-sign OID with flags 0.
  A serialized custom `CERT_STRONG_SIGN_SERIALIZED_INFO` (`dwInfoChoice==1`), any
  other OID, or a non-zero `dwStrongSignFlags` still returns 50 — we do not guess an
  unknown policy or under-enforce a selective exemption. Issuance-policy, URL
  timeout and revocation-freshness fields remain ERROR_NOT_SUPPORTED. Auth level 2
  is a faithful approximation of OS_CURRENT, not a byte-exact reimplementation.
- **Verification:** `PEEKGAME_TRACE_CERTS` build; the blanket `pStrongSignPara`
  rejection no longer fires. Build + clippy clean (no new warnings). Note: separate
  `crypt32: API failure 0x80092004` (CRYPT_E_NOT_FOUND) lines are property probes,
  not this failure.
- **Serialized strong-sign form (2026-10-18, done — live confirmation pending):** a
  re-run showed the error 50 was coming from inside `chain::strong_sign`, not the
  blanket reject — Chromium sends the **serialized** form (`dwInfoChoice==1`,
  `CERT_STRONG_SIGN_SERIALIZED_INFO`), not the OID. `strong_sign` now parses that
  struct at the union pointer (`dwFlags @0, pwszCNGSignHashAlgids @8,
  pwszCNGPubKeyMinBitLengths @16`, wide via `host::paths::wide`): if every listed
  `ALG/HASH` pair uses SHA-256/384/512 it maps to OpenSSL auth level 2
  (`require_strong = true`); a list that still permits SHA-1/MD5, an empty list, a
  non-zero `dwStrongSignFlags`, or an unknown `dwInfoChoice`/OID still returns 50.
  Every Err(50) path now logs its reason under `PEEKGAME_TRACE_CERTS`. Build +
  clippy gate clean. **Next:** re-run and confirm the CEF handshake reaches
  `net_error 0`; if OpenSSL now rejects a real chain at level 2, the trace + OpenSSL
  error will show it and we revisit the key-length / level mapping.
- **Impact:** shared — any program using `CertGetCertificateChain` with the OS
  strong-sign policy (common in Chromium). The default (no strong-sign) path and
  `CertVerifyCertificateChainPolicy` are unchanged; this only *adds* enforcement
  when the policy is requested, so it cannot make verification more permissive.

## SteamUser021 auth ticket forwarding (Palworld online login)

- **Trigger:** Palworld reports `ConnectLoginNoEAS(0) failed due to the platform OSS giving an empty auth token`.
- **Contract:** `ISteamUser021::GetAuthSessionTicket` passes the remote `SteamNetworkingIdentity` as its fourth argument to the real Steam client. Earlier bridged SteamUser versions retain their three-argument form.
- **Implementation:** `native/steam_bridge/steam_bridge.c` forwards the fourth argument for SteamUser021 and logs only ticket handle and length when `STEAM_BRIDGE_TRACE=1`.
- **Verification:** bridge ABI test checks that the fourth pointer reaches the Linux SteamUser021 object and that ticket handle, length, and bytes return through the Windows ABI. A live Palworld trace in `/tmp/goagal_in_actiune.txt` shows the SteamStub ownership ticket (196 bytes), but no `GetAuthSessionTicket` call; online login remains unresolved. It repeatedly requests `SteamNetworking006` and `SteamMatchMakingServers002` during `SteamAPI_Init`; both formerly returned NULL. The bridge now forwards those interface requests to Linux Steam and wraps returned objects so initialization can proceed. Their method slots still trap if called and need implementation from an observed call. The fake-client test verifies both wrappers; live validation is pending because the Steam client was not running during this change.
- **Other engines:** older SteamUser interface versions continue using their existing signature.

## Palworld networking interface requests before CrashReportClient

- **Observation:** the main process requested `SteamNetworkingSockets012` and `SteamNetworkingUtils004`, received NULL, then raised `EXCEPTION_ACCESS_VIOLATION` reading address zero at `Palworld-Win64-Shipping.exe+0x15feedc`. It launched CrashReportClient, where `dbgeng.dll!DebugCreate` was the later STOP. The exact null object's identity is not yet proven.
- **Implementation:** `native/steam_bridge/steam_bridge.c` now forwards both interface requests to Linux Steam and returns Windows ABI wrappers. Method calls still stop with a slot diagnostic until their signatures are implemented.
- **Verification:** fake Steam client checks both wrappers. The real Steam client was not running for an isolated host probe; live confirmation is pending.
- **Next live trace:** the real Linux Steam client returned a non-NULL `SteamNetworkingSockets012` object. Palworld called slot 22, which is `InitAuthentication` in Valve's version 012 header. The bridge now forwards `InitAuthentication` and adjacent `GetAuthenticationStatus` to the Linux object, logging their result codes with `STEAM_BRIDGE_TRACE=1`. The fake-client ABI test checks both calls. Live result is pending.
- **Following trace:** `InitAuthentication` returned 3, then Palworld called `SteamNetworkingUtils004` slot 13. Valve's version 004 header identifies it as `SetDebugOutputFunction`; the bridge now adapts its callback between SysV and Windows x64 calling conventions. The fake-client test verifies callback level and message. `openvr_api.dll` GetModuleHandleEx probes returned false later in the trace, but were not the first STOP.

## GPU at 100% in camera/gameplay — uncapped IMMEDIATE swapchain (quirk: prefers_vsync)

- **Affected API:** `vulkan-1.dll!vkCreateSwapchainKHR` (DXVK → the thunk in
  `src/api/graphics/vulkan.rs`).
- **Symptom:** Unreal games (Palworld) hold the GPU near **100%** with screen tearing,
  even when not GPU-limited — the card renders far more frames than the display shows.
- **Cause:** with vsync off and no effective frame limit, DXVK creates the swapchain
  with `VK_PRESENT_MODE_IMMEDIATE_KHR` (confirmed in `logs/palworld.log`:
  `Present mode: VK_PRESENT_MODE_IMMEDIATE_KHR`, image count 5). PeekGame forwarded it
  unchanged, so the GPU rendered flat out.
- **Implementation:** when no `PEEKGAME_VULKAN_PRESENT` override is set and the game is
  an Unreal shipping build (`quirks::prefers_vsync()` → `is_unreal()`, EXE named
  `*-Win64-Shipping.exe`), and the game requested an uncapped mode (IMMEDIATE/MAILBOX),
  `create_swapchain` substitutes **FIFO** (vsync) and trims `minImageCount` to **3**
  (short, low-latency queue). The GPU then paces to the refresh with no tearing and no
  loss of resolution/settings. FIFO/relaxed requests are left untouched.
- **Scope:** present pacing only; no change to rendered image quality. Overridable with
  `PEEKGAME_VULKAN_PRESENT` (e.g. `=immediate` to restore uncapped, `=mailbox` for
  low-latency no-tear) and `PEEKGAME_VULKAN_IMAGES`; `PEEKGAME_VULKAN_FPS` still caps
  any mode. A game genuinely GPU-bound at the refresh rate still shows high GPU — that
  is real work, not waste.
- **Verification:** `cargo test --offline quirks` (`unreal_shipping_builds_are_recognised`).
  In-game: `PEEKGAME_TRACE_GL=1` logs `present mode 0 -> 2 (vsync default)`; GPU drops
  from ~100% to what the target frame rate needs.

## RESOLVED — choppy mouse in camera look (raw input now from XInput2 raw motion)

- **Symptom:** mouse felt very choppy/"sacadat" during Unreal (Palworld) camera
  look.
- **Cause:** the raw-input path synthesized relative deltas from X11 **core**
  `MotionNotify` (absolute root coords, `delta = x - last`) plus warp-recentering.
  Core motion is accelerated by the desktop pointer curve (quantizes slow movement →
  steppy), coalesced by the server (update rate, not the device report rate), and the
  recenter warp adds an X round-trip. Windows RawInput, which Unreal expects,
  delivers unaccelerated, uncoalesced, relative device deltas.
- **Trigger:** `RegisterRawInputDevices` for the generic-desktop mouse (usage page 1,
  usage 2), then `WM_INPUT` / `GetRawInputData` read each move during camera look.
- **Contract:** [Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/raw-input):
  `RAWMOUSE.lLastX/lLastY` are relative device counts (`MOUSE_MOVE_RELATIVE`),
  independent of the desktop pointer and its acceleration.
- **Implementation:** XInput2 raw motion. `Connection::open` queries the extension and
  `XISelectEvents` for `XI_RawMotion` on the root from all master devices
  (`src/host/x11/mod.rs`, `init_raw_motion`); the event reader decodes the
  `XIRawEvent` cookie's unaccelerated `raw_values` (`src/host/x11/events.rs`,
  `raw_motion`) into `NativeEvent::RawMotion`; the pump routes it to the focused game
  via `input::raw_mouse_relative` (`src/api/window/input/raw.rs`). The XI2 stream owns
  the WM_INPUT deltas **only during relative capture** (cursor hidden/blank and the
  window focused — `cursor::relative_capture`); the core-pointer path
  (`raw_motion_core`) keeps generating deltas otherwise and when XInput2 is absent, so
  exactly one source emits and neither double-counts. XI2 bypasses edge clamping, so
  the recenter warp no longer feeds the deltas — it only keeps a hidden pointer off
  the screen edge for games that do not `ClipCursor`. A sub-count residual accumulator
  avoids dropping slow motion. FFI in `src/host/x11/ffi.rs` (linked `-lXi`; also added
  to the relinked-ELF `HOST_LIBS` in `src/relink.rs`).
- **Scope:** relative mouse motion (camera look). Buttons and wheel are unchanged.
  `PEEKGAME_NO_XI2` forces the old core-pointer path for A/B comparison or a server
  that mishandles XInput2.
- **Verification:** `tests/window_input.c` `check_raw_capture` enters relative capture
  (`ShowCursor(FALSE)` + input focus), injects real relative motion with
  `XTestFakeRelativeMotionEvent(10,12)`, and asserts the `WM_INPUT` delta is exactly
  `(10,12)` — unaccelerated and 1:1 (run via `tests/test_window_abi.py`, PE and
  relinked ELF). In-game: `PEEKGAME_TRACE_INPUT=1 ./dist/peekgame
  .../Palworld-Win64-Shipping.exe --run 2>&1 | grep -i raw` during slow camera look
  shows a smooth fine-grained delta stream, not 0-then-large bursts.
- **Impact:** shared win — any game using RawInput mouse (most FPS/TPS camera look),
  not Unreal-specific; applies equally to id Tech 7 (DOOM Eternal). Touches
  `src/host/x11/{ffi,types,mod,events}.rs`, `src/api/window/{pump,input}.rs` and
  `src/api/window/input/{raw,cursor}.rs`, and `src/relink.rs`.

## Held mouse buttons stopped camera motion (2026-10-19)

- **Trigger:** hold a mouse button during hidden-cursor camera look. The private
  Xvfb regression received the right-button press and core pointer movement, but
  no `WM_INPUT` motion until the button was released.
- **Contract:** [XInput2 protocol, version 2.1 changes](https://cgit.freedesktop.org/xorg/proto/inputproto/tree/specs/XI2proto.txt?h=multitouch-devel)
  makes raw events independent of pointer grabs. The previous 2.0 negotiation
  suppressed the root raw-motion stream during the implicit button grab.
- **Implementation:** `src/host/x11/mod.rs` requests XI 2.1 and falls back to core
  pointer deltas if that version is unavailable. `ClipCursor`'s `XGrabPointer`
  uses `owner_events=False`: with True, the captured test received each raw move
  twice. This matches the [SDL grab fix](https://discourse.libsdl.org/t/sdl-x11-fix-duplicate-xinput2-event-reception/42771).
- **Verification:** `cargo build --release --offline`; `tests/test_window_abi.py`
  on private Xvfb with XI2 and with `PEEKGAME_NO_XI2=1`. The test uses XTest to
  hold right, left, middle and both side buttons simultaneously, releases them
  independently, checks legacy messages, raw button flags and `GetAsyncKeyState`,
  and verifies exact motion after every transition. It repeats under
  `ClipCursor` + `SetCapture`. Before the fix, motion failed on the first held
  button; changing only the XI version exposed doubled motion under clipping.
  The completed fix passes both modes. The fallback test explicitly places the
  cursor in the window and initializes its delta anchor before injection.
- **Scope and impact:** shared X11 input, including Unreal and id Tech games.
  Overlapping button events passed the regression; the reproduced defect was
  camera motion freezing while a button was held. Actual gameplay has not been
  rechecked. Release runtime staged with `tools/stage_dist.py`.
