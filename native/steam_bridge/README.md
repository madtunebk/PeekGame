# Steam bridge: Windows x64 steamclient64.dll ABI -> Linux steamclient.so

The Windows `steam_api64` code loads `steamclient64.dll` and calls its exports and
interface vtables with the MSVC x64 ABI. This bridge implements those exports in
Linux code and forwards every call to the real `linux64/steamclient.so`. It does not
authenticate, decide ownership or answer for Steam. Anything it can't translate either
stops with its name or fails the way Steam's own API fails.

```sh
make -C native/steam_bridge steam_bridge.o   # from RE3/: rebuild only the object the loader links
make            # steam_bridge.o, test double, tests, host probe
make test       # ABI tests against fake_steamclient.so
./host_probe    # real Steam client; Steam must be running
```

The probe validates the AppID supplied through `SteamAppId` (default: RE3,
952060). For DOOM Eternal:

```sh
SteamAppId=782330 SteamGameId=782330 STEAM_BRIDGE_TRACE=1 ./host_probe
```

`fake_steamclient.so` is an explicit test dependency used by `make test`.
The live probe uses the real client selected by `STEAM_BRIDGE_CLIENT` or
`~/.steam/sdk64/steamclient.so`; it does not select the test double by default.

## What it covers

| Area | State |
|---|---|
| Exports | `CreateInterface`, `Steam_BGetCallback`, `Steam_FreeLastCallback`, `Steam_GetAPICallResult`, `Steam_ReleaseThreadLocalMemory`: forwarded. `Breakpad_*`: logged, no minidump (a Linux Breakpad would take over the loader's signal handlers). |
| `SteamClient020` | All 42 slots. The pipe/user calls, `RunFrame`, the hooks and `DestroyAllInterfaces` are forwarded. Getters for interface versions that aren't bridged return NULL, which is Steam's own "unavailable". The two `DEPRECATED_*` slots stop. |
| `SteamClient017` | Its own 37-slot vtable, not an alias of 020 (see "Version evidence"). The same methods as 020 are forwarded, the getters return NULL for versions that aren't bridged, and both `DEPRECATED_*` slots stop. |
| `SteamClient014` | Its own 29-slot vtable (see "Version evidence"). The same methods as 017's slots 0-28 are forwarded. `GetISteamUnifiedMessages` (24) and the other getters return NULL for versions that aren't bridged. There's no `Set_SteamAPI_CCheckCallbackRegisteredInProcess`. |
| `SteamUser021` | Handles, `BLoggedOn`, `GetSteamID`, auth session/ticket calls, encrypted app ticket, levels and phone/2FA queries. Voice, `AdvertiseGame`, the store URL, market and duration-control calls stop. |
| `SteamUtils010` | Everything up to `GetAPICallResult`, plus the overlay, language, VR, Big Picture and Deck queries. Gamepad text input, file signature, filter text and launcher mode stop. |
| Other versions | `SteamClient016`, `SteamUser023` and so on: NULL plus a log line. |

Method order comes from the interface tables in Proton's `lsteamclient.dll` installed here
(SDK naming `ISteamX_SteamXNNN_Method`).

### Version evidence (ISteamClient)

Source: Proton Experimental `experimental-11.0-20261008`, installed at
`steamapps/common/Proton - Experimental/files/lib/wine/`.

1. **Method order.** The per-method name strings in `x86_64-windows/lsteamclient.dll` give the order.
2. **Slots.** In `x86_64-unix/lsteamclient.so`, each `ISteamClient_SteamClientNNN_<Method>` thunk calls
   the Linux object through `call *OFFSET(%reg)`, and OFFSET/8 is the slot. This was extracted
   for every method of 017 and 020 and matches step 1. (Symbol address order alone is misleading:
   the hand-written 020 `Set_SteamAPI_CCheckCallbackRegisteredInProcess` sits at the end of the file
   but calls slot 34.)
3. **Signatures.** The thunk bodies of 017 and 020 were compared after normalising slots. Only two differ:
   - `SetLocalIPBinding`: 017 loads a 32-bit IPv4 address (`uint32, uint16`), while 020 passes a
     pointer (`const SteamIPAddress_t &, uint16`).
   - `Set_SteamAPI_CCheckCallbackRegisteredInProcess`: the same function-pointer argument; only
     Proton's implementation differs.

| 020 slots | 017 slots | Difference |
|---|---|---|
| 0-18 | 0-18 | identical, except the `SetLocalIPBinding` (7) signature |
| 19 `GetISteamGameSearch` | none | not in 017 |
| 20-37 | 19-36 | shifted by one (`RunFrame` 19, `SetWarningMessageHook` 21, `Set_..._CCheckCallbackRegistered` 33) |
| 38-41 Input, Parties, RemotePlay, `DestroyAllInterfaces` | none | not in 017 |

**SteamClient014:**
- **Slots:** the call offsets put its 29 methods at the same slots as 017's slots 0-28. Method order
  matches the Windows name strings.
- **Signatures:** for every shared method, the 014 thunk body is identical to 017's, including the
  32-bit `SetLocalIPBinding`. Slot 24 is `GetISteamUnifiedMessages` (017 calls it
  `DEPRECATED_GetISteamUnifiedMessages`). It has the same `(user, pipe, version)` body.
- **Not in 014:** `GetISteamHTMLSurface`, both `DEPRECATED_*` post-result slots,
  `Set_SteamAPI_CCheckCallbackRegisteredInProcess`, Inventory, Video and ParentalSettings.

014 has its own vtable, trap names (`ISteamClient014::...`) and hook trampoline. It reuses the
017 thunks only where the verified bodies are identical.

The shared client thunks are named by their 020 slot and translated per wrapper with
`client_slot()`. Wrappers are cached by (Linux object, version), so one Linux object requested
as 017 and 020 gets two Windows objects, each with its own vtable. Hooks and the
callback-registered check have one trampoline per client version.

### ABI translation

- **Vtables:** every Windows object is a wrapper with its own vtable of `ms_abi` thunks.
  Linux objects never reach Windows code, and the same Linux object always gets the same wrapper.
- **Return values:** `CSteamID` is returned through MSVC's hidden pointer in RDX, and the thunk
  returns that pointer. By value, `CSteamID`/`CGameID` go in a register in both ABIs.
- **Hooks:** `SetWarningMessageHook` and `Set_SteamAPI_CCheckCallbackRegisteredInProcess` get a
  SysV trampoline that calls the Windows function.
- **Callbacks:** `CallbackMsg_t` is 20 bytes on Linux (pack 4) and 24 on Windows. Callback data is
  passed through only for listed layouts: identical ones go as-is, and ones that differ
  (`ValidateAuthTicketResponse_t`, `MicroTxnAuthorizationResponse_t`, `PersonaStateChange_t`)
  are converted. If the Linux size doesn't match, the callback is dropped and logged.
  Call results with unknown layouts return `false` with `*pbFailed=true`.

## Integration with peek.c (for whoever owns the loader)

1. **Build:** link `steam_bridge/steam_bridge.o` into `peek` (it already uses `-ldl -pthread`)
   and `#include "steam_bridge/steam_bridge.h"`.
2. **Module:** in `load_library`, any name whose basename is `steamclient64.dll`
   (case-insensitive, any directory) returns a dedicated `steam_module` token. If
   `steam_bridge_open(NULL)` fails, return NULL with `last_error=126`.
3. **Exports:** in `proc_address`, for `steam_module`, use `steam_bridge_export(name)`. When it
   returns NULL, create the usual diagnostic trap as `steamclient64.dll!name`.
4. **Missing loader API:** `LoadLibraryExA` (the run currently traps on it, `last-run.log:60`).
5. **Registry** `HKCU\Software\Valve\Steam\ActiveProcess`:
   - Open it only if `steam_bridge_steam_pid()` is non-zero. Otherwise return `ERROR_FILE_NOT_FOUND` as today.
   - `RegQueryValueExW`/`A` for `pid` gives `REG_DWORD` with that PID.
   - `SteamClientDll64` gives `REG_SZ`, a path that step 2 recognises, for example `<steam root>/steamclient64.dll`.
   - Don't invent `ActiveUser` or `SteamClientDll`.
6. **Process check:** `OpenProcess` on the Steam PID, then `GetExitCodeProcess` returns 259
   while `kill(pid,0)` succeeds, and the real exit state otherwise. `CloseHandle` must accept the handle.
7. **Environment:** the Linux client reads `SteamAppId`/`SteamGameId` from the real environment.
   `SetEnvironmentVariableA/W` must call `setenv`, or start with `SteamAppId=952060`.
8. **Threads:** the hook trampolines may run on Steam's Linux threads, where GS isn't a TEB.
   This is harmless for simple hooks but needs per-thread TEBs once `CreateThread` exists.

## Results

- **`make test`:** PASS. The SteamClient017 tests cover:
  - slots 0/2/5/7/9/19/20/21/22/33 reaching the right Linux slots;
  - the `uint32` IP;
  - the hook and callback-registered trampolines;
  - wrapper identity across versions and for one shared Linux object;
  - a trap on deprecated slot 31.
- **SteamClient014 tests** cover:
  - slots 0/2/5/7/9/19/21 reaching the right Linux slots;
  - the `uint32` IP;
  - the getters at 24 and 28 returning NULL;
  - the hook;
  - wrapper identity for one Linux object requested as 020, 017 and 014.
- **`./host_probe`** runs SteamClient020, SteamClient017 and then SteamClient014.
- **`./host_probe` in the agent sandbox** (2026-10-13), with the real client, Steam PID 212006,
  gave the same results for both versions:
  - pipe=1, user=1, AppID=952060, logged_on=1, SteamID valid.
  - `UserHasLicenseForApp=2` means `k_EUserHasLicenseResultNoAuth`. That's expected: this API
    checks other users who have an auth session. It isn't how the app checks its own ownership.
  - One callback (304) was delivered. Five were dropped as unknown layouts (336 and internal
    client IDs above 1,000,000).

## Missing / next

### Live connectivity check, 2026-10-16

With Steam running and AppID 782330, the host probe passed for SteamClient020,
017 and 014: pipe=1, user=1, AppID=782330, logged_on=1 and a nonzero SteamID.
The probe previously compared every result against RE3's AppID; it now compares
against the requested AppID and rejects malformed values before loading Steam.
The label for `UserHasLicenseForApp=2` now explains that this peer-auth query
does not establish local ownership.

The sandbox could not see the Steam PID and CreateSteamPipe returned zero;
the same probe outside the sandbox reached the real client successfully.
Callback 304 was delivered; 336 and several internal callbacks remain unbridged.
The ABI regression suite also passed. These results establish client connectivity,
not `SteamAPI_Init`, game ownership, ticket validation or successful game launch.

Inspection of the separate PeekGame Rust project found no bridge references in
its `src/`, `build.rs` or `tools/stage_dist.py`. A copy under `dist/steam_bridge/`
alone does not integrate the bridge with that loader. Loader integration and
the requested game's interface versions still need to be handled there.

- **Interfaces:** `re3.exe` also asks for `SteamFriends017`, `SteamMatchMaking009` and
  `SteamNetworkingMessages002`. `SteamApps`, `SteamRemoteStorage` and `SteamUserStats` are likely next.
  They currently get NULL.
- **Callbacks:** more layouts are needed. 336 showed up in the host run, and each new interface
  brings its own.
- **Other gaps:** game server interfaces, Breakpad and the deprecated slots.
- **Not exercised against real Steam:** 017/014 `SetLocalIPBinding`, because the probe doesn't change
  network binding. It is only checked against the test client.
- **Host checks to repeat** after integrating into peek:
  - `SteamAPI_Init` gets past "did not locate a running instance of Steam".
  - The interface versions the game requests appear in the log.
  - No `STOP: steam_bridge:` lines appear before the window is created.

## Optional interoperability trace

`STEAM_BRIDGE_TRACE=1` logs successful factory requests (version, Linux object,
factory code when requested, Windows wrapper) and CreateSteamPipe / ConnectToGlobalUser
results. It does not change their arguments/results or log authentication payloads.
From RE3, collect a host trace with:

```sh
SteamAppId=952060 STEAM_BRIDGE_TRACE=1 ./native/peek re3.exe --run > native/steam-trace.log 2>&1
```

These diagnostics establish which real-client calls succeed; they do not establish
that Steam API initialization or the game's launch validation has completed.
