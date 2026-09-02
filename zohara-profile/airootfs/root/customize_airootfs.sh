#!/usr/bin/env bash
# Zohara OS - airootfs post-install customization
# This script runs INSIDE the chroot after all packages are installed.
# mkarchiso automatically runs this script from /root/customize_airootfs.sh

set -e

echo "==> Zohara OS: Running post-install customizations..."

# ── Ensure all Zohara binaries are executable ──────────────────────────────
# Git sometimes drops execute bits; this guarantees they're always set.
chmod +x /usr/local/bin/zohara-* 2>/dev/null || true
echo "  -> Zohara binaries marked executable."

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
ZOHARA_CODENAME="Nexus"
BUILD_ID=rolling
ANSI_COLOR="1;36"
HOME_URL="https://github.com/Zohaib8090/zohara"
DOCUMENTATION_URL="https://github.com/Zohaib8090/zohara"
SUPPORT_URL="https://github.com/Zohaib8090/zohara"
BUG_REPORT_URL="https://github.com/Zohaib8090/zohara/issues"
LOGO=distributor-logo-zohara
EOF

# ── Enable System Services (Bluetooth & Network) ────────────────────────────
# Enablement happens HERE, not by shipping .wants/ entries in the overlay, because this script runs
# inside arch-chroot on Linux where `systemctl` can create real symlinks. The overlay cannot: it is
# checked out on Windows, which has no SeCreateSymbolicLinkPrivilege, so a hand-placed .wants/ entry
# degrades to a REGULAR FILE holding the whole unit text. That still satisfies the dependency (a
# .wants/ entry is matched by filename alone), which is why it appeared to work -- but a real file at
# /etc/systemd/system/<target>.wants/<unit> also SHADOWS /usr/lib/systemd/system/<unit>, so the frozen
# copy wins forever and later bluez / power-profiles-daemon updates ship units systemd never reads.
#
# Verified in the 2026-08-24 image before this was fixed: bluetooth.target.wants/bluetooth.service was
# the overlay's 759-byte regular file dated 2025-10-08, so the `systemctl enable bluetooth.service`
# below failed with "File already exists" and only managed to create the dbus-org.bluez.service alias.
# power-profiles-daemon was enabled *solely* by its 989-byte overlay copy -- hence the explicit enable
# added here, without which removing that file would have silently disabled it.
echo "  -> Enabling System Services..."
systemctl enable bluetooth.service || true
systemctl enable power-profiles-daemon.service || true
systemctl enable NetworkManager.service || true
systemctl enable zohara-sync.service || true

# `systemctl enable bluetooth` only creates bluetooth.target.wants/, and bluetooth.target is activated
# by udev when an adapter appears (99-systemd.rules: SUBSYSTEM=="bluetooth" -> SYSTEMD_WANTS). The
# retired overlay file additionally forced bluetoothd from multi-user.target; keep that behaviour --
# as a proper symlink this time -- so this change cannot regress the bluetooth fix it came from.
# The unit's ConditionPathIsDirectory=/sys/class/bluetooth makes it a no-op on adapterless machines.
systemctl add-wants multi-user.target bluetooth.service || true

# Prevent boot hangs by disabling network wait services
systemctl mask NetworkManager-wait-online.service systemd-networkd-wait-online.service || true

# Turn off systemd-networkd. Zohara uses NetworkManager, and archiso's profile enables networkd --
# running both makes them fight over the same interfaces.
#
# `systemctl disable systemd-networkd.service` does NOT work here and never did. It only removes
# *symlinks* from .wants/ directories, but this profile is checked out on Windows, which cannot
# create symlinks, so git materializes archiso's .wants/ entries as regular files holding the unit
# text. Verified in the 2026-08-23 image: 8 of the 20 entries in multi-user.target.wants/ are
# regular files, systemd-networkd.service among them at 2428 bytes -- and the `disable` produced no
# "Removed ..." output at all (build log line 3196ff), i.e. it removed nothing. A regular unit file
# in a .wants/ directory still creates the dependency; only the filename matters there.
#
# `rm -f` removes the entry whichever form it takes, so use that instead.
for _nd in /etc/systemd/system/multi-user.target.wants/systemd-networkd.service \
           /etc/systemd/system/sockets.target.wants/systemd-networkd.socket \
           /etc/systemd/system/network-online.target.wants/systemd-networkd-wait-online.service \
           /etc/systemd/system/dbus-org.freedesktop.network1.service; do
    if [[ -e "$_nd" || -L "$_nd" ]]; then
        rm -f "$_nd"
        echo "     removed networkd enablement: $_nd"
    fi
done
unset _nd
systemctl disable systemd-networkd.service systemd-networkd-wait-online.service || true

# ── /etc/resolv.conf ────────────────────────────────────────────────────────
# Deliberately NOT touched here. `arch-chroot` (which mkarchiso uses to run this script) bind-mounts
# the *build host's* /etc/resolv.conf over this path so that pacman can resolve names inside the
# chroot. That makes it an active mountpoint for the entire lifetime of this script, so any attempt to
# replace it fails hard:
#     ln: failed to create symbolic link '/etc/resolv.conf': Device or resource busy
# and with `set -e` above that aborts mkarchiso outright -- no ISO is produced. This was tried on
# 2026-08-24 and killed the build; do not re-add it.
#
# The handoff to systemd-resolved is done instead by /etc/tmpfiles.d/zohara-resolv.conf, which
# systemd-tmpfiles-setup.service applies on the booted system where nothing is bind-mounting the
# path. See that file for the details.

# ── Enable PipeWire Sound Services for all user sessions ────────────────────
echo "  -> Enabling PipeWire audio user services..."
systemctl --global enable pipewire.socket pipewire-pulse.socket || true
systemctl --global enable pipewire.service pipewire-pulse.service wireplumber.service || true

# ── Enable Graphical Boot (SDDM autologin → plasma desktop) ─────────────────
# The sddm enable is deferred until AFTER the post-pacstrap install below,
# because `systemctl enable sddm.service` is a silent no-op until sddm is
# actually installed on the system. Same for any unit that depends on a
# package not yet on disk.
#
# (A placeholder is set here so the order is obvious from the diff.)
echo "  -> Graphical target will be set after xorg/sddm install."

# ── Purge unwanted KDE apps from the system ───────────────────────────────
echo "  -> Purging unwanted KDE apps..."
# Only remove packages that are actually installed (avoid noisy errors for absent packages)
for pkg in discover packagekit-qt6; do
    if pacman -Q "$pkg" &>/dev/null; then
        pacman -Rdd --noconfirm "$pkg"
        echo "  -> Removed: $pkg"
    fi
done

# Force KDE's default pinned apps to point to Zohara equivalents.
#
# These aliases MUST be real files, never symlinks. They were symlinks until 2026-08-23, which
# caused two distinct bugs:
#   1. The HIDE_APPS loop below appends "NoDisplay=true" to each entry it hides, and
#      "systemsettings" is on that list. Both `[[ -f ]]` and `>>` follow symlinks, so the append
#      landed in the *target* — zohara-settings.desktop — hiding Zohara Settings itself from the
#      application launcher. Verified in the 2026-08-23 ISO: NoDisplay=true was the last line of
#      /usr/share/applications/zohara-settings.desktop, so Settings was unreachable from the menu.
#   2. A symlink is still a second menu entry with the same Name=, so the launcher would show
#      duplicate "Settings" / "Software Store" tiles.
# A real alias carrying its own NoDisplay=true fixes both: launching the KDE desktop ID still opens
# the Zohara app, but the alias never shows in the menu and can never be written through.
write_desktop_alias() {
    local alias_id="$1" name="$2" exec_cmd="$3" icon="$4"
    rm -f "/usr/share/applications/${alias_id}.desktop"
    cat > "/usr/share/applications/${alias_id}.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=${name}
Exec=${exec_cmd}
Icon=${icon}
Terminal=false
NoDisplay=true
EOF
}
write_desktop_alias org.kde.discover "Software Store" zohara-store    zohara-store
write_desktop_alias systemsettings   "Settings"       zohara-settings preferences-system
echo "  -> KDE desktop-ID aliases point at Zohara apps (hidden from the menu)."

HIDE_APPS=(
    # KDE System Settings duplicates
    "systemsettings"
    "org.kde.systemsettings"
    "kdesystemsettings"
    # KDE Connect
    "org.kde.kdeconnect.daemon"
    "org.kde.kdeconnect-handler"
    "org.kde.kdeconnect.settings"
    "org.kde.kdeconnect-indicator"
    # Avahi browsers
    "avahi-discover"
    "bssh"
    "bvnc"
    # Qt / Python dev tool launchers
    "assistant"
    "designer"
    "linguist"
    "qdbusviewer"
    "org.kde.ksshaskpass"
    # LibreOffice start center (use individual apps instead)
    "startcenter"
    "libreoffice-startcenter"
)

for app in "${HIDE_APPS[@]}"; do
    DESKTOP_FILE="/usr/share/applications/${app}.desktop"
    # Never write through a symlink: `>>` appends to the *target*, so hiding an alias would hide the
    # real application instead. This is exactly how zohara-settings.desktop acquired NoDisplay=true
    # in the 2026-08-23 ISO, making Zohara Settings invisible in the launcher.
    if [[ -L "$DESKTOP_FILE" ]]; then
        echo "  -> Skipping symlink (append would write through): $DESKTOP_FILE"
        continue
    fi
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
# Zohara packages are published to a dedicated repo at
#   https://github.com/Zohaib8090/zohara-packages/releases
# which holds three release channels (stable / beta / alpha), each as a
# separate GitHub release with its own zohara.db.
#
# The [zohara-*] sections are written DISABLED on purpose. pacman treats
# an unreachable repository database as a fatal error, so registering
# any [zohara-*] repo before its zohara.db is uploaded would make every
# `pacman -Sy` fail and break the system.
#
# The `zohara-channel` script (in /usr/local/bin/) detects when a channel
# has been published, uncomments the right [zohara-*] block, and refreshes
# the database. Run it once after install:
#     sudo zohara-channel set stable
echo "  -> Registering Zohara OTA repositories (channels disabled by default)..."
cat << 'REPO_EOF' >> /etc/pacman.conf

# Zohara OS OTA repositories. Each channel is a separate GitHub release at
# https://github.com/Zohaib8090/zohara-packages/releases. Use the
# `zohara-channel` CLI to enable one (it comments/uncomments these blocks):
#     sudo zohara-channel set stable
#     sudo zohara-channel set beta
#     sudo zohara-channel set alpha
#     sudo zohara-channel list
#[zohara-stable]
#SigLevel = Optional TrustAll
#Server = https://github.com/Zohaib8090/zohara-packages/releases/stable/download
#[zohara-beta]
#SigLevel = Optional TrustAll
#Server = https://github.com/Zohaib8090/zohara-packages/releases/channel-beta/download
#[zohara-alpha]
#SigLevel = Optional TrustAll
#Server = https://github.com/Zohaib8090/zohara-packages/releases/channel-alpha/download
REPO_EOF

# Mark the default channel in /etc/zohara/channel so zohara-channel
# shows a sensible value on first run. The user can change it any time.
mkdir -p /etc/zohara
echo "stable" > /etc/zohara/channel

echo "  -> Updating icon cache..."
gtk-update-icon-cache -f -q /usr/share/icons/hicolor/ || true

# ── Pre-build the dynamic linker cache ──────────────────────────────────────
# Without this, the FIRST boot of the ISO runs `ldconfig` against the entire
# /usr/lib of a 3+ GB squashfs image -- tens of thousands of .so files. systemd
# gates every other early-boot service on ldconfig.service with a 15s default
# timeout, and any service that loses the race (most visibly
# systemd-loop@<iso-device>.service, the archiso loopback attach) gets killed
# with a misleading "FAILED to attach loopback block device" red line.
#
# Pre-computing the cache here makes first-boot ldconfig a no-op (it just
# verifies the on-disk cache and exits in <1s), and it costs almost nothing
# at build time.
echo "  -> Pre-building dynamic linker cache..."
ldconfig

# ── sddm + graphical.target enable ──────────────────────────────────────────
# sddm is now installed in the pacstrap transaction (see packages.x86_64),
# so this enable actually has a unit file to point at. Earlier the enable
# was a silent no-op because the post-pacstrap install was failing with
# "not enough free disk space", so sddm.service never existed on disk.
echo "  -> Enabling sddm and switching to graphical.target..."
systemctl enable sddm.service || true
systemctl set-default graphical.target || true

# Re-run ldconfig that pacman invalidated during pacstrap.
ldconfig

echo "==> Zohara OS: Post-install customizations complete."
