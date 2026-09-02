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
# Set SYNC_MODE=1 to wait for the build to finish (used by CI).
#
# Monitor from a separate terminal:  docker logs -f zohara-build
# Re-run while build is in progress:  bash rebuild_fast.sh   (attaches to existing)

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="zohara-builder:latest"
CONTAINER_NAME="zohara-build"
LOG_FILE="$REPO/out/.zohara-build.log"
SYNC_MODE="${SYNC_MODE:-0}"

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

# Launch the build.
#
# Two modes:
#   - SYNC_MODE=0 (default, for local use): launch detached (-d). Script
#     returns immediately. Use `docker logs -f zohara-build` to watch.
#   - SYNC_MODE=1 (for CI): launch ATTACHED (no -d). The host-side stdout
#     shows the build output in real time, and the script blocks until
#     the build finishes. This is what makes `2>&1 | tee` actually work.
if [[ "${SYNC_MODE}" == "1" ]]; then
    echo "[+] Launching build (attached, will block until complete)..."
    echo "[+] Streaming output to stdout and to: $LOG_FILE"
    echo ""

    set +e
    docker run --rm \
      --name "${CONTAINER_NAME}" \
      --privileged \
      --entrypoint bash \
      -v "$REPO":/build \
      -v "$REPO/out":/build/out \
      "${ENV_FLAGS[@]}" \
      "$IMAGE" -c '
        set -e
        # Force line-buffered output so the host-side sees progress in
        # real time and a killed build leaves the partial log readable.
        exec 1> >(stdbuf -oL cat)
        exec 2> >(stdbuf -oL cat >&2)
        export RUSTUP_HOME=/home/builder/.rustup
        export CARGO_HOME=/home/builder/.cargo
        export PATH=$CARGO_HOME/bin:$PATH
        export HOME=/home/builder

        if [[ "${FORCE_REBUILD:-0}" == "1" ]]; then
            echo "[+] FORCE_REBUILD=1 -- rebuilding Rust binaries from source"
            # zohara-settings is now a separate repo. Clone it into /build
            # before running cargo so the rest of this script finds it at
            # the same /build/zohara-settings-rs path.
            if [ ! -d /build/zohara-settings-rs ]; then
                echo "[+] Cloning zohara-settings standalone repo..."
                git clone --depth 1 https://github.com/Zohaib8090/zohara-settings.git /build/zohara-settings-rs
            fi
            (cd /build/zohara-settings-rs && cargo build --release)
            install -Dm755 /build/zohara-settings-rs/target/release/zohara-settings /opt/build/zohara-settings
            install -Dm644 /build/zohara-settings-rs/data/zohara-settings.desktop /opt/build/zohara-settings.desktop

            (cd /build/zohara-store-rs && cargo build --release)
            install -Dm755 /build/zohara-store-rs/target/release/zohara-store /opt/build/zohara-store
            install -Dm644 /build/zohara-store-rs/data/zohara-store.desktop /opt/build/zohara-store.desktop
        else
            echo "[+] Using prebuilt /opt/build/zohara-settings (set FORCE_REBUILD=1 to rebuild)"
            echo "[+] Using prebuilt /opt/build/zohara-store (set FORCE_REBUILD=1 to rebuild)"
        fi

        echo "[+] Running ISO build (mkarchiso)..."
        bash /build/zohara-profile/build-iso.sh

        echo "[+] ISO build finished."
        # The ISO lands at /build/zohara-profile/out/ (mkarchiso -o flag).
        # That is on the host as zohara-profile/out/, NOT /build/out/.
        ls -lh /build/zohara-profile/out/*.iso 2>/dev/null || echo "WARNING: no ISO produced"
        # Also copy to /build/out/ (the separate mount) for the validation step.
        cp /build/zohara-profile/out/*.iso /build/out/zohara-os-x86_64.iso 2>/dev/null || true
      ' 2>&1 | tee "$LOG_FILE"
    exit_code=${PIPESTATUS[0]}
    set -e
    echo ""
    if [ "$exit_code" -eq 0 ]; then
        echo "[+] Build finished successfully (exit 0)."
    else
        echo "[!] Build failed with exit code $exit_code."
        exit $exit_code
    fi
    exit 0
fi

# Local mode: detached launch.
echo "[+] Launching build in container '${CONTAINER_NAME}' (detached)..."
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

    if [[ "${FORCE_REBUILD:-0}" == "1" ]]; then
        echo "[+] FORCE_REBUILD=1 -- rebuilding Rust binaries from source"
        # zohara-settings now lives in https://github.com/Zohaib8090/zohara-settings
        if [ ! -d /build/zohara-settings-rs ]; then
            echo "[+] Cloning zohara-settings standalone repo..."
            git clone --depth 1 https://github.com/Zohaib8090/zohara-settings.git /build/zohara-settings-rs
        fi
        (cd /build/zohara-settings-rs && cargo build --release)
        install -Dm755 /build/zohara-settings-rs/target/release/zohara-settings /opt/build/zohara-settings
        install -Dm644 /build/zohara-settings-rs/data/zohara-settings.desktop /opt/build/zohara-settings.desktop

        (cd /build/zohara-store-rs && cargo build --release)
        install -Dm755 /build/zohara-store-rs/target/release/zohara-store /opt/build/zohara-store
        install -Dm644 /build/zohara-store-rs/data/zohara-store.desktop /opt/build/zohara-store.desktop
    else
        echo "[+] Using prebuilt /opt/build/zohara-settings (set FORCE_REBUILD=1 to rebuild)"
        echo "[+] Using prebuilt /opt/build/zohara-store (set FORCE_REBUILD=1 to rebuild)"
    fi

    echo "[+] Running ISO build (mkarchiso)..."
    bash /build/zohara-profile/build-iso.sh

    echo "[+] ISO build finished."
    # The ISO lands at /build/zohara-profile/out/ (mkarchiso -o flag).
    # That is on the host as zohara-profile/out/, NOT /build/out/.
    ls -lh /build/zohara-profile/out/*.iso 2>/dev/null || echo "WARNING: no ISO produced"
  ' > "$LOG_FILE" 2>&1

echo ""
echo "[+] Build launched in background. To monitor progress:"
echo "    docker logs -f ${CONTAINER_NAME}"
echo "    tail -f $LOG_FILE"
