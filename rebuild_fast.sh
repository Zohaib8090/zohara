#!/usr/bin/env bash
# rebuild_fast.sh -- fast (~5 min) local rebuild of the Zohara OS ISO.
#
# Use this whenever you've changed Rust code or any non-package files.
# The pacstrap step (5 hours on first run) is skipped by reusing work/ if
# the package list hasn't changed.
#
# First run: ~5 hours (full pacstrap)
# Every subsequent run: ~5 minutes (just squashfs + ISO + update bundle)
#
# Set FORCE_FULL=1 to force a complete rebuild from scratch.

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="zohara-builder:latest"

# Optional: set FORCE_FULL=1 in the env to force a complete rebuild.
ENV_FLAGS=()
if [[ "${FORCE_FULL:-0}" == "1" ]]; then
    ENV_FLAGS=(-e "FORCE_FULL=1")
    echo "[i] FORCE_FULL=1 set -- this run will wipe work/ and rebuild from scratch."
fi

docker run --rm --privileged \
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

    # 1. Build the Rust binaries from local source. This is the only step
    #    that has to re-run on every iteration -- it picks up code changes.
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

    # 2. Run the build-iso.sh from this checkout. It will reuse work/ if the
    #    package list is unchanged, so pacstrap (~5 hrs) is skipped.
    echo "[+] Running ISO build (mkarchiso, incremental)..."
    # Invoke with explicit 'bash' so it works even if the bind mount drops
    # the execute bit (GitHub Actions' overlay2 sometimes does this on
    # /build, and a missing +x would otherwise fail with EACCES).
    bash /build/zohara-profile/build-iso.sh

    echo "[+] ISO build finished."
    ls -lh /build/out/*.iso 2>/dev/null || echo "WARNING: no ISO produced"
  '
