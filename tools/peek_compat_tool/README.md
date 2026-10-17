# peek_compat_tool: peek as a Steam compatibility tool

This packages the `peek` native PE loader the same way Proton is packaged, so that Steam itself can launch a Windows game through `peek`, with Steam's full launch environment. It contains no Wine.

**Status:** built and tested without Steam (`./test.sh`). **Nothing is installed and no Steam setting has been changed.**

| File | Role |
|---|---|
| `compatibilitytool.vdf` | Registers the tool. Internal name `peek_native`, `install_path "."`, `from_oslist windows`, `to_oslist linux`. Same layout as Proton's `compatibilitytool.vdf.template`. |
| `toolmanifest.vdf` | `version 2`, `commandline "/peek-launch %verb%"`. It has **no** `require_tool_appid`, so it runs on the host and not inside the Steam Linux Runtime container. Proton requires 1628350, the "sniper" runtime. |
| `peek-launch` | Entry point. It checks the verb and arguments, writes a log, takes a per-app lock, then `exec`s `peek <exe> --run`. |
| `test.sh` | Calls `peek-launch` the way Steam does, with a fake loader and once with the real `../peek` on a synthetic PE. Nothing is installed. |

## How Steam calls it

```
<tool dir>/peek-launch <verb> <game executable> [game arguments...]
```

- Steam fills in `%verb%` and appends the game's command line.
- Each argument is a separate argv element, so paths with spaces arrive intact.
- `peek-launch` never word-splits them. It logs them with `printf %q` and passes `"$exe"` through as a single argument.
- `peek` itself takes the path through `realpath()` and quotes it in the Windows command line.

## Verbs

| Verb | Proton's meaning (`proton`, tag `proton_10.0`, lines 2009-2032) | `peek-launch` |
|---|---|---|
| `waitforexitandrun` | Wait for the prefix's wineserver to exit, then run the game. This is Steam's normal game launch. | **Supported.** It waits on a per-app-id lock (`flock`) held by any previous `peek` session for the same `SteamAppId`, then runs. |
| `run` | Run the game without waiting. | **Supported.** It takes the lock if it is free and never waits. If a session is still running, it logs a note. |
| `getcompatpath` | Convert a Linux path to a Windows path (`winepath -w`). | **Rejected, exit 64.** `peek` has no drive-letter mapping, so there is no correct answer to give. |
| `getnativepath` | Convert a Windows path to a Linux path. | **Rejected, exit 64**, for the same reason. |
| `runinprefix` | Run an arbitrary program in the Wine prefix. | **Rejected, exit 64.** There is no prefix. |
| `destroyprefix` | Delete the prefix's tracked files. | **Rejected, exit 64.** There is no prefix. |
| anything else | `Need a verb.`, exit 1 | **Rejected, exit 64**, as `unknown verb`. |

**Other refusals:**

| Exit | Cause |
|---|---|
| 64 | Missing verb or executable, a path that isn't `.exe`, or extra game arguments. `peek` only accepts `peek EXE [--run]`, so the arguments would otherwise be dropped silently. The refused arguments are printed quoted. |
| 66 | Executable file not found. |
| 69 | Loader missing or not executable. |
| 73 | Log directory or lock file can't be created. |

Every refusal goes to the log and to Steam's original stderr.

## Environment and logging

- **Steam's environment and working directory are passed through unchanged.** `peek-launch` adds and removes nothing. The test checks that `SteamAppId`, `STEAM_COMPAT_DATA_PATH` with a space in it, and a value containing tabs and quotes all reach the loader byte for byte, and that the working directory is kept.
- **Log location:** `${PEEK_LOG_DIR:-${XDG_STATE_HOME:-~/.local/state}/peek-compat}/steam-<SteamAppId>-<time>-<pid>.log`, with a `latest.log` symlink.
- **Log contents:** argv, the working directory, the tool and loader paths, and the Steam-related environment (`Steam*`, `STEAM_*`, `SDL_*`, `DXVK_*`, `VK_*`, display variables, `LD_*`), followed by the loader's own output.
- **Process:** `exec` keeps the PID Steam started, so Steam's Stop button and exit tracking reach `peek` directly. The loader's exit status is Steam's exit status.

| Setting | Default |
|---|---|
| `PEEK_BIN` | `../peek` next to this directory, after resolving symlinks |
| `PEEK_LOG_DIR` | `~/.local/state/peek-compat` |
| `PEEK_TIMEOUT` | No limit. If set, the run uses `timeout --foreground <s>`. |

## Tested (2026-10-13)

`./test.sh` gave 22 of 22 checks passing:
- Both supported verbs.
- All four Proton verbs we don't support, plus an unknown verb, rejected with their messages.
- Missing, wrong-type and extra arguments rejected.
- An executable path with spaces delivered as one argument.
- Environment and working directory preserved.
- The log contents.
- `waitforexitandrun` waiting about 2 s for a running session, and `run` not waiting.
- The real `../peek` launched through `peek-launch` running a synthetic PE (`mov eax,42; ret`) from `…/Steam Library/common/My Game/Fixture Game.exe` and printing `PE entry returned 0x2a`.

**Not tested:** being launched by Steam, since nothing is installed.

## Installing later (not done)

When you decide to try it, link this directory into Steam's tool directory, then restart Steam:

```sh
mkdir -p ~/.steam/root/compatibilitytools.d
ln -s "$PWD/native/peek_compat_tool" ~/.steam/root/compatibilitytools.d/peek_native
```

Then select **"peek native loader (experimental)"** under RE3 → Properties → Compatibility.
- Run the installation commands from the RE3 directory after `make -C native peek`.
- For diagnostics, set RE3's Steam launch options to `STEAM_BRIDGE_TRACE=1 %command%`.
- The symlink keeps `../peek` resolving to the project build.
- The launch log will be in `~/.local/state/peek-compat/latest.log`.

**Next experiment:** launch from Steam and compare the log with the terminal run. Boot success has not been established. The latest terminal trace reports error 126 before the `SteamStart_*` calls; those calls occur in error handling and do not establish the original cause. See `../steam-startup-findings.md` for the current findings; the copied Claude notes describe an earlier investigation.

## Limits

- Game arguments and Steam launch options aren't forwarded yet, because `peek` doesn't accept them. They are rejected rather than dropped.
- The non-ASCII path limit belongs to `peek`; the launcher itself handles any bytes.
- There's no drive mapping, so `getcompatpath`/`getnativepath` can't be answered.
- Steam may call other verbs that aren't listed in Proton's script. Those get exit 64 with `unknown verb` in the log, which is the intended way to find out about them.
