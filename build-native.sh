#!/usr/bin/env bash
# build-native.sh -- build the Zohara OS ISO directly on a native Linux install.
#
# This is the recommended path for the user's Linux partition. It avoids
# Docker entirely, runs mkarchiso natively, and is significantly faster
# than the docker-based rebuild_fast.sh (no overlayfs, no bind mounts).
#
# First run:  ~3-5 hours (full pacstrap of 919 packages)
# Each rerun: ~5 minutes (just squashfs + ISO + update bundle, work/ is reused)
#
# Usage:
#   cd ~/zohara
#   bash build-native.sh                       # foreground (blocks terminal)
#   nohup bash build-native.sh > build.log 2>&1 &   # background, survives logout
#
# After the first build succeeds, you can flash the ISO to USB:
#   sudo dd if=out/zohara-os-*.iso of=/dev/sdX bs=4M status=progress oflag=sync

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILE_DIR="$REPO/zohara-profile"
OUT_DIR="$REPO/out"
LOG_FILE="$REPO/build.log"

# --- Sanity checks ---
if ! command -v pacman >/dev/null 2>&1; then
    echo "ERROR: this script must be run on an Arch-based Linux install."
    echo "(pacman command not found.)"
    exit 1
fi

if ! command -v mkarchiso >/dev/null 2>&1; then
    echo "==> Installing archiso (required for building the ISO)..."
    sudo pacman -Syu --noconfirm --needed archiso
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "==> Installing Rust toolchain (required to build the Settings app)..."
    sudo pacman -Syu --noconfirm --needed rust base-devel
fi

if [ ! -d "$PROFILE_DIR" ]; then
    echo "ERROR: $PROFILE_DIR not found."
    echo "Are you in the zohara repo directory? cd into it first."
    exit 1
fi

# --- Build the Rust binaries (only if you changed source) ---
# This step takes ~4 min on first run. Set FORCE_REBUILD=1 to always rebuild.
if [[ "${FORCE_REBUILD:-0}" == "1" ]]; then
    echo "==> FORCE_REBUILD=1 -- building zohara-settings..."
    (cd "$REPO/zohara-settings-rs" && cargo build --release)
    install -Dm755 "$REPO/zohara-settings-rs/target/release/zohara-settings" \
        "$PROFILE_DIR/airootfs/usr/bin/zohara-settings"
    install -Dm644 "$REPO/zohara-settings-rs/data/zohara-settings.desktop" \
        "$PROFILE_DIR/airootfs/usr/share/applications/zohara-settings.desktop"

    echo "==> Building zohara-store..."
    (cd "$REPO/zohara-store-rs" && cargo build --release)
    install -Dm755 "$REPO/zohara-store-rs/target/release/zohara-store" \
        "$PROFILE_DIR/airootfs/usr/bin/zohara-store"
    install -Dm644 "$REPO/zohara-store-rs/data/zohara-store.desktop" \
        "$PROFILE_DIR/airootfs/usr/share/applications/zohara-store.desktop"
else
    echo "[i] Using pre-staged binaries (run with FORCE_REBUILD=1 to rebuild)"
fi

# --- Run the build ---
echo
echo "==> Running mkarchiso to build the ISO..."
echo "    First run: ~3-5 hours (pacoaging 919 packages)"
echo "    Output:    $OUT_DIR/zohara-os-*.iso"
echo

mkdir -p "$OUT_DIR"
bash "$PROFILE_DIR/build-iso.sh" 2>&1 | tee "$LOG_FILE"

echo
echo "==> Build complete. ISO is in $OUT_DIR/"
ls -lh "$OUT_DIR"/*.iso 2>/dev/null || echo "WARNING: no ISO was produced"
