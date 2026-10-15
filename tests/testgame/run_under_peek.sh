#!/bin/bash
# Runs testgame.exe under ../../peek on a private Xvfb display with llvmpipe Vulkan (no desktop
# involvement) and checks that it ends with RESULT PASS and exit 0.
set -u
cd "$(dirname "$(readlink -f "$0")")"
LOG=${1:-$(cd ../.. && pwd)/build/run_under_peek.log}
fifo=$(mktemp -u); mkfifo "$fifo"
Xvfb -displayfd 3 -screen 0 1280x800x24 -nolisten tcp 3>"$fifo" >/dev/null 2>&1 &
xvfb=$!
trap 'kill $xvfb 2>/dev/null; rm -f "$fifo"' EXIT
read -r -t 10 display <"$fifo" || { echo "Xvfb did not start"; exit 1; }
DISPLAY=":$display" SDL_VIDEO_DRIVER=x11 VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json DXVK_LOG_PATH=none \
    timeout 60 ../../peek testgame.exe --run >"$LOG" 2>&1
status=$?
grep -E '^(window created|D3D11 device created|readback|frames presented|RESULT|FAIL|STOP)' "$LOG"
if [ $status -eq 0 ] && grep -q '^RESULT PASS' "$LOG"; then echo "PASS: testgame.exe under peek"; exit 0; fi
echo "FAIL: exit $status (log: $LOG)"; exit 1
