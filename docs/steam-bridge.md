# Steam bridge integration

PeekGame builds `native/steam_bridge/steam_bridge.c` into its runtime. No build or runtime dependency on the old sdl3-window tree remains for this bridge. `steamclient64.dll` loads through the bridge; named exports use Windows ABI adapters. Static PE imports use the same resolver. Loaded bridge code is pinned for process lifetime.

Default backend: `~/.steam/sdk64/steamclient.so`. `STEAM_BRIDGE_CLIENT` explicitly overrides it. Failed loading returns error 126; missing exports return 127. The fake backend is only used explicitly by tests and is not part of the runtime build.

## Verification

```
cargo build --release --offline
python3 tools/stage_dist.py
python3 tests/test_steam_bridge.py
```

## Limits

SteamClient020/017/014 and selected SteamUser021/SteamUtils010 methods are bridged. Unsupported methods and callback layouts retain the native bridge diagnostics. See `native/steam_bridge/README.md` for details. This does not establish successful SteamAPI_Init or DOOM startup. Steam registry discovery and process queries may still need integration; DOOM currently stops earlier at HeapCreate. No ownership, authentication, or DRM response is fabricated.
