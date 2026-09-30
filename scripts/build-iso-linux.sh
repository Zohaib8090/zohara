#!/usr/bin/env bash
# Build the Zohara ISO locally on a native Linux machine (Ubuntu, Zorin, Fedora, ...).
# Same image and steps as CI (.github/workflows/build-iso.yml), so the result is an Arch-built ISO.
#
#   scripts/build-iso-linux.sh            build the builder image if needed, then the ISO
#   scripts/build-iso-linux.sh --image    rebuild only the builder image (picks up new app code)
#   scripts/build-iso-linux.sh --clean    also drop the builder image and BuildKit cache first
#
# What persists between runs (so only the first build is slow):
#   - the Docker image zohara-builder (Arch + Calamares + debtap + Zohara apps, all compiled)
#   - BuildKit cache mounts from the Dockerfile: pacman package cache, cargo registry and git
#   - out/zohara-os-x86_64.iso and out/zohara-os-<date>-x86_64.iso in the repo (git-ignored)
# work/ inside zohara-profile is wiped on purpose by build-iso.sh (see GOTCHA.md).
#
# Needs: docker (not podman), curl, git, ~30 GB free. Do not run on a full disk (see the handoff).
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO"

MODE=all
case "${1:-}" in
    "")       ;;
    --image)  MODE=image ;;
    --clean)  MODE=clean ;;
    *)        echo "unknown option: $1 (see the top of this file)" >&2; exit 2 ;;
esac

log() { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
die() { printf '\n\033[1;31m!! %s\033[0m\n' "$*" >&2; exit 1; }

command -v docker >/dev/null || die "docker is not installed: sudo apt install docker.io"

# Right after being added to the docker group, this shell does not have it yet. Re-run under it.
if ! docker info >/dev/null 2>&1; then
    if [ -z "${ZOHARA_IN_SG:-}" ] && id -nG | grep -qw docker; then
        exec env ZOHARA_IN_SG=1 sg docker -c "$(printf '%q ' "$0" "$@")"
    fi
    die "cannot talk to docker. Start it (sudo systemctl start docker) and make sure you are in the docker group."
fi

free_gb=$(df --output=avail -BG "$REPO" | tail -1 | tr -dc '0-9')
[ "$free_gb" -ge 30 ] || die "only ${free_gb} GB free here; the build needs about 30 GB."

if [ "$MODE" = clean ]; then
    log "Removing the builder image and BuildKit cache"
    docker image rm -f zohara-builder:latest 2>/dev/null || true
    docker builder prune -af
fi

log "Resolving inputs the same way CI does"
APPROVED_DATE=$(curl -fsSL https://raw.githubusercontent.com/Zohaib8090/zohara-pipeline/main/manifest.json \
    | sed -n 's/.*"approved_date": *"\([0-9/]*\)".*/\1/p')
[ -n "$APPROVED_DATE" ] || die "could not read approved_date from zohara-pipeline"
SETTINGS_SHA=$(git ls-remote https://github.com/Zohaib8090/zohara-settings.git main | cut -f1)
[ -n "$SETTINGS_SHA" ] || die "could not resolve zohara-settings main"
echo "  Arch snapshot: $APPROVED_DATE"
echo "  zohara-settings: $SETTINGS_SHA"

log "Building the builder image (cached layers are reused)"
DOCKER_BUILDKIT=1 docker build \
    --build-arg "APPROVED_DATE=$APPROVED_DATE" \
    --build-arg "ZOHARA_SETTINGS_SHA=$SETTINGS_SHA" \
    -t zohara-builder:latest .

[ "$MODE" = image ] && { log "Image built. Skipping the ISO."; exit 0; }

log "Building the ISO (25-35 min the first time)"
mkdir -p out zohara-profile/out
rm -f zohara-profile/out/*.iso
SYNC_MODE=1 bash scripts/rebuild_fast.sh

ISO=$(ls -t zohara-profile/out/*.iso 2>/dev/null | head -1 || true)
[ -n "$ISO" ] || die "no ISO was produced. Read the log above (out/.zohara-build.log)."
# Docker ran as root, so hand the files back.
sudo chown -R "$(id -u):$(id -g)" out zohara-profile/out 2>/dev/null || true

log "Done. Proof this is the freshly built file:"
ls -l --time-style=full-iso "$ISO"
sha256sum "$ISO"
echo "  Also copied to out/zohara-os-x86_64.iso"
