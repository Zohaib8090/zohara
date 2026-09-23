#!/usr/bin/env bash
# Build the Zohara OS ISO using the prebuilt zohara-builder image, but with the
# LIVE sources from this checkout (so local fixes are included). The image's
# default ENTRYPOINT is overridden; we build the Rust binaries ourselves,
# install them where build-iso.sh expects (/opt/build), then run the ISO build
# and the update-bundle generation.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="zohara-builder:latest"

docker run --rm --privileged \
  --entrypoint bash \
  -v "$REPO":/build \
  -v "$REPO/out":/build/out \
  "$IMAGE" -c '
    set -e
    export RUSTUP_HOME=/home/builder/.rustup
    export CARGO_HOME=/home/builder/.cargo
    export PATH=$CARGO_HOME/bin:$PATH
    export HOME=/home/builder

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

    echo "[+] Running ISO build (mkarchiso)..."
    # Run the LIVE build-iso.sh from this checkout (contains the xorg --overwrite
    # wrapper), NOT the stale copy baked into /opt at image-build time.
    /build/zohara-profile/build-iso.sh

    echo "[+] ISO build finished."
    ls -lh /build/out/*.iso 2>/dev/null || echo "WARNING: no ISO produced"
'
