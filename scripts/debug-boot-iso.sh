#!/usr/bin/env bash
# debug-boot-iso.sh — diagnose systemd-boot AllocatePages failure on a Zohara ISO.
#
# Run from WSL/Linux. Extracts the UEFI FAT image and reports sizes of the
# files that the boot loader is trying to load. A truncated vmlinuz/initramfs
# is the single most common cause of the BS->AllocatePages assertion.
#
# Usage:
#   bash scripts/debug-boot-iso.sh path/to/zohara-os-YYYY.MM.DD-x86_64.iso

set -euo pipefail
ISO="${1:-}"

if [[ -z "$ISO" || ! -f "$ISO" ]]; then
    echo "Usage: $0 <path-to-iso>" >&2
    exit 1
fi

for c in xorriso dd file mdir; do
    command -v "$c" >/dev/null || { echo "missing: $c (apt install xorriso mtools)" >&2; exit 1; }
done

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

echo "==> ISO: $ISO"
echo "    size: $(du -h "$ISO" | cut -f1)"

LBA=$(xorriso -indev "$ISO" -report_el_torito plain 2>/dev/null \
        | awk '/UEFI/ {print $NF; exit}')
BLOCKS=$(xorriso -indev "$ISO" -report_el_torito plain 2>/dev/null \
        | awk '/El Torito img blks/ && $2=="2" {print $NF; exit}')

if [[ -z "$LBA" || -z "$BLOCKS" ]]; then
    echo "!! could not locate UEFI El Torito image" >&2
    exit 1
fi

dd if="$ISO" bs=2048 skip="$LBA" count="$BLOCKS" of="$WORK/efi.img" status=none

echo "==> EFI FAT image: $BLOCKS blocks @ LBA $LBA"
echo
echo "==> /EFI/BOOT tree:"
mdir -i "$WORK/efi.img" ::/EFI/BOOT 2>/dev/null || true
echo
echo "==> /arch/boot/x86_64/ tree:"
mdir -i "$WORK/efi.img" ::/arch/boot/x86_64 2>/dev/null || true
echo
echo "==> /loader/entries/ tree:"
mdir -i "$WORK/efi.img" ::/loader/entries 2>/dev/null || true
echo

# Pull each boot file out and report the actual byte count
extract() {
    local rel="$1"
    local out="$WORK/$(basename "$rel")"
    mcopy -i "$WORK/efi.img" "::/$rel" "$out" 2>/dev/null || {
        echo "    $rel  -- MISSING from EFI image"
        return
    }
    local size
    size=$(stat -c%s "$out")
    local expect_kernel=11000000   # ~11 MiB is a sane vmlinuz-linux-zen floor
    local expect_initrd=40000000   # ~40 MiB is a sane initramfs floor with microcode+archiso
    printf '    %-50s  %10d bytes' "$rel" "$size"
    case "$rel" in
        *vmlinuz*)   [[ $size -lt $expect_kernel ]] && echo "  <-- SUSPICIOUSLY SMALL (kernel)" || echo "  ok" ;;
        *initramfs*) [[ $size -lt $expect_initrd ]] && echo "  <-- SUSPICIOUSLY SMALL (initramfs)" || echo "  ok" ;;
        *) echo ;;
    esac
}

echo "==> Boot file sanity check:"
extract "arch/boot/x86_64/vmlinuz-linux-zen"
extract "arch/boot/x86_64/initramfs-linux-zen.img"
extract "arch/boot/amd-ucode.img"    || true
extract "arch/boot/intel-ucode.img"  || true

echo
echo "==> Loader entries (first 3 lines each):"
mcopy -i "$WORK/efi.img" -s ::/loader/entries "$WORK/entries" 2>/dev/null && \
    for f in "$WORK/entries"/*; do
        echo "---- $(basename "$f") ----"
        head -n 3 "$f"
    done

echo
echo "Done. If vmlinuz or initramfs is missing or suspiciously small, rebuild the ISO."
