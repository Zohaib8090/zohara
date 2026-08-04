#!/usr/bin/env bash
# Zohara OS - airootfs post-install customization
# This script runs INSIDE the chroot after all packages are installed.
# Use it for any branding or file replacements that conflict with package files.

set -e

echo "==> Zohara OS: Running post-install customizations..."

# Replace the Plymouth boot spinner watermark with our logo
LOGO_SRC="/etc/calamares/branding/zohara/logo.png"

if [[ -f "$LOGO_SRC" ]]; then
    echo "  -> Replacing Plymouth watermark with Zohara logo..."
    cp "$LOGO_SRC" /usr/share/plymouth/themes/spinner/watermark.png
    cp "$LOGO_SRC" /usr/share/plymouth/themes/spinner/bgrt-fallback.png
    echo "  -> Plymouth logos replaced."
else
    echo "  WARNING: Zohara logo not found at $LOGO_SRC, skipping Plymouth branding."
fi

# Replace the Arch Linux pixmap logo used in some KDE welcome screens
if [[ -d /usr/share/pixmaps ]]; then
    echo "  -> Replacing archlinux-logo pixmap with Zohara logo..."
    cp "$LOGO_SRC" /usr/share/pixmaps/archlinux-logo.png 2>/dev/null || true
fi

echo "==> Zohara OS: Post-install customizations complete."
