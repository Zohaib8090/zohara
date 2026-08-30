#!/usr/bin/env bash
# rebuild_fast.sh -- fast (~5 min) local rebuild of the Zohara OS ISO.
#
# First run: ~3-5 hours (full pacstrap)
# Every subsequent run: ~5 minutes (just squashfs + ISO + update bundle, via work/ reuse)
#
# Restart-survivable: this script launches the docker container with
# `--restart unless-stopped`, so the build survives a host reboot. After
# a reboot you can re-run this script and it will detect the running
# container and just attach to its logs (no duplicate build).
#
# Set FORCE_FULL=1 to force a complete rebuild from scratch.
#
# Monitor from a separate terminal:  docker logs -f zohara-build
# Re-run while build is in progress:  bash rebuild_fast.sh   (attaches to existing)

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="zohara-builder:latest"
CONTAINER_NAME="zohara-build"
LOG_FILE="$REPO/out/.zohara-build.log"

# If a build container is already running, just attach to its logs.
if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "[i] Build container '${CONTAINER_NAME}' is already running."
    echo "[i] Attaching to its log output (Ctrl-C to detach; build continues):"
    echo ""
    exec docker logs -f "${CONTAINER_NAME}"
fi

# If a stopped container exists with our name, remove it so docker run can reuse the name.
docker rm "${CONTAINER_NAME}" 2>/dev/null || true

# Optional: FORCE_FULL wipes work/ and does a full rebuild.
ENV_FLAGS=()
if [[ "${FORCE_FULL:-0}" == "1" ]]; then
    ENV_FLAGS=(-e "FORCE_FULL=1")
    echo "[i] FORCE_FULL=1 set -- this run will wipe work/ and rebuild from scratch."
fi

# Ensure out/ exists for the log mount.
mkdir -p "$REPO/out"

# Launch the build in a restart-survivable container.
# --restart unless-stopped: Docker restarts the container on host reboot.
# --name: stable handle so re-running this script can detect it.
# The docker run is detached (-d) so the script returns immediately.
# The build logs to /build/out/.zohara-build.log which is bind-mounted to the host.
echo "[+] Launching build in container '${CONTAINER_NAME}'..."
echo "[+] Logs: docker logs -f ${CONTAINER_NAME}"
echo "[+] Or:    tail -f $LOG_FILE"
echo ""

docker run -d \
  --name "${CONTAINER_NAME}" \
  --restart unless-stopped \
  --privileged \
  --entrypoint bash \
  -v "$REPO":/build \
  -v "$REPO/out":/build/out \
  "${ENV_FLAGS[@]}" \
  "$IMAGE" -c '
    set -e
    export RUSTUP_HOME=/home/builder/.rustup
    export CARGO_HOME=/home/builder/.cargo
    export PATH=$CARGO_HOME/bin:$PATH
    export HOME=/home/builder

    # 1. Build the Rust binaries from local source. By default we SKIP
    #    the cargo build and use the prebuilt binaries baked into the
    #    docker image at /opt/build/. The first cold cargo compile of
    #    zohara-settings + zohara-store takes ~3 hours (heavy Rust
    #    deps: gtk4, libadwaita, cairo, pango, etc.), so reusing the
    #    prebuilts saves most of the build time on the first run.
    #
    #    Set FORCE_REBUILD=1 if you have actually changed Rust source
    #    and need a fresh build.
    if [[ "${FORCE_REBUILD:-0}" == "1" ]]; then
        echo "[+] FORCE_REBUILD=1 -- rebuilding Rust binaries from source"
        echo "[+] Building zohara-settings (release)..."
        cp -r /build/zohara-settings-rs /tmp/settings
        rm -rf /tmp/settings/target
        (cd /tmp/settings && cargo build --release)
        install -Dm755 /tmp/settings/target/release/zohara-settings /opt/build/zohara-settings
        install -Dm644 /tmp/settings/data/zohara-settings.desktop /opt/build/zohara-settings.desktop

        echo "[+] Building zohara-store (release)..."
        cp -r /build/zohara-store-rs /tmp/store
        rm -rf /tmp/store/target
        (cd /tmp/store && cargo build --release)
        install -Dm755 /tmp/store/target/release/zohara-store /opt/build/zohara-store
        install -Dm644 /tmp/store/data/zohara-store.desktop /opt/build/zohara-store.desktop
    else
        echo "[+] Using prebuilt /opt/build/zohara-settings (set FORCE_REBUILD=1 to rebuild)"
        echo "[+] Using prebuilt /opt/build/zohara-store (set FORCE_REBUILD=1 to rebuild)"
    fi

    # 2. Run the build-iso.sh from this checkout.
    echo "[+] Running ISO build (mkarchiso)..."
    bash /build/zohara-profile/build-iso.sh

    echo "[+] ISO build finished."
    ls -lh /build/out/*.iso 2>/dev/null || echo "WARNING: no ISO produced"
  ' 2>&1 | tee "$LOG_FILE"

echo ""
echo "[+] Build launched in background. To monitor progress:"
echo "    docker logs -f ${CONTAINER_NAME}"
echo "    tail -f $LOG_FILE"

# SYNC_MODE: when set to 1, wait for the build to finish before returning.
# Used by CI so the workflow doesn't proceed to validation until the ISO exists.
if [[ "${SYNC_MODE:-0}" == "1" ]]; then
    echo ""
    echo "[i] SYNC_MODE=1 -- waiting for build to finish..."
    # docker wait blocks until the container exits.
    docker wait "${CONTAINER_NAME}"
    exit_code=$?
    echo ""
    if [ $exit_code -eq 0 ]; then
        echo "[+] Build finished successfully."
    else
        echo "[!] Build failed with exit code $exit_code."
        exit $exit_code
    fi
fi
