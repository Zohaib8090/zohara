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
# Pre-clean the /usr/lib/Xorg directory conflict between xorg-server and
# xorg-server-common. As of xorg-server 21.1.24, the package ships
# /usr/lib/Xorg/Xorg.wrap (a file) but xorg-server-common ships the
# /usr/lib/Xorg/ directory containing modules/ and protocol.txt. When
# pacstrap installs them in a single transaction, pacman's resolver installs
# xorg-server-common first, which creates the directory, then xorg-server
# fails with "not overwriting dir with file". Removing the dir beforehand
# lets xorg-server-common recreate it (with the proper contents) and then
# xorg-server extracts the wrapper file. Tested with the 2026-08 mirror.
rm -rf /build/work/x86_64/airootfs/usr/lib/Xorg 2>/dev/null || true
script -qec 'yes "" | mkarchiso -v -w ./work -o ./out ./zohara-profile/' /dev/null

# 3. Bundle the resulting airootfs into a self-extracting update script.
bash /build/create_update_bundle.sh
