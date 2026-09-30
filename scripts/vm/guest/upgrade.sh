#!/bin/bash
# Moves the pinned Arch date forward one day (what Zohara Store does after an approval) and upgrades.
# usage (after `sudo -v`): curl -s 10.0.2.2:8000/upgrade.sh | sudo bash -s 2026/09/26
# Posts /tmp/upgrade.log to out/upgrade.log when done.
NEW=${1:?date}
OLD=$(sed -n 's#.*archive.archlinux.org/repos/\([0-9/]*\)/.*#\1#p' /etc/pacman.d/mirrorlist | head -1)
echo "moving pin $OLD -> $NEW"
sed -i "s#$OLD#$NEW#g" /etc/pacman.d/mirrorlist
{
pacman -Sy --noconfirm --needed archlinux-keyring
pacman -Su --noconfirm
echo "pacman exit: $?"
echo "--- snapshots"; snapper --no-dbus -c root list 2>&1 | tail -6
} > /tmp/upgrade.log 2>&1
echo "upgrade finished; posting"
curl -s --data-binary @/tmp/upgrade.log 10.0.2.2:8000/out/upgrade.log >/dev/null && echo posted
