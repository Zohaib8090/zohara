#!/usr/bin/env bash
# Zohara OS - airootfs post-install customization
# This script runs INSIDE the chroot after all packages are installed.
# mkarchiso automatically runs this script from /root/customize_airootfs.sh

set -e

echo "==> Zohara OS: Running post-install customizations..."

LOGO_SRC="/etc/calamares/branding/zohara/logo.png"

# ── Plymouth Boot Logo ─────────────────────────────────────────────────────
if [[ -f "$LOGO_SRC" ]]; then
    echo "  -> Replacing Plymouth watermark with Zohara logo..."
    cp "$LOGO_SRC" /usr/share/plymouth/themes/spinner/watermark.png
    cp "$LOGO_SRC" /usr/share/plymouth/themes/spinner/bgrt-fallback.png
    echo "  -> Plymouth logos replaced."
fi

# Set spinner as the default Plymouth theme
if command -v plymouth-set-default-theme &>/dev/null; then
    echo "  -> Setting Plymouth theme to spinner..."
    plymouth-set-default-theme -R spinner 2>/dev/null || true
fi

# ── Arch Linux Pixmap → Zohara Logo ────────────────────────────────────────
echo "  -> Replacing archlinux-logo pixmap..."
cp "$LOGO_SRC" /usr/share/pixmaps/archlinux-logo.png 2>/dev/null || true

# ── SDDM Theme: Use breeze-dark so it looks more Zohara ────────────────────
echo "  -> Configuring SDDM theme..."
mkdir -p /etc/sddm.conf.d
cat > /etc/sddm.conf.d/10-zohara-theme.conf << 'EOF'
[Theme]
Current=breeze
CursorTheme=breeze_cursors
EOF

# ── Remove any lingering Arch-only branding text ───────────────────────────
# Patch /etc/os-release to make sure it says Zohara everywhere
cat > /etc/os-release << 'EOF'
NAME="Zohara OS"
PRETTY_NAME="Zohara OS"
ID=zohara
ID_LIKE=arch
VERSION="2026.08"
VERSION_ID="2026.08"
BUILD_ID=rolling
ANSI_COLOR="1;36"
HOME_URL="https://github.com/Zohaib8090/zohara"
DOCUMENTATION_URL="https://github.com/Zohaib8090/zohara"
SUPPORT_URL="https://github.com/Zohaib8090/zohara"
BUG_REPORT_URL="https://github.com/Zohaib8090/zohara/issues"
LOGO=distributor-logo-zohara
EOF


# ── Purge unwanted KDE apps from the system ───────────────────────────────
echo "  -> Purging unwanted KDE apps..."
# Only remove packages that are actually installed (avoid noisy errors for absent packages)
for pkg in discover packagekit-qt6; do
    if pacman -Q "$pkg" &>/dev/null; then
        pacman -Rdd --noconfirm "$pkg"
        echo "  -> Removed: $pkg"
    fi
done

# Replace Discover with Zohara Store in the default Plasma task manager pins
LAYOUT_FILE="/usr/share/plasma/layout-templates/org.kde.plasma.desktop.defaultPanel/contents/layout.js"
if [[ -f "$LAYOUT_FILE" ]]; then
    sed -i 's/org.kde.discover.desktop/zohara-store.desktop/g' "$LAYOUT_FILE"
fi

HIDE_APPS=(
    "org.kde.kdeconnect.daemon"
    "org.kde.kdeconnect-handler"
    "org.kde.kdeconnect.settings"
    "org.kde.kdeconnect-indicator"
)

for app in "${HIDE_APPS[@]}"; do
    DESKTOP_FILE="/usr/share/applications/${app}.desktop"
    if [[ -f "$DESKTOP_FILE" ]]; then
        echo "  -> Hiding: $DESKTOP_FILE"
        # Append NoDisplay=true if not already present
        if ! grep -q "^NoDisplay=true" "$DESKTOP_FILE"; then
            echo "NoDisplay=true" >> "$DESKTOP_FILE"
        fi
    fi
done

echo "  -> Launcher cleanup complete."

# ── Setup Zohara OTA Repository ───────────────────────────────────────────────
echo "  -> Configuring Zohara OTA repository..."
cat << 'REPO_EOF' >> /etc/pacman.conf

[zohara]
SigLevel = Optional TrustAll
Server = https://github.com/Zohaib8090/zohara/releases/latest/download
REPO_EOF

echo "==> Zohara OS: Post-install customizations complete."
