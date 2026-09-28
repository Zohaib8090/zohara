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

# ── Install Zohara Store as a real pacman package ──────────────────────────
# Must run before anything that needs its unit files (the timer enable below).
# The package (built in the Dockerfile, staged by build-iso.sh) owns the binary,
# .desktop, icon and update-check units, so later `pacman -S zohara-store`
# upgrades don't hit "exists in filesystem" conflicts.
ZOHARA_PKGS=()
for f in /root/zohara-settings.pkg.tar.zst /root/zohara-store.pkg.tar.zst /root/zohara-welcome.pkg.tar.zst \
         /root/zohara-voice-model.pkg.tar.zst /root/zohara-voice.pkg.tar.zst /root/zohara-snapshots.pkg.tar.zst; do
    if [[ -f "$f" ]]; then ZOHARA_PKGS+=("$f"); else echo "  !! $f missing; it will not be installed."; fi
done
if (( ${#ZOHARA_PKGS[@]} )); then
    echo "  -> Installing Zohara packages (settings, store, welcome, voice typing, snapshots)..."
    # Inside the build chroot pacman cannot resolve the root mount point, so CheckSpace
    # aborts with "not enough free disk space". Use a one-off config without it.
    grep -v '^CheckSpace' /etc/pacman.conf > /tmp/pacman-nocheckspace.conf
    pacman --config /tmp/pacman-nocheckspace.conf -U --noconfirm --needed "${ZOHARA_PKGS[@]}"
    rm -f /tmp/pacman-nocheckspace.conf "${ZOHARA_PKGS[@]}"
fi

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
# One version everywhere (os-release, lsb-release, the installer's branding),
# taken from the day this image was built. These used to disagree: 2026.08
# here and in the branding, 1.0 in the static os-release/lsb-release files.
ZOHARA_VERSION="$(date -u +%Y.%m.%d)"
cat > /etc/os-release << EOF
NAME="Zohara OS"
PRETTY_NAME="Zohara OS"
ID=zohara
ID_LIKE=arch
VERSION="$ZOHARA_VERSION"
VERSION_ID="$ZOHARA_VERSION"
ZOHARA_CODENAME="Nexus"
BUILD_ID=$ZOHARA_VERSION
ANSI_COLOR="1;36"
HOME_URL="https://github.com/Zohaib8090/zohara"
DOCUMENTATION_URL="https://github.com/Zohaib8090/zohara"
SUPPORT_URL="https://github.com/Zohaib8090/zohara"
BUG_REPORT_URL="https://github.com/Zohaib8090/zohara/issues"
LOGO=distributor-logo-zohara
EOF
cat > /etc/lsb-release << EOF
DISTRIB_ID=Zohara
DISTRIB_RELEASE=$ZOHARA_VERSION
DISTRIB_DESCRIPTION="Zohara OS"
EOF
BRANDING=/etc/calamares/branding/zohara/branding.desc
if [[ -f "$BRANDING" ]]; then
    sed -i -E \
        -e "s/^( *version: *).*/\1$ZOHARA_VERSION/" \
        -e "s/^( *shortVersion: *).*/\1$ZOHARA_VERSION/" \
        -e "s/^( *versionedName: *).*/\1Zohara OS $ZOHARA_VERSION/" \
        -e "s/^( *shortVersionedName: *).*/\1Zohara $ZOHARA_VERSION/" "$BRANDING"
fi
echo "  -> Version $ZOHARA_VERSION."

# ── Work around a Calamares mount-module crash on Btrfs installs ───────────
# Upstream bug (six years old, unresolved: multiple distros hit it, no fix
# landed) -- src/modules/mount/main.py's mount_partition() runs
# `subprocess.check_call(["umount", "-v", root_mount_point])` after creating
# the @/@home/etc. subvolumes, and that verbose umount sometimes dies with
# SIGPIPE instead of exiting cleanly, failing the whole install. Confirmed on
# real hardware (Dell Latitude, fresh partition from a Windows shrink) on the
# 2026-09-28 ISO. Community workaround: drop the -v flag, which is cosmetic
# (it only makes umount print what it did) and not needed here.
MOUNT_MODULE=/usr/lib/calamares/modules/mount/main.py
if [[ -f "$MOUNT_MODULE" ]]; then
    sed -i 's/\["umount", "-v", root_mount_point\]/["umount", root_mount_point]/' "$MOUNT_MODULE"
    if grep -q '"umount", "-v", root_mount_point' "$MOUNT_MODULE"; then
        echo "  -> WARNING: Calamares mount-module umount patch did not match; upstream code changed." >&2
    else
        echo "  -> Patched Calamares mount module (dropped verbose umount, works around upstream SIGPIPE bug)."
    fi
fi

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
# zohara-sync.service (a `pacman -Sy` on every boot) is gone: refreshing the
# package lists outside a full upgrade turns the next single-app install into
# a partial upgrade, and it bypassed the approved-date pinning below. Zohara
# Store is now the only thing that refreshes them, as part of an update.
# Inherited from archiso's own profile (as git symlinks, so they only appear on
# a Linux checkout): iwd, which fights NetworkManager for the Wi-Fi card, and
# sshd, whose archiso drop-in allows root login by password. Removed by path,
# the same way as the networkd links below, since `systemctl disable` does
# nothing when a Windows checkout turned a link into a regular file.
rm -f /etc/systemd/system/multi-user.target.wants/iwd.service \
      /etc/systemd/system/multi-user.target.wants/sshd.service
systemctl disable iwd.service sshd.service 2>/dev/null || true

# Printing: socket-activated CUPS, plus Avahi so network printers are found,
# and mDNS name resolution so their "printer.local" addresses resolve.
systemctl enable cups.socket || true
systemctl enable avahi-daemon.service || true
if ! grep -q "mdns_minimal" /etc/nsswitch.conf; then
    sed -i 's/^\(hosts:.*\) resolve/\1 mdns_minimal [NOTFOUND=return] resolve/' /etc/nsswitch.conf
fi

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

# ── Enable Zohara Store update-check timer for all user sessions ───────────
# Ships from zohara-store-rs/data/*.{service,timer} into
# /usr/lib/systemd/user/ (see Dockerfile step 8 / build-iso.sh step 1 for how
# the zohara-store binary itself lands at /usr/bin/zohara-store). `--global`
# enable is required, not per-user: this script runs once at image build
# time, before any real user account exists, so there is no per-user
# systemd --user instance to enable it against. `--global` instead writes the
# symlink under /etc/systemd/user/, which every user's systemd --user picks
# up on first login -- the same mechanism already used for pipewire above.
echo "  -> Enabling Zohara Store update-check timer..."
systemctl --global enable zohara-store-check-updates.timer || true

# Background health check: notifies the user when a service fails, the disk
# fills up, a restart is needed after an update, etc. (zohara-settings).
echo "  -> Enabling Zohara health-check timer..."
systemctl --global enable zohara-settings-health.timer || true

# Voice typing (Meta+H) types into the focused app through ydotoold, which
# runs per user and reaches /dev/uinput via 70-zohara-uinput.rules.
echo "  -> Enabling ydotool for voice typing..."
systemctl --global enable ydotool.service || true

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
# Stable is on by default, so Zohara Store can find and install Zohara updates.
[zohara-stable]
SigLevel = Optional TrustAll
Server = https://github.com/Zohaib8090/zohara-packages/releases/download/stable
#[zohara-beta]
#SigLevel = Optional TrustAll
#Server = https://github.com/Zohaib8090/zohara-packages/releases/download/channel-beta
#[zohara-alpha]
#SigLevel = Optional TrustAll
#Server = https://github.com/Zohaib8090/zohara-packages/releases/download/channel-alpha
REPO_EOF

# Mark the default channel in /etc/zohara/channel so zohara-channel
# shows a sensible value on first run. The user can change it any time.
mkdir -p /etc/zohara
echo "stable" > /etc/zohara/channel

# ── Chaotic-AUR on the installed system ─────────────────────────────────────
# The build's own pacman.conf (zohara-profile/pacman.conf) is not what ends up
# in the image: mkarchiso keeps it to itself. Without this, everything that
# came from Chaotic-AUR (Brave, yay, the Fluent/Nordzy themes, latte-dock, and
# the Store's VSCodium entry) could never be updated or installed after
# install. Signatures are checked for real here, against chaotic-keyring
# (installed from packages.x86_64 and populated by pacman-init / the
# installer's keyring step), unlike the build's trust-all setting.
if [[ -f /etc/pacman.d/chaotic-mirrorlist ]] && ! grep -q '^\[chaotic-aur\]' /etc/pacman.conf; then
    printf '\n[chaotic-aur]\nInclude = /etc/pacman.d/chaotic-mirrorlist\n' >> /etc/pacman.conf
    echo "  -> Chaotic-AUR registered."
else
    echo "  !! chaotic-mirrorlist missing; Chaotic-AUR not registered."
fi

# ── Pin the official mirrors to the approved date ───────────────────────────
# build-iso.sh writes the date this ISO was built against (the signed
# manifest's approved_date). Writing it in the same format Zohara Store uses
# (the "zohara-approved-date:" marker) means a fresh install starts pinned,
# so the Store's partial-upgrade guard and update check see it that way from
# the first boot instead of following live Arch.
APPROVED_FILE=/root/zohara-approved-date
if [[ -f "$APPROVED_FILE" ]]; then
    d="$(tr -d '[:space:]' < "$APPROVED_FILE")"
    if [[ "$d" =~ ^[0-9]{4}/[0-9]{2}/[0-9]{2}$ ]]; then
        cat > /etc/pacman.d/mirrorlist <<EOF
# Written by the Zohara ISO build. Official packages as they were on $d.
# Zohara only offers updates that were tested, so this date moves forward when they are approved.
# zohara-approved-date: $d
Server = https://archive.archlinux.org/repos/$d/\$repo/os/\$arch
EOF
        echo "  -> Official mirrors pinned to $d."
    else
        echo "  !! Ignoring malformed approved date '$d'; mirrors left unpinned."
    fi
    rm -f "$APPROVED_FILE"
else
    echo "  !! No approved date baked into this build; mirrors left unpinned."
fi

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
