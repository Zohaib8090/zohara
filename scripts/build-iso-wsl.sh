#!/usr/bin/env bash
#
# build-iso-wsl.sh — reproducible Zohara OS ISO build from a Windows checkout, via WSL + Docker.
#
# WHY THIS EXISTS
# ---------------
# The archiso profile contains 42 symlinks under zohara-profile/airootfs/ (systemd .wants/ entries,
# /etc/localtime, /etc/resolv.conf, the pipewire user units). Windows checkouts cannot materialize
# them (no SeCreateSymbolicLinkPrivilege), so git leaves them in one of two broken states on disk:
# either MISSING, or -- worse -- materialized as a REGULAR FILE whose contents are the link target
# text. Building straight from the Windows worktree therefore produces an ISO with no
# display-manager.service (no GUI login) and no sshd.service.
#
# All 42 are pinned with `git update-index --skip-worktree` so they are excluded from the rsync below.
# Until 2026-08-24 only 18 were pinned, and the other 24 were rsynced over the clone's real symlinks
# as regular files. That is what left systemd-networkd enabled in the 2026.08.23 ISO despite
# customize_airootfs.sh disabling it: `systemctl disable` only unlinks symlinks from .wants/
# directories, so a regular unit file sitting there stayed put and kept the dependency alive. The
# assertion in step 2 below now fails the build if a symlink ever loses its pin again.
#
# This script sidesteps that by building from a Linux-side *git clone*, where the symlinks are real,
# then layering the developer's uncommitted Windows edits on top with rsync.
#
# It is idempotent: run it as often as you like. The pacman package cache and the git clone persist
# between runs, so a warm rebuild skips the ~2.7 GiB download.
#
# USAGE
#   wsl -d Ubuntu -- bash /mnt/c/Users/<you>/Documents/zohara/scripts/build-iso-wsl.sh [--image-only]
#
#   --image-only   Build the zohara-builder Docker image but stop before running mkarchiso.
#
set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────
# Windows-side repository (the source of truth for uncommitted edits).
REPO_WIN="${REPO_WIN:-/mnt/c/Users/Zohaib Baig/Documents/zohara}"
# Linux-side build tree. Must live on the WSL ext4 filesystem, NOT under /mnt/c:
# mkarchiso needs symlinks, hardlinks and Unix ownership that drvfs cannot provide.
BUILD_DIR="${BUILD_DIR:-/root/zbuild}"
IMAGE="${IMAGE:-zohara-builder}"

log() { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
die() { printf '\n\033[1;31m!! %s\033[0m\n' "$*" >&2; exit 1; }

[ -d "$REPO_WIN/.git" ] || die "Windows repo not found at: $REPO_WIN (override with REPO_WIN=...)"

# ── 1. Preconditions ──────────────────────────────────────────────────────────
log "Checking prerequisites"
for c in git rsync docker; do
    command -v "$c" >/dev/null || die "missing '$c' — install with: apt-get install -y git rsync docker.io"
done
# docker.service is enabled under WSL's systemd; start it if this is a cold boot.
if ! docker info >/dev/null 2>&1; then
    log "Starting docker"
    systemctl start docker containerd || die "could not start docker"
    sleep 3
    docker info >/dev/null 2>&1 || die "docker still unreachable"
fi

# Refuse to start a second build over a running one: both would write $BUILD_DIR/work and out/,
# and `docker run --name zohara-build` would collide. Exit codes do not propagate here (see step 5),
# so the damage would surface only as a confusing missing-ISO failure much later.
if [ -n "$(docker ps -q --filter "name=^zohara-build$" 2>/dev/null)" ]; then
    die "a build is already running (container 'zohara-build'). Watch it with:
       docker logs -f zohara-build
     or stop it with:
       docker stop zohara-build"
fi

# ── 2. Sync the Windows worktree into a Linux clone ───────────────────────────
# The clone gives us the 42 real symlinks; rsync then overlays local edits.
if [ ! -d "$BUILD_DIR/.git" ]; then
    log "Cloning into $BUILD_DIR (first run)"
    git clone "$REPO_WIN" "$BUILD_DIR"
else
    log "Refreshing existing clone in $BUILD_DIR"
    git -C "$BUILD_DIR" fetch origin --quiet
    git -C "$BUILD_DIR" checkout --quiet -- .
    git -C "$BUILD_DIR" reset --hard --quiet FETCH_HEAD
fi

# Paths git is deliberately ignoring on Windows (the symlinks). Must NOT be rsynced, or the clone's
# real symlinks get clobbered -- either by "missing file" state or by a regular file holding the link
# target as text.
mapfile -t SKIPPED < <(git -C "$REPO_WIN" ls-files -v | grep '^S' | cut -c3-)

# Every symlink in the committed tree MUST be in that pinned set, otherwise rsync silently replaces
# it with the Windows stand-in and the ISO ships a regular file where a symlink belongs. Assert it
# rather than trusting that whoever added the last symlink remembered to pin it.
mapfile -t ALL_LINKS < <(git -C "$REPO_WIN" ls-tree -r HEAD | awk '$1=="120000"{print $4}')
mapfile -t UNPINNED < <(comm -23 \
    <(printf '%s\n' "${ALL_LINKS[@]}" | sort) \
    <(printf '%s\n' "${SKIPPED[@]}" | sort))
if [ "${#UNPINNED[@]}" -gt 0 ]; then
    printf '  unpinned symlink: %s\n' "${UNPINNED[@]}" >&2
    die "${#UNPINNED[@]} committed symlink(s) are not skip-worktree pinned; rsync would clobber them.
     Fix with:  git -C '$REPO_WIN' ls-tree -r HEAD | awk '\$1==\"120000\"{print \$4}' \\
                  | xargs -d '\\n' git -C '$REPO_WIN' update-index --skip-worktree --"
fi
echo "  all ${#ALL_LINKS[@]} committed symlinks are pinned against rsync"
# Tracked files the developer has intentionally deleted. rsync runs without --delete
# (so it cannot remove the symlinks), so these must be re-applied by hand afterwards.
mapfile -t DELETED < <(git -C "$REPO_WIN" ls-files -d)

log "Overlaying local edits (${#SKIPPED[@]} symlinks preserved, ${#DELETED[@]} deletions to re-apply)"
RSYNC_ARGS=(-a --no-perms --no-owner --no-group
            --exclude='.git/' --exclude='work/' --exclude='out/' --exclude='pkg-cache/'
            --exclude='localrepo/' --exclude='target/' --exclude='*.iso')
for p in "${SKIPPED[@]}"; do RSYNC_ARGS+=(--exclude="/$p"); done
# Deliberately no --delete: it would delete the symlinks that are absent on the Windows side.
rsync "${RSYNC_ARGS[@]}" "$REPO_WIN/" "$BUILD_DIR/"

for p in "${DELETED[@]}"; do rm -f "$BUILD_DIR/$p"; done
# Prune only the directories those deletions just emptied (e.g. the retired web-shell store assets
# under usr/share/zohara-store), rather than sweeping all of airootfs.
for p in "${DELETED[@]}"; do
    rmdir -p --ignore-fail-on-non-empty "$(dirname "$BUILD_DIR/$p")" 2>/dev/null || true
done

# ── 3. Verify the tree is actually buildable ──────────────────────────────────
log "Verifying build tree"
missing=0
for p in "${SKIPPED[@]}"; do
    [ -L "$BUILD_DIR/$p" ] || { echo "  MISSING SYMLINK: $p"; missing=1; }
done
[ "$missing" -eq 0 ] || die "symlinks did not materialize — is $BUILD_DIR on a real ext4 filesystem?"
echo "  all ${#SKIPPED[@]} symlinks present"
printf '  display-manager.service -> %s\n' \
    "$(readlink "$BUILD_DIR/zohara-profile/airootfs/etc/systemd/system/display-manager.service")"
bash -n "$BUILD_DIR/zohara-profile/airootfs/root/customize_airootfs.sh" \
    || die "customize_airootfs.sh has a syntax error"
echo "  customize_airootfs.sh parses"

# ── 4. Build the builder image ────────────────────────────────────────────────
log "Building $IMAGE image (compiles calamares + debtap from AUR and both Rust crates)"
docker build -t "$IMAGE" "$BUILD_DIR" 2>&1 | tee /tmp/zohara-image.log | tail -5
docker image inspect "$IMAGE" >/dev/null 2>&1 || die "image build failed — see /tmp/zohara-image.log"

if [ "${1:-}" = "--image-only" ]; then
    log "Image built. Stopping here (--image-only)."
    exit 0
fi

# ── 5. Build the ISO ──────────────────────────────────────────────────────────
# NOTE: do not trust `docker run`'s exit status here. Under WSL + docker.io 29.x it returns 0 even
# when the container fails, so success is determined by the presence of the ISO artifact.
mkdir -p "$BUILD_DIR/out" "$BUILD_DIR/pkg-cache"
rm -rf "$BUILD_DIR/work"          # a stale workdir causes "unable to lock database"

# Move any pre-existing artifacts aside. mkarchiso stamps the ISO with the build date, so a rebuild
# on a later day lands a second file next to the old one and it becomes easy to flash the stale
# image by mistake. Moved, never deleted — previous builds stay recoverable under out/previous/.
shopt -s nullglob
prev=("$BUILD_DIR"/out/*.iso "$BUILD_DIR"/out/zohara-update-*.sh)
if [ ${#prev[@]} -gt 0 ]; then
    mkdir -p "$BUILD_DIR/out/previous"
    log "Archiving ${#prev[@]} artifact(s) from a previous build to out/previous/"
    for f in "${prev[@]}"; do
        printf '  %s\n' "$(basename "$f")"
        mv -f "$f" "$BUILD_DIR/out/previous/"
    done
fi

log "Running mkarchiso (first run downloads ~2.7 GiB; cached afterwards)"
docker run --rm --name zohara-build --privileged \
    -v "$BUILD_DIR:/build" \
    -v "$BUILD_DIR/pkg-cache:/var/cache/pacman/pkg" \
    "$IMAGE" 2>&1 | tee /tmp/zohara-iso.log | grep -E '^\[mkarchiso\]|^error|ERROR' || true

# ── 6. Report ─────────────────────────────────────────────────────────────────
shopt -s nullglob
isos=("$BUILD_DIR"/out/*.iso)
if [ ${#isos[@]} -eq 0 ]; then
    echo
    grep -iE '^error|ERROR:|target not found' /tmp/zohara-iso.log | head -20 || true
    die "no ISO produced — full log at /tmp/zohara-iso.log"
fi
log "Build succeeded"
for f in "${isos[@]}"; do printf '  %s  (%s)\n' "$f" "$(du -h "$f" | cut -f1)"; done
echo
echo "  Reachable directly from Windows (no copy needed) at:"
echo "    \\\\wsl\$\\Ubuntu${BUILD_DIR//\//\\}\\out\\"
echo
echo "  Point Rufus/Etcher straight at that path, or copy it locally first with:"
echo "    cp $BUILD_DIR/out/*.iso '$REPO_WIN/out/'"
