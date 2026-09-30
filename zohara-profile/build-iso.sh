#!/usr/bin/env bash
# Zohara OS - build-iso.sh
#
# Wrapped entrypoint for the zohara-builder image. Splitting this out of the
# Dockerfile's ENTRYPOINT lets us use Docker's exec form (JSON array) instead
# of the shell form -- which means PowerShell and other Windows shells that
# try to interpret the unquoted && chain on `docker run` won't break the
# command.
#
# Three responsibilities:
#   1. Install the prebuilt zohara-settings and zohara-store binaries into
#      the airootfs overlay.
#   2. Run mkarchiso inside a `script -qec` pseudo-tty so that pacman's
#      `Enter a number (default=N)` provider prompts -- which pacman reads
#      from /dev/tty, NOT stdin -- get auto-answered. Without the pty, a
#      build with a virtual package in the dependency tree (e.g.
#      phonon-qt6-vlc vs phonon-qt6-mpv) hangs forever at
#      `checking for file conflicts...` waiting on a human.
#   3. Bundle the resulting airootfs into a self-extracting update script.
#
# Every build starts from an empty work/ (see step 2 for why incremental
# reuse was removed); the package cache is what keeps rebuilds quick.
set -euo pipefail

PROFILE_DIR="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
PROFILE_NAME="$(basename "$PROFILE_DIR")"
WORK_DIR="$PROFILE_DIR/work"
OUT_DIR="$PROFILE_DIR/out"

# 0. Shadow `pacman` with a wrapper that injects --overwrite=/usr/lib/Xorg to
#    work around the xorg-server / xorg-server-common dir-vs-file conflict
#    that otherwise aborts the pacstrap transaction. mkarchiso calls `pacman`
#    by bare name, so placing this dir first on PATH makes pacstrap use it.
_WRAP_DIR="$(mktemp -d)"
ln -sf "$(command -v pacman)" "$_WRAP_DIR/pacman.real"
install -Dm755 "$PROFILE_DIR/pacman-overwrite-xorg" "$_WRAP_DIR/pacman"
export PATH="$_WRAP_DIR:$PATH"

# 1. Stage the prebuilt binaries into the airootfs.
# zohara-settings and zohara-store ship as real pacman packages, installed by
# customize_airootfs.sh via `pacman -U`, so pacman owns their files and the Store
# can update them later. (Rust welcome/migrate are plain files, below.)
install -Dm644 /opt/build/zohara-settings.pkg.tar.zst \
    "$PROFILE_DIR/airootfs/root/zohara-settings.pkg.tar.zst"
# zohara-store ships as a real pacman package, installed by customize_airootfs.sh via
# `pacman -U`, so pacman owns its files and can update it later without conflicts.
install -Dm644 /opt/build/zohara-store.pkg.tar.zst \
    "$PROFILE_DIR/airootfs/root/zohara-store.pkg.tar.zst"
rm -f "$PROFILE_DIR/airootfs/usr/bin/zohara-store" "$PROFILE_DIR/airootfs/usr/bin/zohara-settings" \
      "$PROFILE_DIR/airootfs/usr/share/applications/zohara-settings.desktop" \
      "$PROFILE_DIR/airootfs/usr/lib/systemd/user/zohara-settings-health.service" \
      "$PROFILE_DIR/airootfs/usr/lib/systemd/user/zohara-settings-health.timer" \
      "$PROFILE_DIR/airootfs/usr/local/bin/zohara-settings" \
      "$PROFILE_DIR/airootfs/usr/local/bin/zohara-store"
# Welcome/migrate and voice typing ship as packages too (installed by
# customize_airootfs.sh), so Zohara Store can update them later.
for p in zohara-welcome zohara-voice zohara-voice-model zohara-snapshots; do
    install -Dm644 "/opt/build/$p.pkg.tar.zst" "$PROFILE_DIR/airootfs/root/$p.pkg.tar.zst"
done
# Leftovers from older builds that these packages now own. The package
# installs zohara-welcome/zohara-migrate to /usr/bin (see zohara-welcome's
# PKGBUILD), not /usr/local/bin, so both paths are covered here.
rm -f "$PROFILE_DIR/airootfs/usr/local/bin/zohara-welcome" "$PROFILE_DIR/airootfs/usr/local/bin/zohara-migrate" \
      "$PROFILE_DIR/airootfs/usr/bin/zohara-welcome" "$PROFILE_DIR/airootfs/usr/bin/zohara-migrate"
rm -rf "$PROFILE_DIR/airootfs/usr/share/zohara-store"

# 2. Always start from an empty work/.
#
#    This used to keep work/ between builds ("incremental") and wipe it only
#    when packages.x86_64 / pacman.conf changed. That never worked:
#    mkarchiso marks each finished step, including the whole ISO build mode
#    (work/build._build_buildmode_iso), with a sentinel file, so a kept work/
#    makes it skip *everything* -- it prints "Validating options... Done!",
#    exits 0 and produces no ISO. Seen on a local rebuild 2026-09-27. It also
#    never noticed changes to Zohara's own packages staged above.
#
#    The time goes to downloading packages, not installing them, and the
#    package cache (/var/cache/pacman/pkg, a persistent volume in local
#    builds) keeps those between builds, so a full build from a warm cache
#    stays reasonably quick.
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR" "$OUT_DIR"

# The Arch day this image is built against (from the signed manifest, via the
# Dockerfile's APPROVED_DATE). customize_airootfs.sh writes it into the
# image's mirrorlist so fresh installs start pinned, then deletes this file.
APPROVED_STAGE="$PROFILE_DIR/airootfs/root/zohara-approved-date"
rm -f "$APPROVED_STAGE"
if [[ "${ZOHARA_APPROVED_DATE:-}" =~ ^[0-9]{4}/[0-9]{2}/[0-9]{2}$ ]]; then
    printf '%s\n' "$ZOHARA_APPROVED_DATE" > "$APPROVED_STAGE"
    echo "[i] Pinning the image to Arch as of $ZOHARA_APPROVED_DATE."
else
    echo "[!] No approved date given (ZOHARA_APPROVED_DATE); the image's mirrors stay unpinned."
fi

# 3. Run mkarchiso. The `script -qec` pty wrapper auto-answers pacman provider
#    prompts that would otherwise hang on /dev/tty. `yes "" | ...` ensures any
#    such prompt gets the default answer.
set +e
script -qec "yes '' | mkarchiso -v -w '$WORK_DIR' -o '$OUT_DIR' '$PROFILE_DIR/'" /dev/null
MKARCHISO_RC=$?
set -e
rm -f "$APPROVED_STAGE"
echo "[i] mkarchiso exited with code $MKARCHISO_RC"
if ! ls -lh "$OUT_DIR"/*.iso 2>/dev/null; then
    echo "[!] mkarchiso did NOT produce an ISO in $OUT_DIR/" >&2
    exit 1
fi

# 4. Bundle the resulting airootfs into a self-extracting update script.
bash "$PROFILE_DIR/../scripts/create_update_bundle.sh"
