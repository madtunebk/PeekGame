# id Tech 7 import ledger

## Baseline

- **Game:** DOOM Eternal 1.0.2 (id Tech 7.1.1, build `20260811-165453-brown-sapphire`),
  Steam AppID 782330, `DOOMEternalx64vk.exe`
  sha256 `69dc13e88d1c19133ead7950dc64ebcbd4a5a3f6bd6f9c336ebffe56df6a1c11`.
- **Renderer:** the game's own Vulkan renderer through `vulkan-1.dll` (no DXVK);
  host NVIDIA GeForce RTX 3060, driver 595.99.02, on the desktop X server.
- **Launch:** `./dist/peekgame ~/.steam/debian-installation/steamapps/common/DOOMEternal/DOOMEternalx64vk.exe --run`
  with Steam running. Log: `logs/DOOMEternalx64vk/doom.log` (overwritten per run).
- **Where it stops now:** `WINHTTP.dll!WinHttpSetOption`, after resource streaming,
  the Bink boot video load, swapchain creation and decl/material loading. Not a
  claim of menu, gameplay or shutdown support.

## bcrypt.dll — hashes, HMAC and AES

`BCryptOpenAlgorithmProvider`, `BCryptCloseAlgorithmProvider`, `BCryptGetProperty`,
`BCryptSetProperty`, `BCryptCreateHash`, `BCryptHashData`, `BCryptFinishHash`,
`BCryptDuplicateHash`, `BCryptDestroyHash`, `BCryptGenerateSymmetricKey`,
`BCryptDuplicateKey`, `BCryptDestroyKey`, `BCryptEncrypt`, `BCryptDecrypt`

- **Trigger:** `FATAL ERROR: Failed to read default.cfg` after
  `binaryFile:default.cfg ... load binary failed with CORRUPT`. A file trace showed
  the bytes read from `gameresources_patch3.resources` intact; they are encrypted
  `binaryFile` data (AES-128-CBC + HMAC-SHA256). The game opens `SHA256` with
  `BCRYPT_ALG_HANDLE_HMAC_FLAG`; the previous code ignored the flag and the hash
  secret (plain SHA-256), so the tag never matched.
- **Contract:** [CNG primitives](https://learn.microsoft.com/en-us/windows/win32/seccng/cng-cryptographic-primitive-functions):
  HMAC when the provider has the HMAC flag (secret = key); `BCryptFinishHash`
  needs cbOutput equal to the digest length; CBC/ECB pad only with
  `BCRYPT_BLOCK_PADDING`, otherwise whole blocks; CBC writes the last ciphertext
  block back to pbIV; NULL output is a size query.
- **Implementation:** `src/api/system/random.rs`, registry
  `src/registry/system/security.rs`. OpenSSL EVP digests and ciphers; HMAC built
  from ipad/opad over EVP. Keys copy the provider's chaining mode and keep their
  own IV state for calls with no IV. Handles are process-local tokens; caller
  hash/key object buffers are not used. Shared, not engine-specific.
- **Scope/errors:** RNG, AES (128/192/256; CBC, ECB, GCM), SHA-1/256/384/512, MD5.
  Other algorithms STATUS_NOT_FOUND (previously silently SHA-256); properties
  other than ChainingMode are STATUS_NOT_SUPPORTED on set (previously accepted).
  Chained GCM calls (`BCRYPT_AUTH_MODE_CHAIN_CALLS_FLAG`) STATUS_NOT_SUPPORTED.
  Hash handles are always reusable after finish. `PEEKGAME_TRACE_CRYPTO` logs calls.
- **Verification:** `python3 tests/test_bcrypt.py` — SHA-256 + duplicate, HMAC
  (chunked, reuse, long key) against Python `hmac`; AES-CBC padded/unpadded,
  chained IV, in-place decrypt and ECB against `openssl enc`; GCM round trip and
  tag mismatch; size/buffer errors; unknown algorithm/property. **PASS**. Game:
  `default.cfg`, `package.cfg` and `strings/english.blang` (13782 strings) load.
- **Impact:** any program using CNG; RNG behavior unchanged.

## KERNEL32.dll!ReadFileScatter

- **Trigger:** STOP at caller 0x14037e64e right after `english.blang` (resource
  streaming).
- **Contract:** [ReadFileScatter](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfilescatter):
  overlapped no-buffering handle, FILE_SEGMENT_ELEMENT array of page-sized
  buffers, length a multiple of the page size, offset from OVERLAPPED, lpReserved NULL.
- **Implementation:** `files::read_scatter` in `src/api/filesystem/files.rs`,
  registry `src/registry/filesystem.rs`. `preadv` at the OVERLAPPED offset,
  resumed after short reads; completion shared with ReadFile (status, event, IOCP).
- **Scope/errors:** completes at once; non-page length, NULL OVERLAPPED,
  non-overlapped handle or non-NULL reserved: ERROR_INVALID_PARAMETER; NULL or
  unaligned segment: ERROR_NOACCESS. Reads past EOF return the bytes present.
  NO_BUFFERING is not tracked or enforced.
- **Verification:** `python3 tests/test_read_scatter.py` (shuffled pages, offset,
  event, GetOverlappedResult, IOCP packet, EOF, errors). **PASS**. Game: streamer
  reports 2237 reads, 12.5 MiB.
- **Impact:** shared file code; `test_iocp.py`, `test_files_sync.py`,
  `test_iocp_pipe.py` pass.

## USER32.dll!CreateCursor

- **Trigger:** STOP at caller 0x141dc1809 in `Input Initialization`.
- **Contract:** [CreateCursor](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-createcursor):
  monochrome AND/XOR planes, WORD-aligned rows, MSB first; AND 1/XOR 0
  transparent, 0/0 black, 0/1 white, 1/1 inverted.
- **Implementation:** `input::create_cursor` in `src/api/window/input/cursor.rs`,
  `Connection::bitmap_cursor` in `src/host/x11/mod.rs`, registry `window.rs`.
- **Scope/errors:** up to 256x256; X cannot invert, so 1/1 pixels show black.
  Bad size/hotspot/planes: ERROR_INVALID_PARAMETER. DestroyCursor still does not
  free X cursors.
- **Verification:** game passes input init. No isolated test yet.

## XINPUT1_3.dll (also 1_4, 9_1_0)

`XInputGetState` (ordinal 2), `XInputSetState`, `XInputGetCapabilities`,
`XInputEnable`, `XInputGetBatteryInformation`, `XInputGetKeystroke`,
`XInputGetStateEx` (ordinal 100), `XInputGetDSoundAudioDeviceGuids`,
`XInputGetAudioDeviceIds`

- **Trigger:** STOP at `XINPUT1_3.dll!#2`, caller 0x141dc4b33.
- **Contract:** [XInput](https://learn.microsoft.com/en-us/windows/win32/api/xinput/):
  ERROR_DEVICE_NOT_CONNECTED for an empty slot, ERROR_BAD_ARGUMENTS for user > 3.
- **Implementation:** `src/api/window/input/xinput.rs`; registry `window.rs`;
  ordinal table in `src/registry/ordinals.rs`.
- **Scope:** no controller backend — every slot is reported empty, as on Windows
  with no pads. Real gamepads remain TODO.
- **Verification:** game passes input init. No isolated test yet.

## KERNEL32.dll!GlobalMemoryStatus

- **Trigger:** STOP at caller 0x141dbfb2f after the memory report.
- **Contract:** MEMORYSTATUS, 56 bytes on x64, dwLength set by the function.
- **Implementation:** `info::global_memory_status` from the Ex values.
- **Verification:** game continues. No isolated test yet.

## vulkan-1.dll!vkCreateWin32SurfaceKHR (static import)

- **Trigger:** `vkCreateWin32SurfaceKHR failed with error (VK_ERROR_EXTENSION_NOT_PRESENT)`.
  DOOM imports the function statically, so the host `vkCreateXlibSurfaceKHR`
  pointer was never cached through vkGetInstanceProcAddr.
- **Implementation:** `create_win32_surface` in `src/api/graphics/vulkan.rs` now
  resolves the host function from the instance when the cache is empty.
- **Verification:** game creates its swapchain twice and continues to loading.

## WINHTTP.dll — client without a network transport

`WinHttpOpen`, `WinHttpCloseHandle`, `WinHttpConnect`, `WinHttpOpenRequest`,
`WinHttpSetStatusCallback`, `WinHttpSetOption`, `WinHttpSetTimeouts`,
`WinHttpAddRequestHeaders`, `WinHttpSendRequest`, `WinHttpReceiveResponse`,
`WinHttpQueryHeaders`, `WinHttpQueryDataAvailable`, `WinHttpReadData`, `WinHttpWriteData`

- **Trigger:** STOP at `WinHttpSetOption` (caller 0x1403e7c21). The game opens an
  async session `idLib (Steam)`, sets option 84 (secure protocols) and sends
  `HEAD https://ghostrc.game.idtech.services/config-<candidate>.json`.
- **Contract:** [WinHTTP](https://learn.microsoft.com/en-us/windows/win32/api/winhttp/):
  handle hierarchy; children inherit async mode and status callback; async
  failures arrive as `WINHTTP_CALLBACK_STATUS_REQUEST_ERROR` with
  `WINHTTP_ASYNC_RESULT { API_SEND_REQUEST, error }` and the SendRequest context.
- **Implementation:** `src/api/network/winhttp.rs`, registry `src/registry/network.rs`.
  Real handles, options and timeouts per handle; callbacks for async handles run
  in order on one Windows thread (CreateThread), HANDLE_CREATED synchronously.
- **Scope/errors:** synchronous and asynchronous GET/HEAD requests use the host
  libcurl with certificate and hostname verification. `WinHttpAddRequestHeaders`
  accepts ADD, ADD_IF_NEW, REPLACE and coalesced headers, including DOOM's `Connection` header. Request bodies,
  POST and unsupported header modifiers fail explicitly. Async SendRequest
  delivers SENDREQUEST_COMPLETE or REQUEST_ERROR; ReceiveResponse delivers
  HEADERS_AVAILABLE; QueryDataAvailable delivers DATA_AVAILABLE. Numeric status
  queries, common response headers, raw headers and buffered body reads are
  implemented, including async READ_COMPLETE with the caller buffer. SYSTEMTIME
  header conversion remains unsupported. `PEEKGAME_TRACE_HTTP`
  logs method/URL, redacted header names, HTTP status or WinHTTP transport error.
- **Verification:** `python3 tests/test_winhttp_transport.py` **PASS** with a
  private HTTP server: Rust PE GET receives status 200 and body; a Windows async
  HEAD fixture receives callbacks/status and Content-Type, forwards
  `Connection: close`, and completes an async read with a null byte-count
  pointer at EOF.
  `python3 tests/test_winhttp_offline.py` **PASS** for refused localhost
  connections, handle errors and async REQUEST_ERROR. A public HTTPS probe to
  `example.com` also returned status 200 and read its body. DOOM's real
  service returned HTTP 200 for its HEAD request; the subsequent async read
  had failed with ERROR_INVALID_PARAMETER before this callback fix. Later
  authentication steps still require a user run.

## steamclient64.dll — ISteamRemoteStorage014 and ISteamUserStats011

- **Trigger:** in-game "There was an error saving your game" and `Profile error 0x40`.
  `GetISteamGenericInterface(STEAMREMOTESTORAGE_INTERFACE_VERSION014)` and
  `(STEAMUSERSTATS_INTERFACE_VERSION011)` returned NULL (not bridged); DOOM keeps its
  profile and saves in Steam Cloud.
- **Contract:** Steamworks SDK 1.42 headers (Proton `lsteamclient/steamworks_sdk_142`).
  RemoteStorage014 has 55 slots on PC (the PS3/server block is compiled out).
  UserStats011 has 43; MSVC reverses each overload group, so Windows slots 1-4,
  17-18 and 39-42 map to the other Linux slot (checked against Proton's
  `winISteamUserStats.c` vtable). Callback layouts computed with gcc at pack 4
  (Linux) and pack 8 (Windows).
- **Implementation:** `native/steam_bridge/steam_bridge.c`: two new kinds, getters
  for `GetISteamRemoteStorage`/`GetISteamUserStats` and the generic getter.
  RemoteStorage slots 0-23 (cloud files, async read/write, write streams, quota,
  cloud flags); UserStats all slots except `GetDownloadedLeaderboardEntry`
  (LeaderboardEntry_t is 28 vs 32 bytes; stays a trap). Layouts: 1101-1110, 1112,
  1331, 1332. `call_result` staging buffer raised from 64 to 256 bytes.
  Unbridged interface names are logged once each.
- **Scope:** forwards to the real client; never answers for Steam. Workshop/UGC
  slots (24-54) remain traps. FileShare's result (272 bytes with a 260-byte name)
  is not converted and reports failure.
- **Verification:** `python3 tests/test_steam_cloud.py` **PASS** (fake client: sync
  files, async call results 20->24 bytes, overload order, float/double arguments,
  1103/1102 callbacks). Game: profile loads, `remote/PROFILE/profile.bin` and
  `remote/GAME-AUTOSAVE0/*` written in the user's Steam userdata; no Profile error.
  Note: `tests/test_steam_bridge.py` cannot run, its `native/steam_bridge/test_bridge.c`
  source is missing (also absent from sdl3-window).

## USER32.dll!ChangeDisplaySettingsExA (and W)

- **Trigger:** the game window stayed a 720x576 borderless window in a corner
  ("full screen"). DOOM asks for its r_mode size with `ChangeDisplaySettingsExA`.
- **Bugs fixed:** the size was compared with the whole X screen (6400x1440 over
  three monitors), so even the native 2560x1440 was refused; the A variant read
  DEVMODEW offsets (172/176) instead of DEVMODEA (108/112).
- **Implementation:** `compat::change_display_settings_ex_a/_w`, device names
  through `display::device_size` (`\\.\DISPLAYn`, primary when NULL).
- **Scope:** still no mode switching: NULL/restore and the device's current size
  succeed; other sizes DISP_CHANGE_BADMODE. A non-native r_mode therefore stays a
  small window; scaling (Proton-style) is not implemented.
- **Verification:** probe on the 3-monitor desktop: A/W primary 2560x1440 -> 0,
  720x576 -> -2, DISPLAY2 1920x1080 -> 0, DISPLAY3 2560x1440 -> -2, restore -> 0.

## Audit fixes (shared, found by a code sweep, not by a game stop)

- `GetClientRect`/`GetWindowRect(GetDesktopWindow())`: the primary monitor, as on
  Windows; previously the whole X screen / virtual rectangle (6400x1440).
  `host::x11::screen_size` (whole X screen) removed: it caused this and the
  ChangeDisplaySettingsEx bug.
- `MessageBoxA/W`: a real modal zenity dialog returning the pressed button;
  previously logged and auto-answered IDOK/IDYES/IDRETRY. No display or zenity:
  0 with ERROR_NOT_SUPPORTED (GTK's "cannot open display" exit 1 is not "Cancel").
- `MoveFileExW`: no destination-folder creation; copy+delete only across
  filesystems (EXDEV) with MOVEFILE_COPY_ALLOWED, else ERROR_NOT_SAME_DEVICE;
  DELAY_UNTIL_REBOOT ERROR_NOT_SUPPORTED; unknown flags ERROR_INVALID_PARAMETER.
- `GetDeviceCaps`: TECHNOLOGY (2) = DT_RASDISPLAY; DRIVERVERSION (0) = 0x100.
- `LoadIconA/W`: system names outside IDI_* fail with ERROR_RESOURCE_NAME_NOT_FOUND.
- **Verification:** `python3 tests/test_audit_fixes.py` **PASS** (private Xvfb;
  real ext4 -> tmpfs move); `test_files_sync`, `test_move_file`,
  `test_display_topology`, `test_window_abi`, `cargo test` pass.

## USER32.dll!EnumDisplaySettingsA/W (mode list)

- **Trigger:** after the ChangeDisplaySettingsExA fix, DOOM kept its 32x32 creation
  window and recreated its swapchain in a loop. It had picked 720x576 (stale r_mode)
  from the advertised mode list, the switch was refused, and it never resized.
  (Before, the A variant's wrong offsets read 0 = "no change" and succeeded by accident.)
- **Fix:** each monitor lists only its current mode (index 0 and ENUM_CURRENT/REGISTRY
  settings); further indexes end the enumeration. This matches what
  ChangeDisplaySettingsEx can do.
- **Verification:** game: `ShowGameWindow: (0, 0) 2560 x 1440, full screen`,
  SetWindowPos 2560x1440, render size 2560x1440, 2 swapchains (was 400+).
  `test_display_topology.py` passes. Lower resolutions need Proton-style scaling.
- `PEEKGAME_TRACE_WINDOW` logs CreateWindowEx, ShowWindow, CloseWindow, SetWindowPos.

## Audit group 1: save safety (shared)

- `NtSetInformationFile`: unknown classes returned SUCCESS doing nothing (rename
  included: "write temp, rename over" lost the save); disposition and basic info
  faked success. Now: rename (10/65, NT `\??\` paths, RootDirectory, replace flag),
  delete disposition (13/64), basic info (4: access/write times, read-only),
  position, allocation hint, EOF; anything else STATUS_INVALID_INFO_CLASS.
- `SetFileInformationByHandle`: FileDispositionInfo faked success. Now: basic info
  (0), rename (3/22), disposition (4/21), allocation (5), EOF (6); others
  ERROR_NOT_SUPPORTED.
- `CreateFile`: FILE_FLAG_DELETE_ON_CLOSE honoured. File entries keep their host
  path (follows renames); deletion happens when the last (duplicated) handle closes.
- `SetFileTime`: 0xFFFFFFFF_FFFFFFFF ("don't change") and pre-1970 times panicked
  (u64 underflow / SystemTime overflow); now handled.
- NT status mapping: file-not-found was STATUS_IO_DEVICE_ERROR; now
  OBJECT_NAME/PATH_NOT_FOUND, NAME_COLLISION, DIRECTORY_NOT_EMPTY, NOT_SAME_DEVICE,
  SHARING_VIOLATION, END_OF_FILE, INFO_LENGTH_MISMATCH, OBJECT_NAME_INVALID.
- **Verification:** `python3 tests/test_save_safety.py` **PASS**; files_sync,
  move_file, iocp, iocp_pipe, read_scatter, audit_fixes, crt_directory, filelength,
  `cargo test` pass. DOOM: fullscreen 2560x1440, Steam Cloud saves written.

## Audit group 2: hangs and crashes (shared)

- Mutexes: a thread ending while owning a mutex left it owned forever (every later
  waiter hung). Now released as abandoned at thread exit (`objects::thread_exited`,
  called from the single Windows-thread exit point); the next acquirer gets
  WAIT_ABANDONED_0 + index (single, multiple, alertable waits).
- Critical sections: InitializeCriticalSection at an address already registered
  (memory freed without DeleteCriticalSection, then reused) exited with 78; now
  re-initializes as Windows does.
- FlushProcessWriteBuffers: was a local fence; now membarrier PRIVATE_EXPEDITED
  (registered once; GLOBAL, then a fence, as fallbacks). Host kernel mask 0x3ff.
- InitOnce: a zeroed INIT_ONCE at an address whose side record was Complete (stack
  or reused memory) skipped the callback and returned the stale context; the word
  is now authoritative.
- **Verification:** `python3 tests/test_hang_fixes.py` **PASS**; crt_core,
  files_sync, tls_threads, iocp, endthread, seh, pe_dlls, save_safety, `cargo test`
  pass. MarioVsLuigi (Mono) 45 s, 0 exceptions in Player.log, network room joined;
  DOOM fullscreen 2560x1440.

## Audit group 3: case-insensitive paths and device names (shared)

- `host::paths::host` (every Windows path: CreateFile, Find*, CRT fopen, shell
  folders, …) was case-sensitive. Now `resolve_case`: the exact path when it exists
  (no extra cost), else per-component case-insensitive matching (exact wins, ties
  sorted); unmatched components stay as written so creation works under the
  resolved parent.
- Device names in the last component (also with an extension): NUL -> /dev/null;
  CON/PRN/AUX/COM1-9/LPT1-9 -> ERROR_FILE_NOT_FOUND. CONIN$/CONOUT$ unaffected.
- **Verification:** `python3 tests/test_case_paths.py` **PASS**; files_sync,
  move_file, crt_directory, save_safety, audit_fixes, crt_core, pe_dlls,
  early_appdata, crt_port, filelength, max_stdio, `cargo test` pass. DOOM streamed
  37239 reads at ~8.7 GB/s; MarioVsLuigi 0 exceptions.

## Audit batch A: filesystem follow-ups (shared)

- CreateFile: GENERIC_READ with OPEN_ALWAYS/CREATE_NEW/CREATE_ALWAYS failed (Rust
  refuses to create without write access); now open(2) O_CREAT[/O_EXCL/O_TRUNC].
  TRUNCATE_EXISTING without write access is ERROR_INVALID_PARAMETER.
- LockFile/LockFileEx/UnlockFile(Ex) faked success; now Linux OFD locks (per open
  file, conflicting between handles in one process, shared by duplicates):
  shared/exclusive, fail-immediately or wait, zero length locks nothing. Advisory,
  unlike Windows' mandatory locks; unlocking an unlocked range is not reported.
- NtCreateFile: RootDirectory-relative names, FILE_DIRECTORY_FILE /
  FILE_NON_DIRECTORY_FILE / FILE_DELETE_ON_CLOSE, invalid disposition
  STATUS_INVALID_PARAMETER, Information FILE_CREATED/OPENED/OVERWRITTEN/SUPERSEDED.
- Not changed: an overlapped read failing at once with EOF still queues its IOCP
  packet. The audit called this a double notification, but `test_iocp.py`
  deliberately asserts the packet and real Windows behaviour is unverified here.
- **Verification:** `python3 tests/test_fs_batch_a.py` **PASS**; files_sync,
  save_safety, iocp, iocp_pipe, read_scatter, case_paths, move_file, audit_fixes pass.

## Audit batch B: threading follow-ups (shared)

- CreateEvent rejected any SECURITY_ATTRIBUTES (ERROR_NOT_SUPPORTED); now accepted.
  Named events (process-local, "Local\\" = no prefix, weak name table so a name
  lives with its handles), ERROR_ALREADY_EXISTS for an existing name, collision with
  a mutex/semaphore name is ERROR_INVALID_HANDLE; OpenEventA/W; CreateEventExW names.
- Waitable timers: completion routines were never run (fake success); now queued
  as APCs to the setting thread (alertable waits only). Each set spawned a thread
  that slept out the old deadline; threads now leave as soon as the timer is set
  again, cancelled or closed (300 re-arms: 1 thread left). Closing forgets the timer.
- Critical sections: Enter/Leave used one global table lock; the host mutex pointer
  now lives in LockSemaphore. OwningThread, RecursionCount and LockCount (-1/-2)
  are maintained.
- Exposed by named events: UnityPlayer now starts UnityCrashHandler64.exe.
  InitializeProcThreadAttributeList/UpdateProcThreadAttribute/
  DeleteProcThreadAttributeList implemented; CreateProcessA/W fail with
  ERROR_NOT_SUPPORTED and log the command line (Unity continues).
- Not changed: WaitForMultipleObjects(bWaitAll) atomicity (narrow race; a fix would
  route every single-event wait through the global condition variable).
- **Verification:** `python3 tests/test_threading_batch_b.py` **PASS**; hang_fixes,
  crt_core, files_sync, iocp, tls_threads, endthread, seh, `cargo test` pass.
  MarioVsLuigi 0 exceptions (crash handler refused, room joined); DOOM runs.

## SetWindowPos size hints (windowed mode stuck at 32x32)

- **Trigger:** in windowed mode (r_fullscreen 0, from the cloud profile) DOOM rendered
  and saved r_windowWidth/Height 32. Its window is created 32x32 and resized with
  SetWindowPos to 2560x1440. Non-resizable windows get fixed-size WM hints (min = max);
  SetWindowPos sent the X resize before updating them, so Mutter constrained the new
  request with the old 32x32 limits.
- **Fix:** `set_window_pos` updates the fixed-size hints before XMoveResizeWindow.
  Mutter now fits the 2560x1440 request into the work area (2560x1321 below the top
  bar), which DOOM renders and saves.
- `PEEKGAME_TRACE_WINDOW` also logs X ConfigureNotify sizes and every XMoveResizeWindow.
- **Verification:** windowed DOOM: render size 2560x1321, saved 2560x1321;
  window_abi, godot_input, display_topology, audit_fixes, mouse_hooks pass.

## PEEKGAME_VULKAN_FPS (frame-rate cap)

- **Trigger:** FIFO tears on this compositor (NVIDIA/X11/GNOME); mailbox is
  tear-free but uncapped, pinning the GPU at 100%. A cap gives tear-free + bounded.
- **Implementation:** `queue_present` (`src/api/graphics/vulkan.rs`) sleeps to the
  next frame boundary after vkQueuePresentKHR; `PEEKGAME_VULKAN_FPS=N` sets the rate
  (0/unset = passthrough). Per-thread deadline, reset when late (no burst catch-up).
  Like create_win32_surface, present is a static import, so the real host function
  is fetched via the cached instance when the proc-addr cache is empty.
- **Use:** `PEEKGAME_VULKAN_PRESENT=mailbox PEEKGAME_VULKAN_FPS=144` — tear-free and
  capped. Verified on DOOM (runs clean; GPU no longer pinned).

## vkCreateSwapchainKHR present mode (tearing)

- **Finding:** DOOM requests VK_PRESENT_MODE_FIFO (2, vsync), which is tear-free by
  spec; the bridge forwards it unchanged, so tearing is a display-stack issue
  (NVIDIA on X11/GNOME), not the game or the bridge.
- **Added knob:** `create_swapchain` (`src/api/graphics/vulkan.rs`) reads
  presentMode at offset 88 of VkSwapchainCreateInfoKHR; `PEEKGAME_VULKAN_PRESENT`
  overrides it (immediate/mailbox/fifo|vsync/relaxed). `PEEKGAME_TRACE_GL` logs the
  requested and applied mode. Default behaviour unchanged (pure passthrough).
- **Real fixes for the NVIDIA/X11 tearing:** `nvidia-settings` "Force Full
  Composition Pipeline" on the X screen, or true fullscreen (r_fullscreen 1).
- **Verification:** DOOM logs present mode 2; with the override, `2 -> 1`.

## Next

- Done: `CloseWindow` (minimize); unbridged interfaces logged once; 720x576 came
  from a stale `r_mode "14"` in DOOMEternalConfig.local (fresh config: 1920x1080).
- Dropped Steam callbacks seen in the log: 336, 1040011, 1040015, 1040044, 1270006,
  1270009 (layouts not bridged).

## GetLastError / WSAGetLastError call-site trace (diagnostic)

- **Trigger:** DOOM Eternal Battlemode "Create Arena" fails with `PARTY_STATE_CHANGE_TYPE_REGIONS_CHANGED failed: 1/8`; PartyError 8 is PartyWin's "unmapped platform error" (a Win32/Winsock error it has no mapping for), so the failing API has to be found.
- **Contract:** unchanged; both return the calling thread's last error.
- **Implementation:** both exports enter `peekgame_get_last_error_traced` (`src/api/system/error.rs`), which passes the return address along. With `PEEKGAME_TRACE_LAST_ERROR=PartyWin.dll` (or `all`), each nonzero read prints `[last-error] Module+RVA read error N` (first 3 per call site and code). The RVA points just after the failed call.
- **Also:** WinHTTP prints `MultiplayerServer/*` response bodies under `PEEKGAME_TRACE_HTTP` (all bodies with `PEEKGAME_TRACE_HTTP_BODY`), with credential-like JSON values redacted.
- **Verification:** unit test `redact_json_hides_credentials_only`; a mingw fixture showed the expected RVAs for SetLastError/WSASetLastError reads.
- **Other engines:** no behavior change; the trace is off unless the variable is set.

## CRYPT32 for PlayFab Party's DTLS identity (PartyWin.dll)

- **Trigger:** PartyWin creates its UDP socket, then builds a local self-signed certificate. `CryptEncodeObjectEx(X509_NAME)` returned ERROR_NOT_SUPPORTED, PartyWin turned it into `0x80070032` ("unmapped platform error", PartyError 8) and closed the socket, so regions/QoS never ran (`REGIONS_CHANGED failed: 1/8`, `CreateNewNetwork 1/4202`).
- **Sequence called** (from the PartyWin call sites):
  1. `CryptEncodeObjectEx(X509|PKCS7, X509_NAME, CN=<UTF8>)`
  2. `CertCreateSelfSignCertificate(0, name, 0, NULL, NULL, start, end, NULL)`
  3. `CryptHashCertificate2(L"SHA256")` and `CertVerifyTimeValidity`
  4. separately: a collection of the system stores My/Root (CurrentUser and LocalMachine, read-only), searched with `CertFindCertificateInStore(CERT_FIND_SHA1_HASH)`
- **Implementation (`src/api/network/certificates/`):**
  - `encode.rs`: X509_NAME DER, covering RDN string types, encoded blobs, the caller's buffer, MORE_DATA and the LocalAlloc flag.
  - `selfsign.rs`: an OpenSSL RSA-2048 key and a v3 certificate. The signature defaults to sha1RSA as documented; sha256/384/512RSA can be requested. The validity comes from the SYSTEMTIMEs, defaulting to now plus 1 year. The private key is kept host-side, keyed by the certificate DER, for a future SSPI credential.
  - Collection stores, an empty read-only `MY` system store, SHA1 find, CNG-named certificate hashes, time validity.
- **Scope limits:** CertCreateSelfSignCertificate rejects caller keys/providers, flags and extensions with ERROR_NOT_SUPPORTED. Other encode structures and find types return ERROR_NOT_SUPPORTED. A writable MY store also returns ERROR_NOT_SUPPORTED.
- **Verification:** `tests/test_party_certificate.py`. The generated certificate is checked with `openssl x509`/`openssl verify`. Two unit tests cover the DER and OID encoding.
- **Next for Party:** Schannel DTLS (SspiCli: AcquireCredentialsHandleW, Initialize/AcceptSecurityContext, Encrypt/DecryptMessage), GetAddrInfoExW, and overlapped WSASendTo/WSARecvFrom with WSAGetOverlappedResult. All are still missing (STOP or 10045).
- **Other engines:** stores and encodes that used to fail now work for these exact cases; everything else is unchanged.

## Overlapped UDP and asynchronous DNS for PlayFab Party (PartyWin.dll)

- **Trigger:** once the DTLS certificate existed, PartyWin posted `WSARecvFrom` with an OVERLAPPED (event, no completion routine) on its dual-stack IPv6 UDP socket and expected `WSA_IO_PENDING`. PeekGame returned 10045. PartyWin's error path then hit `LeaveCriticalSection without ownership` and a SIGSEGV. The QoS hosts (`mpsqosprod.<region>.cloudapp.azure.com`) are resolved with `GetAddrInfoExW` + OVERLAPPED.
- **Contract:**
  - Overlapped WSASend/WSASendTo/WSARecv/WSARecvFrom complete synchronously when possible: return 0, fill OVERLAPPED and signal the event.
  - Otherwise they return WSA_IO_PENDING. Completion writes Internal (status), InternalHigh (bytes) and Offset (flags), then signals hEvent.
  - WSAGetOverlappedResult returns WSA_IO_INCOMPLETE while pending, waits on hEvent when asked, and reports WSAEMSGSIZE for truncated datagrams. closesocket aborts pending operations with WSA_OPERATION_ABORTED.
  - GetAddrInfoExW (NS_ALL/NS_DNS) works synchronously or asynchronously: WSA_IO_PENDING, *ppResult set, event signalled, status from GetAddrInfoExOverlappedResult. FreeAddrInfoExW frees the list.
- **Implementation:** `src/api/network/overlapped.rs` (a poll-thread reactor; per-socket order kept by direction), `src/api/network/resolve.rs` (a host getaddrinfo per async request on a host thread).
- **Scope limits:** completion routines (WSA and GetAddrInfoEx) return WSAEOPNOTSUPP. IOCP association for sockets is not provided. GetAddrInfoEx timeouts, providers and GetAddrInfoExCancel are not implemented.
- **Verification:** `tests/test_overlapped_udp.py`.
- **Other engines:** synchronous WSA calls are unchanged; overlapped calls that used to fail now work in this event-based form.

## UCRT time names for msvcp140 (`_Getdays` and friends)

- **Trigger:** `STOP: missing Windows API api-ms-win-crt-time-l1-1-0.dll!_Getdays`, called from msvcp140.dll (the Microsoft C++ library extracted from the VC redist) when it builds its time facets (`_Locinfo`).
- **Contract:**
  - `_Getdays`/`_Getmonths` (and the `_W_` versions) return ":Sun:Sunday:..." / ":Jan:January:..." in CRT memory, freed with `free`.
  - `_Gettnames`/`_W_Gettnames` return one CRT block: a copy of UCRT's `__crt_lc_time_data` (704 bytes on x64) followed by its strings.
  - `_Strftime`/`_Wcsftime` are strftime with an explicit time-data argument.
- **Implementation:** `src/api/crt/cxx_support.rs`. The data is the C locale, the only locale this CRT has, so the time-data argument always describes it.
- **Verification:** `tests/test_crt_time_names.py` (struct layout static assert, strings, `_Strftime` output).
- **Other engines:** any program using MSVC's std::time_get/put with msvcp140 benefits.

## Remaining PartyWin / msvcp140 imports (CRT, thread pool, files, WinRT string)

- **Trigger:** `STOP: ...api-ms-win-crt-time-l1-1-0.dll!_mkgmtime64` (PartyWin). The rest of the unresolved list was done in the same change, so the game doesn't stop on each one in turn.
- **CRT** (`src/api/crt/cxx_support.rs`):
  - `_mkgmtime64` (normalises the struct tm; range 1970–3000, otherwise -1/EINVAL), `_difftime64`, `fmax`, `_dtest`.
  - `__stdio_common_vsnprintf_s`: count/_TRUNCATE truncation returns -1; overflowing the buffer empties it with ERANGE.
  - `__stdio_common_vsscanf`: narrow conversions through the host sscanf. The Windows size prefixes are translated, and secure-CRT buffer sizes become field widths (max 32 conversions). Wide conversions fail with EINVAL.
- **Thread pool** (`src/api/threading/pool.rs`): work, timer and wait objects, plus FreeLibraryWhenCallbackReturns, IsThreadpoolTimerSet and the Wait*Callbacks functions. Callbacks run on Windows threads. Custom pools, cleanup groups and RaceDll return ERROR_NOT_SUPPORTED.
- **Files:** `CreateFile2` (extended parameters to CreateFileW). `CopyFile2` with COPY_FILE_FAIL_IF_EXISTS; other flags and callbacks return HRESULT ERROR_NOT_SUPPORTED. `CreateSymbolicLinkW` (relative targets stay relative), `CreateHardLinkW`.
- **TerminateThread:** works for the calling thread (as ExitThread). Another thread returns ERROR_NOT_SUPPORTED, because stopping a thread at an arbitrary instruction is not safe here.
- **WindowsConcatString:** a new HSTRING; NULL counts as empty.
- **Verification:** `tests/test_crt_party_batch.py`, `tests/test_kernel32_party_batch.py`.
- **Still missing:** SspiCli (Schannel DTLS) for PartyWin.

## GetAdaptersAddresses: DadState and all addresses (PlayFab Party local candidates)

- **Trigger:** with QoS working, Create Arena got as far as `/Party/RequestParty`. Then `ENDPOINT_DESTROYED ... 1/131` and `ConnectToNetwork 10/131`, with no relay packets at all. Public error 131 is PartyWin's internal `0x807a102e`, raised when its local address candidate list is empty. PartyWin keeps only unicast entries with `DadState == IpDadStatePreferred` (and skips loopback and Teredo adapters). PeekGame left DadState at 0 (IpDadStateInvalid), so every address was dropped.
- **Contract/implementation (`src/api/network/adapters.rs`):**
  - Every address of each adapter is listed as a unicast entry (AF_UNSPEC: IPv4 then IPv6).
  - Each entry has DadState Preferred, infinite Valid/Preferred/Lease lifetimes and its OnLinkPrefixLength. Link-local IPv6 addresses carry the interface as scope ID.
  - Ipv4Enabled/Ipv6Enabled reflect the addresses present.
- **Scope limit:** origins are "other", because Linux does not say whether an address came from DHCP or was set manually.
- **Verification:** `tests/test_iphlpapi_adapters.py` (every unicast entry: length, sockaddr size, DadState, lifetimes, family flags; PartyWin's GAA flags).

## WSOCK32 ordinals 10–12 (Battlemode "Invalid host address")

- **Trigger:** Battlemode matchmaking found real players and the lobby filled, then `BattleArenaGameSession error: (state: STATE_TRANSITIONING, error: 0/2) Invalid host address`. The log also showed `ioctlsocket(0x7bc49712)`, `ioctlsocket(0x13)` and similar calls with argument values that are not ioctl commands.
- **Cause:** DOOMEternalx64vk.exe imports Winsock 1.1 from WSOCK32.dll **by ordinal**. In WSOCK32, 10/11/12 are `inet_addr`/`inet_ntoa`/`ioctlsocket`; in WS2_32 they are `ioctlsocket`/`inet_addr`/`inet_ntoa` (Wine's wsock32.spec and ws2_32.spec agree). PeekGame used the WS2_32 table for both, so `inet_addr("127.0.0.1")` (call site 0x1403e1432, "Sys_InitNetworking: adding loopback interface") and the host-address conversions ran ioctlsocket and got an error instead of an address.
- **Fix:** `src/registry/ordinals.rs` has WSOCK32's own numbering for 10–12. `src/registry/mod.rs::find_import` picks the table by the DLL actually named, since WSOCK32 still canonicalizes to ws2_32 for name lookups.
- **Verification:** unit tests `winsock_ordinals_differ_at_10_to_12` and `import_by_ordinal_uses_the_dll_s_own_numbering`. The game still has to be re-run.
- **Other engines:** any program importing WSOCK32 #10–#12 by ordinal was affected the same way.

## Windowed mode stuck at 32x32 (GNOME applies the creation size at first map)

- **Trigger:** starting DOOM Eternal in windowed mode (or switching to it) left the window at 32x32, and the game saved `r_windowWidth/Height "32"`, which made it permanent. The trace (`PEEKGAME_TRACE_WINDOW`) showed `SetWindowPos 1280x720`, X confirming 1280x720, then an unrequested `X Configure (0,66) 32x32` from the window manager after `XMapWindow`. DOOM creates its window as a 32x32 popup and resizes it before showing it.
- **Ruled out:** the size hints (resizable, no min/max after the style change), the resize-before-map order, update_fullscreen.
- **Fix:** `Window::map_guard` (`src/api/window/state.rs`) is set when a top-level window is first mapped and holds the program's size. A later SetWindowPos updates it. Within 2 s, the first window-manager configure with a different size is answered once by requesting the program's size again (`src/api/window/pump.rs`) instead of being reported as WM_SIZE. On Windows the program's size stands.
- **Also:** the window trace now covers style changes, AdjustWindowRectEx, Get/SetWindowPlacement, client-size changes, WM_NORMAL_HINTS and XMapWindow. A temporary opt-in focus hack (`PEEKGAME_TINY_WINDOW_720P`) was removed.
- **Verification:** user run on GNOME/X11: the window keeps the configured size (1920x1080) and the config is no longer overwritten. This can't be reproduced in the Xvfb tests (no window manager).
