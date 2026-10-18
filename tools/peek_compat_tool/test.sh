#!/bin/bash
# Exercises peek-launch the way Steam would call it, without Steam: verbs, rejections, paths with
# spaces, environment pass-through, logging, the per-app lock, and one end-to-end run of the real
# ../peek on a synthetic PE fixture (no game files). Nothing is installed.
set -u
cd "$(dirname "$(readlink -f "$0")")"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
LAUNCH=$PWD/peek-launch
pass=0 fail=0
ok() { echo "ok   $1"; pass=$((pass+1)); }
bad() { echo "FAIL $1"; fail=$((fail+1)); }
expect() { # name expected_status actual_status
    [ "$2" = "$3" ] && ok "$1 (exit $3)" || bad "$1: expected exit $2, got $3"
}

GAME="$T/Steam Library/common/My Game"
mkdir -p "$GAME" "$T/logs"
printf 'MZ' >"$GAME/Game Launcher.exe"

# Fake loader: records argv, cwd and selected environment, then exits with $FAKE_STATUS.
cat >"$T/fake-peek" <<'EOF'
#!/bin/bash
printf 'FAKE argc=%d\n' $#; for a in "$@"; do printf 'FAKE arg=[%s]\n' "$a"; done
printf 'FAKE cwd=[%s]\n' "$PWD"
printf 'FAKE SteamAppId=[%s] STEAM_COMPAT_DATA_PATH=[%s] ODD=[%s]\n' "${SteamAppId-}" "${STEAM_COMPAT_DATA_PATH-}" "${ODD-}"
printf 'FAKE LD_LIBRARY_PATH=[%s] LD_PRELOAD=[%s]\n' "${LD_LIBRARY_PATH-}" "${LD_PRELOAD-}"
[ -n "${FAKE_SLEEP:-}" ] && sleep "$FAKE_SLEEP"
exit "${FAKE_STATUS:-0}"
EOF
chmod +x "$T/fake-peek"

run() { # verb args... -> runs from the game dir with a Steam-like environment
    (cd "$GAME" && env PEEK_BIN="$T/fake-peek" PEEK_LOG_DIR="$T/logs" SteamAppId=952060 SteamGameId=952060 \
        STEAM_COMPAT_DATA_PATH="$T/compat data" ODD=$'two  spaces\ttab "quote"' "$LAUNCH" "$@")
}
latest() { cat "$T/logs/latest.log"; }

run waitforexitandrun "$GAME/Game Launcher.exe"; expect "waitforexitandrun launches" 0 $?
log=$(latest)
grep -qF 'FAKE argc=2' <<<"$log" && grep -qF "FAKE arg=[$GAME/Game Launcher.exe]" <<<"$log" \
    && grep -qF 'FAKE arg=[--run]' <<<"$log" && ok "exe path with spaces reaches the loader as one argument" \
    || bad "argument passing: $log"
grep -qF "FAKE cwd=[$GAME]" <<<"$log" && ok "Steam's working directory preserved" || bad "cwd"
grep -qF "STEAM_COMPAT_DATA_PATH=[$T/compat data]" <<<"$log" && grep -qF "ODD=[two  spaces	tab \"quote\"]" <<<"$log" \
    && grep -qF 'SteamAppId=[952060]' <<<"$log" && ok "environment passed through unchanged" || bad "environment"
grep -qE '^argv: waitforexitandrun ' <<<"$log" && grep -qF '  SteamGameId=952060' <<<"$log" \
    && grep -qF '=== loader output' <<<"$log" && ok "log has argv, Steam environment and loader output" || bad "log content"

FAKE_STATUS=7 run run "$GAME/Game Launcher.exe"; expect "run launches and returns the loader's exit status" 7 $?

for v in getcompatpath getnativepath runinprefix destroyprefix bogus; do
    run "$v" "$GAME/Game Launcher.exe" 2>"$T/stderr"; s=$?
    expect "verb $v rejected" 64 $s
    grep -qF "verb '$v'" "$T/stderr" || grep -qF "unknown verb '$v'" "$T/stderr" || bad "$v: no message on stderr"
done
run 2>/dev/null; expect "missing verb rejected" 64 $?
run run 2>/dev/null; expect "missing executable rejected" 64 $?
FAKE_STATUS=0 run run "$GAME/Game Launcher.exe" -dx11 "two words"; expect "extra game arguments accepted" 0 $?
alog=$(latest)
grep -qF 'FAKE arg=[-dx11]' <<<"$alog" && grep -qF 'FAKE arg=[two words]' <<<"$alog" \
    && ok "extra game arguments reach the loader" || bad "argument passthrough: $alog"
run run "$GAME/Missing Game.exe" 2>/dev/null; expect "missing executable file rejected" 66 $?
printf x >"$GAME/readme.txt"; run run "$GAME/readme.txt" 2>/dev/null; expect "non-.exe rejected" 64 $?
(cd "$GAME" && env PEEK_BIN="$T/nope" PEEK_LOG_DIR="$T/logs" "$LAUNCH" run "$GAME/Game Launcher.exe" 2>/dev/null)
expect "missing loader rejected" 69 $?

# Steam-runtime library directories are removed for the loader; other entries and LD_PRELOAD stay.
LD_LIBRARY_PATH="/rt/ubuntu12_32/steam-runtime/pinned_libs_64:/game/dir:/rt/steam-runtime/usr/lib/x86_64-linux-gnu::/opt/lib" \
LD_PRELOAD=/rt/ubuntu12_64/gameoverlayrenderer.so run run "$GAME/Game Launcher.exe" >/dev/null
grep -qF 'FAKE LD_LIBRARY_PATH=[/game/dir:/opt/lib] LD_PRELOAD=[/rt/ubuntu12_64/gameoverlayrenderer.so]' "$T/logs/latest.log" \
    && ok "Steam runtime removed from LD_LIBRARY_PATH, overlay preload kept" \
    || { bad "LD_LIBRARY_PATH filtering"; grep FAKE "$T/logs/latest.log"; }

# waitforexitandrun waits for the previous session of the same app id; run does not.
FAKE_SLEEP=2 run run "$GAME/Game Launcher.exe" &
first=$!; sleep 0.5
start=$(date +%s.%N); run waitforexitandrun "$GAME/Game Launcher.exe"; end=$(date +%s.%N)
wait $first
awk -v a="$start" -v b="$end" 'BEGIN{exit !(b-a >= 1.0)}' && ok "waitforexitandrun waited for the running session" \
    || bad "waitforexitandrun did not wait"
FAKE_SLEEP=2 run run "$GAME/Game Launcher.exe" &
first=$!; sleep 0.5
start=$(date +%s.%N); run run "$GAME/Game Launcher.exe"; end=$(date +%s.%N)
wait $first
awk -v a="$start" -v b="$end" 'BEGIN{exit !(b-a < 1.0)}' && grep -qF 'still holds' "$T/logs/latest.log" \
    && ok "run does not wait and notes the held lock" || bad "run lock behaviour"

# End to end with the real loader on a synthetic PE (mov eax,42; ret), path with spaces.
if [ -x ../peek ]; then
    python3 - "$GAME/Fixture Game.exe" <<'EOF'
import struct, sys
data = bytearray(0x1200); data[:2] = b"MZ"; struct.pack_into("<I", data, 60, 0x80)
data[0x80:0x84] = b"PE\0\0"; struct.pack_into("<HHIIIHH", data, 0x84, 0x8664, 1, 0, 0, 0, 240, 0x22)
o = 0x98; struct.pack_into("<H", data, o, 0x20B); struct.pack_into("<I", data, o + 16, 0x1000)
struct.pack_into("<Q", data, o + 24, 0x140000000); struct.pack_into("<II", data, o + 32, 4096, 512)
struct.pack_into("<II", data, o + 56, 0x2000, 0x200); struct.pack_into("<I", data, o + 108, 16)
s = o + 240; data[s:s + 8] = b".text\0\0\0"; struct.pack_into("<IIII", data, s + 8, 0x1000, 0x1000, 0x1000, 0x200)
struct.pack_into("<I", data, s + 36, 0x60000020); data[0x200:0x206] = b"\xb8\x2a\0\0\0\xc3"
open(sys.argv[1], "wb").write(data)
EOF
    (cd "$GAME" && env PEEK_LOG_DIR="$T/logs" PEEK_TIMEOUT=10 SteamAppId=1 "$LAUNCH" waitforexitandrun "$GAME/Fixture Game.exe")
    s=$?
    expect "real peek via peek-launch" 0 $s
    grep -qF 'PE entry returned 0x2a' "$T/logs/latest.log" && ok "real peek ran the fixture from a path with spaces" \
        || { bad "real peek output"; cat "$T/logs/latest.log"; }
else
    echo "skip real peek: ../peek not built"
fi

echo "passed=$pass failed=$fail"
[ $fail -eq 0 ]
