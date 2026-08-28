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
# Incremental build support (Option A):
#   - The pacstrap step (919 packages, ~5 hours) is BY FAR the slowest part.
#   - mkarchiso already tracks completed steps via sentinel files in work/
#     (e.g. work/iso.pacstrap). If work/ exists with valid sentinels and the
#     package list is unchanged, mkarchiso will SKIP pacstrap and only redo
#     the squashfs + ISO steps, which takes ~2-5 minutes.
#   - This script detects whether the package list (or pacman.conf) changed
#     and only wipes work/ in that case. For code-only rebuilds, work/ is
#     preserved and the build is fast.
set -euo pipefail

PROFILE_DIR="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
PROFILE_NAME="$(basename "$PROFILE_DIR")"
WORK_DIR="$PROFILE_DIR/work"
OUT_DIR="$PROFILE_DIR/out"
STAMP_FILE="$WORK_DIR/.zohara-build-stamp"

# 0. Shadow `pacman` with a wrapper that injects --overwrite=/usr/lib/Xorg to
#    work around the xorg-server / xorg-server-common dir-vs-file conflict
#    that otherwise aborts the pacstrap transaction. mkarchiso calls `pacman`
#    by bare name, so placing this dir first on PATH makes pacstrap use it.
_WRAP_DIR="$(mktemp -d)"
ln -sf "$(command -v pacman)" "$_WRAP_DIR/pacman.real"
install -Dm755 "$PROFILE_DIR/pacman-overwrite-xorg" "$_WRAP_DIR/pacman"
export PATH="$_WRAP_DIR:$PATH"

# 1. Stage the prebuilt binaries into the airootfs.
install -Dm755 /opt/build/zohara-settings \
    "$PROFILE_DIR/airootfs/usr/bin/zohara-settings"
install -Dm755 /opt/build/zohara-store \
    "$PROFILE_DIR/airootfs/usr/bin/zohara-store"
rm -f "$PROFILE_DIR/airootfs/usr/local/bin/zohara-settings" \
      "$PROFILE_DIR/airootfs/usr/local/bin/zohara-store"
install -Dm644 /opt/build/zohara-settings.desktop \
    "$PROFILE_DIR/airootfs/usr/share/applications/zohara-settings.desktop"
install -Dm644 /opt/build/zohara-store.desktop \
    "$PROFILE_DIR/airootfs/usr/share/applications/zohara-store.desktop"
rm -rf "$PROFILE_DIR/airootfs/usr/share/zohara-store"

# 2. Decide whether to reuse work/ (fast incremental) or wipe (full rebuild).
#
#    We wipe if:
#      a) work/ doesn't exist (first run)
#      b) packages.x86_64 changed since the last successful build
#      c) pacman.conf changed since the last successful build
#      d) pacman-overwrite-xorg changed (xorg fix logic changed)
#      e) FORCE_FULL=1 is set
#
#    Otherwise we keep work/ and let mkarchiso's sentinels skip pacstrap,
#    turning a 5-hour build into a 2-5 minute rebuild.
WIPE=0
if [[ ! -d "$WORK_DIR" ]]; then
    WIPE=1
    echo "[i] No work/ found -- this is a full build (5 hrs first time)."
elif [[ "${FORCE_FULL:-0}" == "1" ]]; then
    WIPE=1
    echo "[i] FORCE_FULL=1 -- forcing full rebuild."
else
    # Compare current input files to the stamp. If any differ, wipe.
    changed=0
    for f in packages.x86_64 pacman.conf pacman-overwrite-xorg; do
        if [[ -f "$PROFILE_DIR/$f" ]] && [[ "$PROFILE_DIR/$f" -nt "$STAMP_FILE" ]]; then
            echo "[i] $f changed since last build -- full rebuild required."
            changed=1
        fi
    done
    if (( changed )); then
        WIPE=1
    else
        echo "[i] work/ found and inputs unchanged -- incremental build (~5 min)."
    fi
fi

if (( WIPE )); then
    rm -rf "$WORK_DIR"
fi
mkdir -p "$WORK_DIR" "$OUT_DIR"

# 3. Run mkarchiso. The `script -qec` pty wrapper auto-answers pacman provider
#    prompts that would otherwise hang on /dev/tty. `yes "" | ...` ensures any
#    such prompt gets the default answer.
script -qec "yes '' | mkarchiso -v -w '$WORK_DIR' -o '$OUT_DIR' '$PROFILE_DIR/'" /dev/null

# 4. Record the build stamp (mtime = now) so the next run can compare.
touch "$STAMP_FILE"

# 5. Bundle the resulting airootfs into a self-extracting update script.
bash "$PROFILE_DIR/../create_update_bundle.sh"
