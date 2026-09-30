#!/bin/bash
# Records the boot-relevant state of the installed system and posts it to the host.
# usage: curl -s 10.0.2.2:8000/state.sh | bash -s LABEL
L=${1:-state}
{
echo "=== $L $(date -u +%T) ==="
echo "running kernel: $(uname -r)"
pacman -Q linux-zen linux-zen-headers nvidia-open-dkms grub mkinitcpio 2>&1
echo "--- /boot"; ls -l --time-style=+%T /boot | sed 1d
echo "--- sizes (blocks*512 < size means a hole)"
for f in /boot/vmlinuz-linux-zen /usr/lib/modules/*/vmlinuz; do stat -c '%n size=%s allocated=%b*%B' "$f"; done
echo "--- extents of /boot/vmlinuz-linux-zen"; filefrag -v /boot/vmlinuz-linux-zen | tail -4
echo "--- hashes"; sha256sum /boot/vmlinuz-linux-zen /usr/lib/modules/*/vmlinuz | cut -c1-16,65-
echo "--- initramfs"; ls -l --time-style=+%T /boot/initramfs-linux-zen.img
echo "--- pin"; grep -E 'zohara-approved-date|^Server' /etc/pacman.d/mirrorlist | head -3
echo "--- grub.cfg kernel lines"; grep -E '^\s*(linux|initrd)\s' /boot/grub/grub.cfg | sort | uniq -c | head -6
echo "--- dkms"; dkms status 2>&1 | head -3
} > /tmp/$L.txt 2>&1
curl -s --data-binary @/tmp/$L.txt 10.0.2.2:8000/out/$L.txt >/dev/null && echo "posted $L"
