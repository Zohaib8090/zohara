#!/usr/bin/env bash
# Zohara OS - build-iso.sh
#
# Wrapped entrypoint for the zohara-builder image. Splitting this out of the
# Dockerfile's ENTRYPOINT lets us use Docker's exec form (JSON array) instead
# of the shell form -- which means PowerShell and other Windows shells that
# try to interpret the unquoted && chain on `docker run` won't break the
# command.
#
# Two responsibilities:
#   1. Install the prebuilt zohara-settings and zohara-store binaries into
#      the airootfs overlay.
#   2. Run mkarchiso inside a `script -qec` pseudo-tty so that pacman's
#      `Enter a number (default=N)` provider prompts -- which pacman reads
#      from /dev/tty, NOT stdin -- get auto-answered. Without the pty, a
#      build with a virtual package in the dependency tree (e.g.
#      phonon-qt6-vlc vs phonon-qt6-mpv) hangs forever at
#      `checking for file conflicts...` waiting on a human.
set -euo pipefail

# 0. Shadow `pacman` with a wrapper that injects --overwrite=/usr/lib/Xorg to
#    work around the xorg-server / xorg-server-common dir-vs-file conflict
#    that otherwise aborts the pacstrap transaction. mkarchiso calls `pacman`
#    by bare name, so placing this dir first on PATH makes pacstrap use it.
_WRAP_DIR="$(mktemp -d)"
ln -sf "$(command -v pacman)" "$_WRAP_DIR/pacman.real"
install -Dm755 /build/zohara-profile/pacman-overwrite-xorg "$_WRAP_DIR/pacman"
export PATH="$_WRAP_DIR:$PATH"

# 1. Stage the prebuilt binaries into the airootfs.
install -Dm755 /opt/build/zohara-settings \
    /build/zohara-profile/airootfs/usr/bin/zohara-settings
install -Dm755 /opt/build/zohara-store \
    /build/zohara-profile/airootfs/usr/bin/zohara-store
rm -f /build/zohara-profile/airootfs/usr/local/bin/zohara-settings \
      /build/zohara-profile/airootfs/usr/local/bin/zohara-store
install -Dm644 /opt/build/zohara-settings.desktop \
    /build/zohara-profile/airootfs/usr/share/applications/zohara-settings.desktop
install -Dm644 /opt/build/zohara-store.desktop \
    /build/zohara-profile/airootfs/usr/share/applications/zohara-store.desktop
rm -rf /build/zohara-profile/airootfs/usr/share/zohara-store

# 2. Clean any stale work/ from a previous aborted run and build the ISO.
rm -rf ./work
# NOTE: xorg-server stays in packages.x86_64 (pulled in via
# plasma-x11-session). The /usr/lib/Xorg conflict with xorg-server-common is
# handled by the pacman wrapper above, NOT by removing packages -- removing
# them broke the package set before.
script -qec 'yes "" | mkarchiso -v -w ./work -o ./out ./zohara-profile/' /dev/null

# 3. Bundle the resulting airootfs into a self-extracting update script.
bash /build/create_update_bundle.sh
