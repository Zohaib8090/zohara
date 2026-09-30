#!/bin/bash
# Checks a freshly installed Zohara system: what is installed, that the two voice engines really differ, and that
# pacman can sync every repo (including zohara-stable) and has nothing broken to do.
# usage (after `sudo -v`):  curl -s 10.0.2.2:8000/check-installed.sh | bash -s LABEL     -> out/ci-LABEL.txt
L=${1:-run}; V=/usr/lib/zohara/whisper
{
echo "=== $L $(date -u +%T) ==="
echo "os-release: $(. /etc/os-release; echo "$PRETTY_NAME $VERSION_ID $BUILD_ID")"; echo "kernel: $(uname -r)"
echo "--- browser"; pacman -Q brave-origin-bin 2>&1; pacman -Q brave-bin 2>&1 | sed 's/^/brave-bin (Brave Browser): /'
echo "--- zohara packages"; pacman -Q | grep -E '^zohara-' 
echo "--- voice engines (must differ: same hash = the AVX2 build has no AVX2)"; sha256sum $V/* | cut -c1-12,65-
echo "--- CPU flags used by Settings to pick one: $(for f in avx avx2 bmi2 fma f16c; do grep -qw -m1 $f /proc/cpuinfo && printf '%s ' $f; done)"
echo "--- pacman repos in pacman.conf: $(grep -E '^\[' /etc/pacman.conf | tr '\n' ' ')"
echo "--- pacman -Sy (real, system's own config)"; sudo pacman -Sy 2>&1 | grep -vE '^\s*$|MiB/s|KiB/s|\[#' | cut -c1-120; echo "exit=${PIPESTATUS[0]}"
echo "--- pacman -Sl zohara-stable"; pacman -Sl zohara-stable 2>&1
echo "--- boot files"; stat -c '%n size=%s allocated=%b*%B' /boot/vmlinuz-linux-zen; filefrag /boot/vmlinuz-linux-zen
echo "--- failed units"; systemctl --failed --no-legend 2>&1 | head -5; echo "(none above = none)"
} > /tmp/ci-$L.txt 2>&1
curl -s --data-binary @/tmp/ci-$L.txt 10.0.2.2:8000/out/ci-$L.txt >/dev/null && echo "posted $L"
