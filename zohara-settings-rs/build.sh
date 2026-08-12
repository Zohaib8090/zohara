#!/bin/bash
# Build script for zohara-settings-rs
# Run this inside the Docker builder container or on Arch

set -e

cd "$(dirname "$0")"

echo "==> Installing build dependencies..."
pacman -S --noconfirm --needed \
    rust \
    gtk4 \
    libadwaita \
    pkgconf \
    dbus \
    base-devel

echo "==> Building zohara-settings (release)..."
cargo build --release

echo "==> Installing binary..."
install -Dm755 target/release/zohara-settings /usr/bin/zohara-settings

echo "==> Installing desktop file..."
install -Dm644 data/zohara-settings.desktop /usr/share/applications/zohara-settings.desktop

echo "==> Done! Run: zohara-settings"
