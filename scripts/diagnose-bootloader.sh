# Zohara OS - find out why the installer's bootloader step failed.
#
# Run it on the live USB after the installer shows "Bootloader installation
# error": copy this whole file from GitHub, paste it into Konsole with
# Ctrl+Shift+V, press Enter. It prints what it finds and, if it can, also
# saves it on the Ventoy USB as zohara-bootloader-diag.txt so it can be read
# on another computer. It changes nothing on the disk except that one file.
(
SUDO=""
[ "$(id -u)" = 0 ] || SUDO=sudo
out=/tmp/zohara-bootloader-diag.txt

{
    echo "=== Zohara bootloader diagnostics, $(date -u '+%Y-%m-%d %H:%M UTC') ==="

    echo
    echo "=== Booted in UEFI mode? ==="
    if [ -d /sys/firmware/efi ]; then
        echo "yes"
    else
        echo "NO - this USB was started in legacy BIOS mode"
    fi

    echo
    echo "=== Installer log around grub-install ==="
    $SUDO grep -n -B5 -A40 "grub-install" /root/.cache/calamares/session.log 2>&1 | tail -80

    echo
    echo "=== Disks ==="
    lsblk -o NAME,SIZE,FSTYPE,LABEL,PARTTYPENAME

    echo
    echo "=== EFI system partition: free space and contents ==="
    esp=$(lsblk -rno PATH,PARTTYPE | awk 'tolower($2)=="c12a7328-f81f-11d2-ba4b-00a0c93ec93b" {print $1; exit}')
    if [ -z "$esp" ]; then
        echo "No EFI system partition found"
    else
        echo "EFI partition: $esp"
        mnt=$(mktemp -d)
        if $SUDO mount -o ro "$esp" "$mnt"; then
            df -h "$mnt"
            echo
            $SUDO du -sh "$mnt"/EFI/* 2>&1
            $SUDO umount "$mnt"
        else
            echo "Could not mount $esp"
        fi
        rmdir "$mnt"
    fi

    echo
    echo "=== Firmware boot entries ==="
    $SUDO efibootmgr 2>&1 | head -30
} 2>&1 | tee "$out"

# Save a copy on the Ventoy USB. Ventoy sometimes keeps its partition busy
# while an ISO is running; then the mount just fails and nothing is written.
echo
vt=$(lsblk -rno PATH,LABEL | awk '$2=="Ventoy" {print $1; exit}')
saved=no
if [ -n "$vt" ]; then
    vmnt=$(mktemp -d)
    if $SUDO mount "$vt" "$vmnt" 2>/dev/null; then
        $SUDO cp "$out" "$vmnt/zohara-bootloader-diag.txt" && saved=yes
        sync
        $SUDO umount "$vmnt"
    fi
    rmdir "$vmnt"
fi
if [ "$saved" = yes ]; then
    echo ">>> Saved to the USB as zohara-bootloader-diag.txt"
else
    echo ">>> Could not save to the USB. Please take a photo of the output above."
fi
)
