#!/bin/bash
set -e

DATE=$(date +%Y.%m.%d)
OUT_DIR="/build/out"
BUNDLE_DIR="/tmp/zohara-update-bundle"

rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/airootfs" "$BUNDLE_DIR/profile"

echo "[+] Packaging complete airootfs system tree and configurations..."

# Copy compiled binaries into airootfs first
# zohara-settings is now built from its own repo (cloned into /build/zohara-settings-rs
# by rebuild_fast.sh / Dockerfile). The path layout is preserved here so the rest
# of the bundle packaging is unchanged.
install -Dm755 /build/zohara-settings-rs/target/release/zohara-settings /build/zohara-profile/airootfs/usr/bin/zohara-settings
install -Dm755 /build/zohara-store-rs/target/release/zohara-store /build/zohara-profile/airootfs/usr/bin/zohara-store
install -Dm644 /build/zohara-settings-rs/data/zohara-settings.desktop /build/zohara-profile/airootfs/usr/share/applications/zohara-settings.desktop
install -Dm644 /build/zohara-store-rs/data/zohara-store.desktop /build/zohara-profile/airootfs/usr/share/applications/zohara-store.desktop

# Copy entire custom airootfs tree
cp -a /build/zohara-profile/airootfs/. "$BUNDLE_DIR/airootfs/"

# Strip build-only and live-ISO-only files from the bundle payload.
#
# install_update.sh below does `cp -a "$DIR/airootfs/." /`, so ANYTHING left in the payload is
# installed onto the user's root filesystem -- and step 4 then `systemctl enable --now`s every unit
# left in multi-user.target.wants/. airootfs/ is a *live ISO* overlay, so most of archiso's own
# scaffolding in there is actively destructive on an installed machine: etc/passwd wipes the user's
# accounts, etc/shadow leaves root with an empty password, the getty/SDDM drop-ins autologin root,
# and mkinitcpio.conf.d/archiso.conf makes the next kernel update build an unbootable initramfs.
#
# scripts/ota-exclude.txt is the shared list of those paths, with the full reasoning and the command
# used to derive it. .github/workflows/build-update.yml applies the same list to the pacman-package
# channel. Keep the two in sync by keeping both pointed at that one file.
#
# Stripped here at staging rather than inside install_update.sh so the files are absent even if
# someone extracts the tarball by hand.
EXCLUDE_LIST=/build/scripts/ota-exclude.txt
if [[ ! -f "$EXCLUDE_LIST" ]]; then
    echo "[-] FATAL: $EXCLUDE_LIST is missing -- refusing to build a bundle that would ship" >&2
    echo "    live-ISO scaffolding (etc/passwd, etc/shadow, root autologin) onto user systems." >&2
    exit 1
fi

echo "[+] Stripping live-ISO-only paths from the update payload..."
stripped=0
while IFS= read -r rel; do
    rel="${rel%%#*}"                       # drop trailing comments
    rel="$(printf '%s' "$rel" | tr -d '[:space:]')"
    [[ -z "$rel" ]] && continue
    case "$rel" in /*|*..*) echo "  !! refusing suspicious entry: $rel" >&2; continue ;; esac
    if [[ -e "$BUNDLE_DIR/airootfs/$rel" || -L "$BUNDLE_DIR/airootfs/$rel" ]]; then
        rm -f "$BUNDLE_DIR/airootfs/$rel"
        stripped=$((stripped + 1))
    fi
done < "$EXCLUDE_LIST"
echo "[+] Stripped $stripped live-ISO-only path(s) from the payload."

# Fail loudly if any of the account/credential files survived -- those are the ones that destroy a
# user's system, so a silently-misapplied exclusion list must not ship.
for critical in etc/passwd etc/shadow root/customize_airootfs.sh \
                etc/systemd/system/getty@tty1.service.d/autologin.conf; do
    if [[ -e "$BUNDLE_DIR/airootfs/$critical" ]]; then
        echo "[-] FATAL: $critical is still in the payload after stripping. Refusing to build." >&2
        exit 1
    fi
done


# Copy package list & pacman config
cp /build/zohara-profile/packages.x86_64 "$BUNDLE_DIR/profile/"
cp /build/zohara-profile/pacman.conf "$BUNDLE_DIR/profile/"

# Create the full-system updater script embedded in the bundle
cat << 'EOF' > "$BUNDLE_DIR/install_update.sh"
#!/bin/bash
set -e

if [ "$EUID" -ne 0 ]; then
  echo "[-] Error: Please run the update script with sudo or as root:"
  echo "    sudo bash zohara-update-*.sh"
  exit 1
fi

echo "=========================================="
echo "    Zohara OS Full System Upgrade         "
echo "=========================================="

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 1. Stop running Zohara utilities while updating
echo "[+] Closing active Zohara applications..."
killall zohara-store zohara-settings 2>/dev/null || true

# 2. Update System Packages (Installs any new/updated packages from release list)
echo "[+] Synchronizing package databases and upgrading system packages..."
pacman -Sy --noconfirm 2>/dev/null || true

if [ -f "$DIR/profile/packages.x86_64" ]; then
  # Filter out comments and blank lines from package list
  PKGS=$(grep -v '^#' "$DIR/profile/packages.x86_64" | grep -v '^\s*$')
  echo "[+] Ensuring all release packages are installed..."
  pacman -S --needed --noconfirm $PKGS 2>/dev/null || true
fi

# 3. Synchronize all customized files, configs, scripts, binaries & themes to /
echo "[+] Applying all system file changes, configurations, and binaries..."
cp -a "$DIR/airootfs/." /

# Clean legacy duplicate locations
rm -f /usr/local/bin/zohara-settings /usr/local/bin/zohara-store

# 4. Enable and start all configured systemd services
echo "[+] Reloading and enabling background system services..."
systemctl daemon-reload 2>/dev/null || true

if [ -d "$DIR/airootfs/etc/systemd/system/multi-user.target.wants" ]; then
  for service in "$DIR/airootfs/etc/systemd/system/multi-user.target.wants"/*.service; do
    if [ -f "$service" ]; then
      SERVICE_NAME=$(basename "$service")
      systemctl enable --now "$SERVICE_NAME" 2>/dev/null || true
    fi
  done
fi

# 5. Refresh Desktop & Icon Caches
echo "[+] Refreshing system caches..."
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true

echo "=========================================="
echo "[✓] Zohara OS System Successfully Upgraded!"
echo "=========================================="

if command -v notify-send >/dev/null 2>&1; then
  notify-send "Zohara OS Upgrade" "Full system update applied successfully!"
fi
EOF

chmod +x "$BUNDLE_DIR/install_update.sh"

# Package as a self-extracting archive in /build/out
TARGET_SCRIPT="$OUT_DIR/zohara-update-$DATE.sh"
LATEST_SCRIPT="$OUT_DIR/zohara-update-latest.sh"

echo "#!/bin/bash" > "$TARGET_SCRIPT"
echo "# Zohara OS Full Offline Upgrade Bundle ($DATE)" >> "$TARGET_SCRIPT"
echo "TMP_DIR=\$(mktemp -d)" >> "$TARGET_SCRIPT"
echo "echo '[+] Extracting Zohara OS full upgrade package...'" >> "$TARGET_SCRIPT"
echo "ARCHIVE_LINE=\$(grep -a -n '^__ARCHIVE_BELOW__' \"\$0\" | cut -d: -f1)" >> "$TARGET_SCRIPT"
echo "tail -n +\$((ARCHIVE_LINE + 1)) \"\$0\" | tar -xz -C \"\$TMP_DIR\"" >> "$TARGET_SCRIPT"
echo "sudo bash \"\$TMP_DIR/install_update.sh\"" >> "$TARGET_SCRIPT"
echo "rm -rf \"\$TMP_DIR\"" >> "$TARGET_SCRIPT"
echo "exit 0" >> "$TARGET_SCRIPT"
echo "__ARCHIVE_BELOW__" >> "$TARGET_SCRIPT"

tar -czf - -C "$BUNDLE_DIR" . >> "$TARGET_SCRIPT"
chmod +x "$TARGET_SCRIPT"
cp "$TARGET_SCRIPT" "$LATEST_SCRIPT"

# ── Generate latest.json (consumed by zohara-settings' OTA page) ─────────
# This file describes the latest published bundle. It must be uploaded to
# the same release as $TARGET_SCRIPT on GitHub Releases, at:
#   https://github.com/Zohaib8090/zohara/releases/latest/download/latest.json
#
# The settings app's updates page GETs this URL on every "Check for
# updates" click and compares its `version` field with the local
# `zohara-settings --version` output.
LOCAL_VERSION=$("${BUNDLE_DIR}/airootfs/usr/bin/zohara-settings" --version 2>/dev/null | awk '{print $2}')
if [[ -z "$LOCAL_VERSION" ]]; then
    # Fallback: if the binary's --version is missing (e.g. an older build
    # was packaged), leave the field empty so the manifest is still
    # valid JSON. The settings app will treat an empty local version as
    # "unknown" and just say "update available" without a strict compare.
    LOCAL_VERSION=""
fi
MANIFEST="$OUT_DIR/latest.json"
BUNDLE_SHA256=$(sha256sum "$TARGET_SCRIPT" | awk '{print $1}')
BUNDLE_SIZE=$(stat -c%s "$TARGET_SCRIPT")
cat > "$MANIFEST" <<EOF
{
  "version": "$LOCAL_VERSION",
  "date": "$DATE",
  "download_url": "https://github.com/Zohaib8090/zohara/releases/latest/download/zohara-update-$DATE.sh",
  "size_bytes": $BUNDLE_SIZE,
  "sha256": "$BUNDLE_SHA256",
  "changelog": "Zohara OS $DATE build. See GitHub release notes for details.",
  "min_zohara_settings_version": "$LOCAL_VERSION"
}
EOF
echo "[✓] Wrote manifest: $MANIFEST"
echo "    (Upload alongside $TARGET_SCRIPT to GitHub Releases)"

echo "[✓] Successfully generated full system upgrade bundle: $TARGET_SCRIPT"
