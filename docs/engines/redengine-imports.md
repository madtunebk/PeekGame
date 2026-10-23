# REDengine import ledger

## Cyberpunk 2077 — host cursor visibility

- **Trigger:** at the title screen, the OS pointer showed over the game and
  movement felt wrong while the game drew its own UI pointer.
- **Implementation:** the `HideHostCursorWhileFocused` quirk in
  `src/quirks/mod.rs` makes `src/api/window/input/cursor.rs` use the blank X
  cursor on Cyberpunk windows while they have keyboard focus. Focus loss
  restores the X cursor; focus gain reapplies the blank cursor.
- **Scope:** this changes only the host pointer image, not game mouse events,
  raw motion, or the cursor visibility value reported to Windows code.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — audio investigation

The game reached its rendered title screen, but no sound was heard. The current
launcher log contains no `WASAPI:` playback initialization or render messages.
The executable embeds the `MMDeviceEnumerator` COM class ID, so the next run
used `PEEKGAME_TRACE_AUDIO=1`: COM activation succeeds, but the trace has no
WASAPI stream initialization or render messages. The trace now also reports
endpoint enumeration, default-device queries, property-store access, and
`IAudioClient` activation to locate the failed step. PeekGame's host
PipeWire/PulseAudio server is available; the current default sink is NVIDIA HDMI
and an HG9053 USB headset is also present. No output device was changed during
this investigation.

## Cyberpunk 2077 — pointer recentering at title screen

- **Trigger:** the game rendered a visible "Press to Continue" screen, but the
  pointer repeatedly snapped to the middle of the window.
- **Cause:** PeekGame's core-pointer motion handler could warp a hidden cursor
  back to the window center even while XInput2 raw relative motion supplied
  camera deltas independently. That warp is unnecessary with XInput2 and can
  disturb a game-drawn menu cursor.
- **Implementation:** `src/api/window/input/raw.rs` skips the recenter when
  XInput2 owns relative motion. The core-pointer fallback still recenters near
  an edge. `PEEKGAME_TRACE_CURSOR_WARP=1` distinguishes game-requested
  `SetCursorPos` warps from PeekGame's fallback recentering.
- **Verification:** release build and staged distribution; a live Cyberpunk
  rerun is needed to confirm the pointer behavior.

## Cyberpunk 2077 — `GetKeyNameTextA`

- **Trigger:** Cyberpunk reached a visible loading screen after creating a
  2560×1440 DXGI swapchain; the next stop was `USER32.dll!GetKeyNameTextA`
  at `Cyberpunk2077.exe+0x22063c5`.
- **Contract:** return a human-readable key name from the scan-code and
  extended-key bits in `lParam`, writing at most the caller's character count.
- **Implementation:** `src/api/window/input/keyboard.rs::get_key_name_text_a`,
  registered in `src/registry/window.rs`. It shares the existing scan-code/name
  mapping with `GetKeyNameTextW` and writes the narrow result with a terminator.
- **Scope:** names are the X11 keyboard mapping's current set; non-ASCII names
  are UTF-8 bytes rather than Windows ACP bytes.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `_dupenv_s`

- **Trigger:** after `_splitpath_s` was added, the next stop was
  `api-ms-win-crt-environment-l1-1-0.dll!_dupenv_s`, called from
  `0x771477a86206`.
- **Contract:** find an environment variable case-insensitively, allocate a
  caller-owned copy with `malloc`-compatible storage, and return the byte count
  including NUL. A missing variable succeeds with null output and count zero.
  See [Microsoft's CRT reference](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/dupenv-s-wdupenv-s).
- **Implementation:** `src/api/crt/ucrt.rs::dupenv_s`, registered in
  `src/registry/crt.rs`. It uses the process environment and the same host CRT
  allocator as PeekGame's `free`.
- **Scope:** host environment values are converted to UTF-8 bytes; non-UTF-8
  bytes are replaced. Null required pointers return `EINVAL`; allocation
  failure returns `ENOMEM`.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `_splitpath_s`

- **Trigger:** after `_msize` was added, the next stop was
  `api-ms-win-crt-filesystem-l1-1-0.dll!_splitpath_s`, called from
  `0x77c3036f0809`.
- **Contract:** split an ANSI path into drive, directory, base name and
  extension; validate requested output buffers, return `EINVAL` on bad
  arguments or `ERANGE` if a buffer is too short. See [Microsoft's CRT
  reference](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/splitpath-s-wsplitpath-s).
- **Implementation:** `src/api/crt/ucrt.rs::splitpath_s`, registered in
  `src/registry/crt.rs`. It clears requested outputs before validation, then
  writes all four components only if they fit.
- **Scope:** byte-oriented path separators and drive letters; DBCS trail-byte
  awareness is not implemented.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `_msize`

- **Trigger:** after `QueryActCtxW` was registered, the next stop was
  `api-ms-win-crt-heap-l1-1-0.dll!_msize`, called from `0x7635b7452dc4`.
- **Contract:** return the allocation size of a CRT heap block. A null pointer
  returns `SIZE_MAX` with `EINVAL` after parameter validation. See [Microsoft's
  CRT reference](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/msize).
- **Implementation:** `src/api/crt/ucrt.rs::msize`, registered in
  `src/registry/crt.rs`. It queries glibc's usable block size because PeekGame's
  CRT `malloc`/`calloc`/`realloc`/`_recalloc` use that allocator.
- **Scope:** valid pointers must come from the host-backed CRT heap; the UCRT
  invalid-parameter callback is not invoked for null pointers.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `QueryActCtxW`

- **Trigger:** after MFC and `CChromaEditorLibrary64.dll` initialized,
  execution stopped at `KERNEL32.dll!QueryActCtxW`, called from
  `0x792f64aeca6d`.
- **Contract:** query information about an activation context associated with
  a handle, loaded module, address, or the current thread. See [Microsoft's API
  reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-queryactctxw).
- **Implementation:** `src/api/system/compat.rs::query_act_ctx_w`, registered in
  `src/registry/system/compatibility.rs`. PeekGame currently does not build
  activation contexts from manifests, so it reports absence with `FALSE` and
  `ERROR_SXS_KEY_NOT_FOUND`, without fabricating context data.
- **Scope:** no activation-context classes are populated. Set
  `PEEKGAME_TRACE_ACTCTX=1` to see the flags and class requested by a live call.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `GetProfileIntW`

- **Trigger:** after loading `CChromaEditorLibrary64.dll` and its `mfc140u.dll`
  dependency, execution stopped at `KERNEL32.dll!GetProfileIntW`, called from
  `0x7d6656735db1` in the user's run.
- **Contract:** the legacy Win.ini lookup returns the caller's default when the
  key is absent. A negative stored value returns zero. See [Microsoft's API
  reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getprofileintw).
- **Implementation:** `src/api/system/compat.rs::get_profile_int_w`, registered
  in `src/registry/system/compatibility.rs`. PeekGame has no Win.ini profile,
  so all lookups use the documented absent-key default, as `GetProfileIntA`
  already does.
- **Scope:** no Win.ini file or registry INI mapping is read. This affects any
  program calling `GetProfileIntW`, including MFC-based components.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `__stdio_common_vsnwprintf_s`

- **Trigger:** after MFC initialized, execution stopped at
  `api-ms-win-crt-stdio-l1-1-0.dll!__stdio_common_vsnwprintf_s`, called from
  `0x7f5450347138`.
- **Contract:** format a UTF-16 string using a Windows x64 `va_list`, respect the
  output buffer and maximum count, terminate the result, and report truncation.
- **Implementation:** `src/api/crt/cxx_support.rs::common_vsnwprintf_s`,
  registered in `src/registry/crt.rs`. It delegates formatting to the existing
  wide formatter and applies the secure buffer/count behavior already used for
  the narrow counterpart.
- **Scope:** inherits the existing wide formatter's conversion coverage; an
  invalid buffer fails with `EINVAL`, and a non-truncating overflow fails with
  `ERANGE` and empties the buffer.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — `_recalloc`

- **Trigger:** after `GetProfileIntW` was added, the next stop was
  `api-ms-win-crt-heap-l1-1-0.dll!_recalloc`, called from `0x72c7df3bfd50`.
- **Contract:** resize a CRT block and zero newly allocated bytes; null input
  behaves like `calloc`, zero requested size frees the block, and allocation
  failure leaves the original block intact. See [Microsoft's CRT reference](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/recalloc).
- **Implementation:** `src/api/crt/ucrt.rs::recalloc`, registered in
  `src/registry/crt.rs` so the UCRT heap API set resolves through the existing
  CRT mapping. It uses the host allocator shared by PeekGame's `malloc`,
  `calloc`, `realloc` and `free` implementations.
- **Scope:** overflow and allocation failures set `ENOMEM`; newly allocated
  bytes beyond the host allocator's prior usable block size are zeroed. The
  host allocator may reserve more bytes than the original requested size, so
  zeroing within that spare capacity is not tracked separately.
- **Verification:** release build and staged distribution; live Cyberpunk
  rerun pending.

## Cyberpunk 2077 — playback device format property

- **Trigger:** `PEEKGAME_TRACE_AUDIO=1` showed successful playback device
  discovery, followed by repeated reads of `PKEY_AudioEngine_DeviceFormat`
  and no `IAudioClient` activation. The property store returned `VT_EMPTY`.
- **Implementation:** `src/api/audio/properties.rs` now returns a `VT_BLOB`
  containing the same 48 kHz stereo float `WAVEFORMATEX` that
  `IAudioClient::GetMixFormat` advertises. The blob uses COM task memory and is
  freed by `PropVariantClear`.
- **Verification:** release build and staged distribution; live Cyberpunk
  audio rerun pending.

## FIFO presentation default

- **Trigger:** Cyberpunk rendered at 1920×1080 while the GPU remained at 99%.
  Its DXGI path had not yet logged the actual Vulkan present mode.
- **Implementation:** the Vulkan bridge now uses `VK_PRESENT_MODE_FIFO_KHR`
  globally if a swapchain requests IMMEDIATE or MAILBOX.
  `PEEKGAME_VULKAN_PRESENT` still overrides this default. With
  The bridge logs requested and applied modes at swapchain creation.
- **Verification:** release build and staged distribution. The live Cyberpunk
  log reports `present mode 2` on both swapchains; the FIFO benchmark averaged
  about 80 FPS at 1920×1080 with High textures.

### Cyberpunk mailbox experiment

- With `PEEKGAME_VULKAN_PRESENT=mailbox`, the log confirms two swapchains
  requested FIFO and were overridden (`present mode 2 -> 1`). The reported
  1080p benchmark averaged 21.30 FPS, but its screenshot shows the
  `Ray Tracing: Overdrive` preset, FSR 2.1 Auto, and a 60 FPS cap. The earlier
  ~80 FPS FIFO result used a Custom preset, no resolution scaling, and an
  ~89 FPS cap. These runs do not isolate the effect of present mode.
- Cyberpunk's DXGI layer enables `KHR_present_wait` and sets a three-frame
  latency limit while requesting FIFO. The user reports mailbox is slightly
  faster than FIFO with matched settings; the exact paired benchmark numbers
  were not captured here. The 21.30 FPS Overdrive-preset run cannot quantify
  that difference because its settings differed from the earlier Custom run.

## Cyberpunk 2077 — audio retry loop and benchmark overhead

- **Trigger:** the Cyberpunk run logged thousands of failed `IAudioClient`
  initialization attempts and repeatedly queried the same playback endpoint.
- **Implementation:** endpoint availability is probed once per process instead
  of opening a temporary PulseAudio stream on every device query. The first
  eight `Initialize` calls log the requested format and rejection reason when
  `PEEKGAME_TRACE_AUDIO=1`.
- **Verification:** release build and staged distribution; a live rerun is
  needed to identify the requested format and measure the performance impact.

## Shared WASAPI mix-format correction

- **Trigger:** Cyberpunk repeatedly attempted `IAudioClient::Initialize`
  without starting playback, and an Unreal title also had no sound. The
  repository's actual WASAPI playback test failed at `IsFormatSupported`.
- **Cause:** the advertised 18-byte float32 stereo `WAVEFORMATEX` encoded
  `nAvgBytesPerSec` incorrectly. The byte layout shifted the following
  `nBlockAlign` and `wBitsPerSample` fields, so the format could not parse.
- **Fix:** `src/api/audio/format.rs::MIX_BYTES` now encodes 48 kHz, two channels,
  384000 bytes/s, eight-byte frames, and 32 bits/sample at the correct offsets.
  A unit test checks the fields and parser result.
- **Verification:** `cargo test --offline advertised_mix_format_has_valid_waveformatex_fields`
  and `env PEEKGAME_TRACE_AUDIO=1 python3 tests/test_wasapi.py` pass. The latter
  captured sustained float32, PCM16, and rate-adjust playback through a
  temporary PipeWire null sink. Live Cyberpunk and Unreal game audio pending.

## Cyberpunk 2077 — swapchain surface failure after a run

- **Evidence:** the older `logs/launcher-cyber.log` ends with
  `dxgi_vk_swap_chain_update_formats_locked: Failed to query surface formats`
  and then `STOP: signal 11` (fault address `0x110`). This is a real crash.
- **Boundary:** the newer mailbox log ends with display-metadata fallbacks and
  pipeline-cache writes, without a device-loss or fatal line. The
  `GetDisplayConfigBufferSizes` error does not by itself explain the older
  surface-format failure. The exact user-visible symptom and trigger remain
  to be established.

## REDprelauncher startup, October 19

- **Trigger:** the installed Steam compatibility tool was older than the source and stopped in Qt5Core at UCRT `getenv_s`.
- **Implementation:** the existing secure environment lookup is now compiled and installed; Rust 2024 unsafe blocks were added to its neighboring UCRT functions. `GetLocaleInfo` now returns `LOCALE_ILANGUAGE` as the current four-digit hexadecimal LANGID.
- **Scope:** these changes cover the two observed startup imports. They do not establish launcher, game, or save support.
- **Verification:** release build passed; direct launcher run advanced past `getenv_s` and `GetLocaleInfo(0x1)`, then stopped at unsupported `GetLocaleInfo(0x13)`.
- **Steam launch routing:** `peek-launch` now redirects `REDprelauncher.exe` to `bin/x64/Cyberpunk2077.exe` under the same game root, preserving Steam's environment and game arguments. Shell syntax and an installed-tool invocation with `/bin/echo` confirmed the target. A real Steam launch remains to be checked in the Steam session.

## Saved Games path returned by GetFullPathName

- **Trigger:** the game showed an empty Load Game list while `ManualSave-0` and ten AutoSave directories existed in PeekGame's Saved Games folder.
- **Contract:** `GetFullPathNameW/A` must keep an absolute `C:` path on `C:`. PeekGame previously converted it to the host path and returned a `Z:\home\...` path.
- **Implementation:** canonicalize `C:` paths in Windows form before returning them; leave host path translation to file I/O. File tracing now includes these results and both Win32 and NT directory enumeration.
- **Verification:** focused Rust path test and release build pass; the updated binary was staged and installed. A live Load Game run is still needed to confirm whether this resolves the empty list.

## Cyberpunk save enumeration, October 19

- **Observed call path:** a traced Steam run returned `C:\users\steamuser\Saved Games`, mapped it to the directory containing the saves, then queried `*.dat` (zero matches) and `*.` (only `.` and `..`). The game did not see the extensionless `ManualSave-0` and `AutoSave-*` directories.
- **Cause:** PeekGame's `FindFirstFileExW` wildcard matcher treated the trailing dot in `*.` literally. Windows treats it as an empty extension, so these save directories must match. The earlier `GetFullPathName` fix was a separate API correction and did not resolve the list.
- **Fix:** match trailing-dot patterns against names with no extension. The profile's Windows paths and host directories now share the mapping in `host::paths`, including Saved Games and AppData. `FOLDERID_Profile` now returns the profile rather than Roaming AppData.
- **Verification:** unit tests cover the matcher, an actual `FindFirstFileExW` enumeration of extensionless save directories, and round trips for known folders. A live Load Game run with the rebuilt binary is still needed.
