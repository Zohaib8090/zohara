cat > /tmp/zohara-bootfix.sh <<'ZOHARA_BOOTFIX'
# Zohara OS: the laptop says "no bootable device" for Zohara. Run this from
# the live USB: it shows the laptop's boot list and what is on the EFI
# partition, and on request puts the Zohara bootloader back from inside the
# installed system. Nothing changes unless you type yes. It never touches
# Windows files: the only thing it writes on the EFI partition is the
# EFI/Zohara_OS folder.
#
# Written to a file first and run from there, so the program reading the
# script is not also the one reading your answer.

SUDO=""
[ "$(id -u)" = 0 ] || SUDO=sudo

echo "=== Zohara boot entry check, $(date -u '+%Y-%m-%d %H:%M UTC') ==="
if [ -d /sys/firmware/efi ]; then
    echo "This USB started in UEFI mode: yes"
else
    echo "This USB started in UEFI mode: NO - restart and pick the UEFI entry for the USB"
fi

echo
echo "=== Boot entries stored in the laptop ==="
if command -v efibootmgr >/dev/null 2>&1; then
    $SUDO efibootmgr -v 2>&1 | cut -c1-160 | head -20
else
    echo "efibootmgr is not available here"
fi

esp=$(lsblk -rno PATH,PARTTYPE | awk 'tolower($2)=="c12a7328-f81f-11d2-ba4b-00a0c93ec93b" && $1 ~ /nvme/ {print $1; exit}')
if [ -z "$esp" ]; then
    esp=$(lsblk -rno PATH,PARTTYPE | awk 'tolower($2)=="c12a7328-f81f-11d2-ba4b-00a0c93ec93b" {print $1; exit}')
fi

echo
if [ -z "$esp" ]; then
    echo "=== EFI partition: NOT FOUND ==="
    lsblk -o NAME,SIZE,FSTYPE,PARTTYPENAME
    exit 1
fi
echo "=== EFI partition ($esp): what is on it ==="
e=$(mktemp -d)
if $SUDO mount -o ro "$esp" "$e"; then
    $SUDO find "$e/EFI" -maxdepth 2 -printf '%p  %s bytes\n' 2>&1 | sed "s#$e##" | head -40
    echo
    if $SUDO test -f "$e/EFI/Zohara_OS/grubx64.efi"; then
        echo "Zohara bootloader file (EFI/Zohara_OS/grubx64.efi): PRESENT"
    else
        echo "Zohara bootloader file (EFI/Zohara_OS/grubx64.efi): MISSING"
    fi
    $SUDO umount "$e"
else
    echo "Could not read the EFI partition"
fi

m=$(mktemp -d)
found=""
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
        break
    fi
    $SUDO umount "$m"
done

echo
if [ -z "$found" ]; then
    echo "Could not find the installed Zohara system, so nothing can be repaired."
    exit 1
fi
echo "Installed Zohara system: $found"

echo
printf 'Reinstall the Zohara bootloader onto the EFI partition? Type yes and Enter (anything else stops here): ' > /dev/tty
ans=""
read -r ans < /dev/tty
if [ "$ans" != yes ]; then
    echo "Nothing changed."
    $SUDO umount "$m"
    exit 0
fi

echo "Reinstalling the bootloader..."
$SUDO mount -o remount,rw "$m" || { echo "Could not make the installed system writable."; $SUDO umount "$m"; exit 1; }
$SUDO mkdir -p "$m/boot/efi"
$SUDO mount "$esp" "$m/boot/efi" || { echo "Could not mount the EFI partition."; $SUDO umount "$m"; exit 1; }
$SUDO arch-chroot "$m" grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=Zohara_OS --force < /dev/null
rc=$?
sync

echo
echo "=== After: boot entries stored in the laptop ==="
$SUDO efibootmgr -v 2>&1 | cut -c1-160 | head -20
echo
echo "=== After: Zohara files on the EFI partition ==="
$SUDO find "$m/boot/efi/EFI/Zohara_OS" -printf '%p  %s bytes\n' 2>&1 | sed "s#$m/boot/efi##"
$SUDO umount -R "$m"

echo
if [ "$rc" = 0 ]; then
    echo "RESULT: bootloader reinstalled. Restart, press F12 and pick Zohara_OS."
else
    echo "RESULT: grub-install FAILED (code $rc) - send a photo of this screen."
fi
ZOHARA_BOOTFIX
bash /tmp/zohara-bootfix.sh
