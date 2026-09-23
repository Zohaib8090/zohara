#!/usr/bin/env bash
# zohara-on-linux.sh -- one-shot setup + ISO build for the Zohara OS Linux partition.
#
# USAGE: Boot into your Arch-based Linux partition, open a terminal, and paste:
#
#   curl -fsSL https://raw.githubusercontent.com/Zohaib8090/zohara/master/zohara-on-linux.sh | bash
#
# Or if you have the zohara repo on Windows and want to use the local checkout:
#   git clone https://github.com/Zohaib8090/zohara.git ~/zohara
#   cd ~/zohara
#   bash zohara-on-linux.sh
#
# What this does:
#   1. Verifies the system is Arch-based (pacman-based)
#   2. Installs build dependencies: archiso, git, base-devel, squashfs-tools, etc.
#   3. Clones the Zohara repo (or uses local copy if present)
#   4. Runs the ISO build with the work/ reuse (Option A) so the next build
#      is fast.
#
# The result lands in ~/zohara/out/zohara-os-*.iso -- ready to flash to USB.

set -euo pipefail

# --- 1. Sanity check: must be running on Arch ---
if ! command -v pacman >/dev/null 2>&1; then
    echo "ERROR: this script requires an Arch-based Linux (pacman not found)."
    echo "If you're already booted into your Linux partition, open a terminal there."
    exit 1
fi

echo "==> Detected Arch-based system. Continuing."
echo

# --- 2. Install build dependencies ---
echo "==> Installing build dependencies (archiso, base-devel, etc.)..."
sudo pacman -Syu --noconfirm --needed \
    archiso \
    base-devel \
    git \
    squashfstools \
    dosfstools \
    libarchive \
    gptfdisk \
    erofs-utils \
    mtools \
    xorriso \
    grub \
    efibootmgr

echo
echo "==> Build dependencies installed."

# --- 3. Get the Zohara source code ---
if [ -d "$HOME/zohara/.git" ]; then
    echo "==> Found existing Zohara checkout at ~/zohara. Pulling latest..."
    cd "$HOME/zohara"
    git pull --ff-only || echo "  (pull failed, continuing with local copy)"
else
    echo "==> Cloning Zohara OS repo to ~/zohara..."
    git clone https://github.com/Zohaib8090/zohara.git "$HOME/zohara"
    cd "$HOME/zohara"
fi

# --- 4. Build the ISO ---
echo
echo "==> Building Zohara OS ISO..."
echo "    First run: ~3-5 hours (full pacstrap on your Linux partition)"
echo "    Output:    ~/zohara/out/zohara-os-*.iso"
echo

cd "$HOME/zohara"
chmod +x rebuild_fast.sh zohara-profile/build-iso.sh zohara-profile/pacman-overwrite-xorg

# Use bash rebuild_fast.sh with no SYNC_MODE so the script returns
# immediately. The build runs in the foreground of this terminal --
# you can Ctrl-C and re-run later, and mkarchiso's sentinels will
# skip work that was already done.
bash rebuild_fast.sh
