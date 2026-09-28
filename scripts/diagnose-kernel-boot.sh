cat > /tmp/zohara-kdiag.sh <<'ZOHARA_KDIAG'
# Zohara OS: the installed system stops at GRUB with "premature end of file
# /@/boot/vmlinuz-linux-zen". Run this from the live USB (not the installed
# system): it finds the installed Zohara, shows what is really in /boot, and
# offers to rewrite the kernel and boot files as plain, uncompressed copies.
# Nothing is changed unless you type yes at the question. It never touches
# Windows or the EFI partition.
#
# Written to an ordinary file first and run from there, so the program that
# reads the script is not also the one reading your answer.

SUDO=""
[ "$(id -u)" = 0 ] || SUDO=sudo
m=$(mktemp -d)

found=""
foundfs=""
for entry in $(lsblk -rno PATH,FSTYPE | awk '$2=="btrfs" || $2=="ext4" {print $1":"$2}'); do
    dev=${entry%%:*}
    fs=${entry##*:}
    if [ "$fs" = btrfs ]; then
        $SUDO mount -o ro,subvol=@ "$dev" "$m" 2>/dev/null || $SUDO mount -o ro "$dev" "$m" 2>/dev/null || continue
    else
        $SUDO mount -o ro "$dev" "$m" 2>/dev/null || continue
    fi
    if [ -f "$m/etc/os-release" ] && ls "$m"/usr/lib/modules/*/vmlinuz >/dev/null 2>&1; then
        found=$dev
        foundfs=$fs
        break
    fi
    $SUDO umount "$m"
done

echo "=== Zohara kernel diagnostics, $(date -u '+%Y-%m-%d %H:%M UTC') ==="
if [ -z "$found" ]; then
    echo "Could not find an installed Zohara system on this computer."
    lsblk -o NAME,SIZE,FSTYPE,LABEL
    exit 1
fi
echo "Installed system: $found ($foundfs), mounted read-only"
echo -n "Mount options: "
findmnt -no OPTIONS "$m"

src=$(ls "$m"/usr/lib/modules/*/vmlinuz | head -1)
dst="$m/boot/vmlinuz-linux-zen"

echo
echo "=== Files in /boot (sizes in bytes) ==="
ls -la "$m/boot" | head -20

echo
echo "=== The kernel: what GRUB loads vs. the copy it was made from ==="
echo "expected size (2026-09-28 ISO): 18522624"
ls -la "$src" "$dst" 2>&1
if [ -f "$dst" ]; then
    if cmp -s "$src" "$dst"; then
        echo "contents: IDENTICAL to the source copy"
    else
        echo "contents: DIFFERENT from the source copy"
    fi
fi

echo
echo "=== How the boot files are stored on disk ==="
if command -v filefrag >/dev/null 2>&1 && [ -f "$dst" ]; then
    $SUDO filefrag -v "$dst" 2>&1 | head -14
    echo "(flag 'encoded' = stored compressed; 'shared' = shares data with another file)"
fi

echo
echo "=== Packages in the installed system ==="
pacman --root "$m" --dbpath "$m/var/lib/pacman" -Q grub linux-zen btrfs-progs 2>&1

echo
echo "=== What grub.cfg tells GRUB to load ==="
grep -n -m6 -E "^[[:space:]]*(linux|initrd)[[:space:]]" "$m/boot/grub/grub.cfg" 2>&1

echo
echo "=== Btrfs messages from this live session ==="
dmesg 2>/dev/null | grep -i -E "btrfs.*(error|csum|corrupt)" | tail -5
echo "(nothing above = none)"

echo
printf 'Rewrite the kernel and boot files as plain copies? Type yes and Enter (anything else stops here): ' > /dev/tty
ans=""
read -r ans < /dev/tty
if [ "$ans" != yes ]; then
    echo "Nothing changed."
    $SUDO umount "$m"
    exit 0
fi

echo "Repairing..."
$SUDO mount -o remount,rw,compress=no "$m" || { echo "Could not make it writable."; $SUDO umount "$m"; exit 1; }
# --sparse=never: the kernel ends in zeros stored as a hole, which GRUB cannot read
$SUDO chattr +m "$m/boot" 2>/dev/null
cp --sparse=never --reflink=never "$src" "$dst.new" && sync && mv -f "$dst.new" "$dst"
for f in initramfs-linux-zen.img initramfs-linux-zen-fallback.img; do
    if [ -f "$m/boot/$f" ]; then
        cp --sparse=never --reflink=never "$m/boot/$f" "$m/boot/$f.new" && sync && mv -f "$m/boot/$f.new" "$m/boot/$f"
    fi
done

isz=$(stat -c %s "$m/boot/initramfs-linux-zen.img" 2>/dev/null || echo 0)
if [ "$isz" -lt 1000000 ]; then
    echo "The initramfs is missing or tiny ($isz bytes): rebuilding it."
    $SUDO arch-chroot "$m" mkinitcpio -p linux-zen < /dev/null
fi
sync

echo
echo "=== After the repair ==="
ls -la "$m/boot" | head -20
if cmp -s "$src" "$dst"; then
    echo "RESULT: kernel copy is identical to the original."
else
    echo "RESULT: kernel copy is STILL different - send a photo of this screen."
fi
$SUDO filefrag -v "$dst" 2>&1 | head -8
$SUDO umount "$m"
echo "Done. Restart the laptop and pick Zohara."
ZOHARA_KDIAG
bash /tmp/zohara-kdiag.sh
