#!/bin/bash
# usage (after sudo -v): curl -s 10.0.2.2:8000/test-update.sh | bash
{
echo "=== real update against the live release, system's own /etc/pacman.conf ==="
grep -E '^\[' /etc/pacman.conf | tr '\n' ' '; echo
echo "--- pacman -Sy"; sudo pacman -Sy 2>&1 | grep -vE '^\s*$|MiB/s|KiB/s|\[#' | cut -c1-140; echo "exit=${PIPESTATUS[0]}"
echo "--- pacman -Sl zohara-stable"; pacman -Sl zohara-stable
echo "--- pacman -Su --noconfirm"; sudo pacman -Su --noconfirm 2>&1 | grep -E 'there is nothing|upgrading|installing|error|warning: .*(fail|could)|Total' | cut -c1-140 | head -20; echo "exit=${PIPESTATUS[0]}"
echo "--- after"; pacman -Q zohara-voice zohara-voice-model zohara-settings zohara-welcome zohara-store zohara-snapshots 2>&1
} > /tmp/tu.txt 2>&1
curl -s --data-binary @/tmp/tu.txt 10.0.2.2:8000/out/tu.txt >/dev/null && echo posted
