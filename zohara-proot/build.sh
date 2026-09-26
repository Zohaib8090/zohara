#!/usr/bin/env bash
# Builds Zohara for ARM phones (aarch64), to run in Termux through proot-distro.
#
# Runs as root on an aarch64 Linux machine (CI: GitHub's ubuntu-24.04-arm runner).
# Everything happens in chroots made from the official Arch Linux ARM tarball:
#
#   1. build chroot   -> compiles Zohara's packages for aarch64 and makes a
#                        pacman repository of them (out-arm/repo/)
#   2. system chroot  -> Arch Linux ARM + Plasma + those packages, prepared for
#                        proot, packed as out-arm/zohara-rootfs-aarch64.tar.xz
#
# Needs: bsdtar (libarchive-tools), xz, git, curl.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(dirname "$HERE")"
OUT="${OUT:-$REPO/out-arm}"
WORK="${WORK:-/var/tmp/zohara-arm}"
ALARM_URL="${ALARM_URL:-http://os.archlinuxarm.org/os/ArchLinuxARM-aarch64-latest.tar.gz}"
SETTINGS_REPO="${SETTINGS_REPO:-https://github.com/Zohaib8090/zohara-settings.git}"
SETTINGS_BRANCH="${SETTINGS_BRANCH:-main}"
STAMP="$(date -u +%Y%m%d%H%M)"

log() { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
die() { printf '\n\033[1;31m!! %s\033[0m\n' "$*" >&2; exit 1; }

[ "$(id -u)" = 0 ] || die "run as root"
[ "$(uname -m)" = aarch64 ] || die "run on an aarch64 machine (this builds native ARM packages)"

MOUNTED=()
# Background processes started inside the chroot (gpg-agent from pacman-key)
# keep its mounts busy and its sockets in the archive: stop them first.
kill_chroot_procs() {
    local root="$1" p
    for p in /proc/[0-9]*; do
        if [ "$(readlink "$p/root" 2>/dev/null)" = "$root" ]; then kill "${p#/proc/}" 2>/dev/null || true; fi
    done
    sleep 2
}

cleanup() {
    for ((i = ${#MOUNTED[@]} - 1; i >= 0; i--)); do umount -R "${MOUNTED[i]}" 2>/dev/null || umount -lR "${MOUNTED[i]}" 2>/dev/null || true; done
}
trap cleanup EXIT

# ── Chroot helpers ─────────────────────────────────────────────────────────

# new_root DIR: extract Arch Linux ARM into DIR and make it usable as a chroot.
new_root() {
    local root="$1"
    # A leftover from an interrupted run may still have /dev and /sys mounted
    # inside it: unmount first, and never let rm cross into another filesystem.
    umount -R "$root" 2>/dev/null || true
    if findmnt -rn -o TARGET | grep -q "^$root"; then die "$root still has mounts; unmount them first"; fi
    rm -rf --one-file-system "$root"
    mkdir -p "$root"
    bsdtar -xpf "$WORK/alarm.tar.gz" -C "$root"
    # The root must be a mount point, or pacman's disk space check fails.
    mount --bind "$root" "$root"; MOUNTED+=("$root")
    mount -t proc proc "$root/proc"
    mount --rbind /sys "$root/sys"; mount --make-rslave "$root/sys"
    mount --rbind /dev "$root/dev"; mount --make-rslave "$root/dev"
    mount -t tmpfs tmpfs "$root/tmp"
    rm -f "$root/etc/resolv.conf"
    printf 'nameserver 1.1.1.1\nnameserver 8.8.8.8\n' > "$root/etc/resolv.conf"
    # pacman's download sandbox needs kernel features a chroot may not grant.
    grep -q '^DisableSandbox' "$root/etc/pacman.conf" || sed -i 's/^\[options\]$/[options]\nDisableSandbox/' "$root/etc/pacman.conf"
    sed -i 's/^#\?ParallelDownloads.*/ParallelDownloads = 8/' "$root/etc/pacman.conf"
    in_root "$root" 'pacman-key --init && pacman-key --populate archlinuxarm'
    # A phone has its own kernel: drop the board kernel and firmware before
    # upgrading, so no initramfs is ever generated.
    in_root "$root" 'pkgs=$(pacman -Qq | grep -E "^linux-(aarch64|firmware)" || true); [ -z "$pkgs" ] || pacman -Rdd --noconfirm $pkgs'
    in_root "$root" 'pacman -Syu --noconfirm'
}

in_root() {
    local root="$1"; shift
    chroot "$root" /usr/bin/env -i HOME=/root TERM=xterm LANG=C.UTF-8 \
        PATH=/usr/local/sbin:/usr/local/bin:/usr/bin /bin/bash -c "set -e; $*"
}

as_builder() {
    local root="$1"; shift
    chroot "$root" /usr/bin/runuser -u builder -- /usr/bin/env -i HOME=/home/builder TERM=xterm LANG=C.UTF-8 \
        PATH=/home/builder/.cargo/bin:/usr/local/bin:/usr/bin /bin/bash -c "set -e; $*"
}

cargo_version() { grep '^version' "$1/Cargo.toml" | head -1 | cut -d'"' -f2; }

mkdir -p "$WORK" "$OUT"
if [ ! -s "$WORK/alarm.tar.gz" ]; then
    log "Downloading Arch Linux ARM"
    curl -fL --retry 3 -o "$WORK/alarm.tar.gz" "$ALARM_URL"
    curl -fsL --retry 3 -o "$WORK/alarm.tar.gz.md5" "$ALARM_URL.md5"
    (cd "$WORK" && sed 's/ .*/  alarm.tar.gz/' alarm.tar.gz.md5 | md5sum -c -) || die "Arch Linux ARM download is corrupt"
fi

# ── 1. Build Zohara's packages for aarch64 ─────────────────────────────────

B="$WORK/build-root"
log "Preparing the build chroot"
new_root "$B"
in_root "$B" 'pacman -S --noconfirm --needed base-devel git rust cmake pkgconf gtk4 libadwaita glib2 dbus polkit curl'
in_root "$B" 'id builder >/dev/null 2>&1 || useradd -m builder'
# Arch Linux ARM's makepkg may default to .pkg.tar.xz; the repository wants zstd.
in_root "$B" "sed -i -E 's/^#?PKGEXT=.*/PKGEXT=\".pkg.tar.zst\"/' /etc/makepkg.conf && grep -n '^PKGEXT' /etc/makepkg.conf"

mkdir -p "$B/build"
cp -a "$REPO/zohara-store-rs" "$REPO/zohara-welcome" "$REPO/zohara-voice" "$REPO/zohara-voice-model" "$B/build/"
rm -rf "$B/build/zohara-store-rs/target" "$B/build/zohara-welcome/target"
git clone --depth 1 --branch "$SETTINGS_BRANCH" "$SETTINGS_REPO" "$B/build/zohara-settings"
in_root "$B" 'chown -R builder:builder /build'

log "Building zohara-settings"
as_builder "$B" "cd /build/zohara-settings && cargo build --release && cargo test --release && _PKGVER=$(cargo_version "$B/build/zohara-settings").$STAMP makepkg --nodeps --nocheck"
log "Building zohara-store"
as_builder "$B" "cd /build/zohara-store-rs && cargo test --release && _PKGVER=$(cargo_version "$B/build/zohara-store-rs").$STAMP makepkg --nodeps --nocheck"
log "Building zohara-welcome"
as_builder "$B" "cd /build/zohara-welcome && cargo test --release && _PKGVER=$(cargo_version "$B/build/zohara-welcome").$STAMP makepkg --nodeps --nocheck"
log "Building zohara-voice (whisper.cpp for ARM)"
as_builder "$B" 'cd /build/zohara-voice && makepkg --nodeps --nocheck'
log "Building zohara-voice-model"
as_builder "$B" 'cd /build/zohara-voice-model && makepkg --nodeps --nocheck'

log "Making the aarch64 package repository"
rm -rf "$OUT/repo"; mkdir -p "$OUT/repo"
echo "Packages found:"; find "$B/build" -maxdepth 2 -name '*.pkg.tar.*' -printf '  %p\n'
ls "$B"/build/*/*.pkg.tar.zst >/dev/null 2>&1 || die "no .pkg.tar.zst packages were produced (see the list above)"
cp "$B"/build/*/*.pkg.tar.zst "$OUT/repo/"
in_root "$B" 'rm -rf /tmp/repo && mkdir /tmp/repo'
cp "$OUT"/repo/*.pkg.tar.zst "$B/tmp/repo/"
in_root "$B" 'cd /tmp/repo && repo-add -q zohara.db.tar.gz ./*.pkg.tar.zst'
# Real files, not symlinks: GitHub release assets can't be symlinks.
for f in zohara.db zohara.files; do cp -L "$B/tmp/repo/$f" "$OUT/repo/$f"; done
cp "$B/tmp/repo/zohara.db.tar.gz" "$B/tmp/repo/zohara.files.tar.gz" "$OUT/repo/"
ls -lh "$OUT/repo"

# ── 2. The phone system ────────────────────────────────────────────────────

NAME=zohara-rootfs
R="$WORK/$NAME"
log "Preparing the system"
new_root "$R"
mapfile -t WANT < <(grep -v '^\s*#' "$HERE/packages.txt" | awk 'NF {print $1}')
# Optional: names that differ between Plasma releases. Install what exists.
for p in $(grep -v '^\s*#' "$HERE/packages-optional.txt" | awk 'NF {print $1}'); do
    if in_root "$R" "pacman -Si $p >/dev/null 2>&1"; then WANT+=("$p"); else echo "  (skipping $p: not in Arch Linux ARM)"; fi
done
in_root "$R" "pacman -S --noconfirm --needed ${WANT[*]}"

log "Installing Zohara's apps"
mkdir -p "$R/tmp/zohara"
cp "$OUT"/repo/*.pkg.tar.zst "$R/tmp/zohara/"
in_root "$R" 'pacman -U --noconfirm --needed /tmp/zohara/zohara-settings-*.pkg.tar.zst /tmp/zohara/zohara-store-*.pkg.tar.zst /tmp/zohara/zohara-welcome-*.pkg.tar.zst'
# Voice typing (zohara-voice*) is built into the ARM repository but not
# installed here: typing into apps needs /dev/uinput, which proot can't reach,
# and Settings hides it on phones.

log "Applying Zohara's phone configuration"
cp -a "$HERE/rootfs/." "$R/"
chmod 755 "$R/usr/local/bin/"*
# Updates for the phone come from the aarch64 repository.
grep -q '^\[zohara-stable\]' "$R/etc/pacman.conf" || cat >> "$R/etc/pacman.conf" <<'EOF'

[zohara-stable]
SigLevel = Optional TrustAll
Server = https://github.com/Zohaib8090/zohara-packages/releases/download/stable-aarch64
EOF
sed -i 's/^#\(en_US.UTF-8 UTF-8\)/\1/' "$R/etc/locale.gen"
in_root "$R" 'locale-gen >/dev/null'
echo 'LANG=en_US.UTF-8' > "$R/etc/locale.conf"
echo zohara > "$R/etc/hostname"
# Things that need a real kernel or systemd and can't work in proot.
rm -f "$R/etc/xdg/autostart/zohara-privacy-indicator.desktop"

log "Cleaning up"
in_root "$R" 'pacman -Scc --noconfirm >/dev/null; rm -rf /var/cache/pacman/pkg/* /var/lib/pacman/sync/* /tmp/* /root/.cache'
# proot-distro sets these up itself.
rm -f "$R/etc/resolv.conf" "$R/etc/machine-id"
kill_chroot_procs "$R"
rm -f "$R"/etc/pacman.d/gnupg/S.* "$R"/etc/pacman.d/gnupg/.#* 2>/dev/null || true
cleanup; MOUNTED=()
if findmnt -rn -o TARGET | grep -q "^$R"; then die "$R still has mounts; refusing to pack the running system folders"; fi

log "Packing the system"
tar -C "$WORK" --numeric-owner --one-file-system -cpf - "$NAME" | xz -T0 -6 > "$OUT/zohara-rootfs-aarch64.tar.xz"
SHA="$(sha256sum "$OUT/zohara-rootfs-aarch64.tar.xz" | cut -d' ' -f1)"
BASE="https://github.com/Zohaib8090/zohara/releases/download/proot-latest"
sed -e "s|@URL@|$BASE/zohara-rootfs-aarch64.tar.xz|" -e "s|@SHA256@|$SHA|" "$HERE/termux/zohara.sh.in" > "$OUT/zohara.sh"
cp "$HERE/termux/install.sh" "$HERE/termux/zohara" "$OUT/"
ls -lh "$OUT"
log "Done"
