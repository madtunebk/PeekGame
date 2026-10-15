# IOCP and Discord SDK

## Run from the distribution

```sh
cargo build --release --offline
python3 tools/stage_dist.py
./dist/peekgame /path/to/game.exe --run
```

`dist/` contains the runner, optional Linux import library, Windows DXVK DLLs
under `windows-dlls/`, and built DXVK Native libraries under `lib/` when available.
It is ignored by Git. Source dependencies remain in `sources/` and `deps/`;
`target/` is compilation/test output, not the runtime DLL installation directory.

## Common runtime support

- `CreateIoCompletionPort`, `PostQueuedCompletionStatus`, and single/batch
  dequeue, with pointer-sized keys, FIFO packets, timeouts, alertable batch waits,
  duplicated handles and wakeup when the last port handle closes.
- Concurrent-consumer slots are released on the next dequeue, explicit PeekGame
  waits and thread exit. This is not a complete Windows scheduler: accounting for
  arbitrary host blocking calls and Windows' LIFO waiter scheduling remains work.
- Overlapped disk-file completions and per-file notification modes. Duplicate
  file handles share their completion association.
- Local byte-mode named-pipe **clients** connect to real Unix stream sockets.
  Both `\\.\pipe\NAME` and `\\?\pipe\NAME` map to `$XDG_RUNTIME_DIR/NAME`.
  Override the directory with `PEEKGAME_PIPE_DIR` (for example a Flatpak Discord
  IPC directory). There is no emulation of Discord's messages or account state.
- Pipe reads/writes complete through IOCP and/or the OVERLAPPED event. Cancellation
  and close finish outstanding requests with `ERROR_OPERATION_ABORTED`.
- `ExitThread` and `FreeLibraryAndExitThread` work for Windows-created threads;
  the runtime runs TLS/DLL detach, records the exit code and wakes thread waiters.
  Threads without a Windows entry boundary are explicitly unsupported.

Still missing: named-pipe servers/message mode, overlapped Winsock/AFD,
AcceptEx/ConnectEx, full I/O cancellation coverage and general NT file APIs.
Unsupported operations must report failure rather than return fake success.

## Verification

```sh
python3 tests/test_iocp.py
python3 tests/test_iocp_pipe.py
```

The second test uses an isolated Unix peer in `cache/`, never the user's Discord
socket. An optional probe uses the official SDK header (download/extract it
locally into `cache/discord-sdk/c/`) and a user-supplied Windows SDK DLL:

```sh
python3 tools/probe_discord.py --dll /path/to/discord_game_sdk.dll
# Controlled peer test, independent of an installed Discord client:
python3 tests/test_discord_sdk.py --dll /path/to/discord_game_sdk.dll
```

The probe uses application ID `1`, does not publish activities or send chat,
and reports SDK failure results as failures. The real Windows DLL completed
`DiscordCreate`, ten callback polls and destruction against a controlled READY
fixture. This verifies the SDK/transport integration, not a live Discord account.
A subsequent probe also passed against the running native Linux Discord client
(`DiscordCreate` and ten callback polls returned success, then clean destruction).
The direct probe does not test rich-presence updates, invitations or the overlay.

Unsupported KTM transactions now return a real Windows error, allowing Discord
launch-command registration to fail without trapping or pretending to persist it.

Unity also passed Discord DLL startup on private Xvfb; its next observed stop was
`CRYPT32!CertOpenStore`. That HTTPS gap is separate from IOCP/Discord transport.
