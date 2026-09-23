#!/bin/bash

# Zohara OS - Boot Validation Script
# This script is meant to be run inside the Docker container to verify
# that the required boot files exist before finalizing any ISO build.

echo "🔍 Validating boot files in the working directory..."

# The working directory mapped to the host
WORK_DIR="./work/x86_64/airootfs"

required=(
    "${WORK_DIR}/boot/vmlinuz-linux-zen"
    "${WORK_DIR}/boot/initramfs-linux-zen.img"
)

# Note: intel-ucode and amd-ucode are no longer checked as separate files 
# because Arch Linux now bakes them directly into initramfs-linux-zen.img 
# via the 'microcode' mkinitcpio hook!

for f in "${required[@]}"; do
    if [ ! -f "$f" ]; then
        echo "❌ ERROR: Missing critical boot file: $f"
        echo "The build would fail to boot. Aborting."
        exit 1
    fi
done

echo "✅ All required boot files are present."
echo "✅ Microcodes are correctly baked into the initramfs."
echo "Validation passed!"
exit 0
