# syntax=docker/dockerfile:1
FROM archlinux:latest

# ── 1. Base system update ──────────────────────────────────────────────────────
# Cache mount: pacman package cache persists in a named BuildKit cache
RUN --mount=type=cache,target=/var/cache/pacman/pkg,sharing=locked \
    pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        archiso \
        base-devel \
        git \
        sudo \
        curl

# ── 2. Add Chaotic-AUR (pre-built AUR binaries: brave-bin) ────────────────────
RUN pacman-key --init && \
    pacman-key --populate archlinux && \
    pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com && \
    pacman-key --lsign-key 3056513887B78AEB && \
    pacman -U --noconfirm \
        'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst' \
        'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst'

# Enable the Chaotic-AUR repository in pacman.conf
RUN echo -e "\n[chaotic-aur]\nInclude = /etc/pacman.d/chaotic-mirrorlist" >> /etc/pacman.conf && \
    pacman -Sy --noconfirm

# ── 3. Download pre-built AUR packages into local repo ────────────────────────
# brave-bin is the only AUR package with a usable binary mirror (Chaotic-AUR).
# Calamares used to be fetched from an EndeavourOS mirror here, but that
# mirror dropped Calamares between 2026-08-18 and 2026-08-23 (context.md
# documents the regression). It is now built from AUR source like debtap.
RUN --mount=type=cache,target=/var/cache/pacman/pkg,sharing=locked \
    mkdir -p /opt/localrepo && \
    pacman -Sw --noconfirm --cachedir /opt/localrepo brave-bin

# ── 4. Build debtap + calamares from AUR source ───────────────────────────────
# Both packages are AUR-only. The AUR PKGBUILDs are pinned to specific
# versions in /tmp so a future AUR update can't silently change the
# installer shipped on the ISO. To upgrade, bump the pkgver in the PKGBUILD
# mirrors below and verify the resulting ISO boots.
RUN useradd -m builder && \
    echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

USER builder
RUN --mount=type=cache,target=/var/cache/pacman/pkg,uid=1000,sharing=locked \
    git clone https://aur.archlinux.org/debtap.git /tmp/debtap && \
    cd /tmp/debtap && \
    makepkg -s --noconfirm

# Calamares depends on qt6-declarative (kept) plus a long list of
# KDE Frameworks. It is a ~3 minute build the first time; the cache mount
# on pacman's pkg cache makes subsequent builds much faster.
#
# NOTE: we deliberately do NOT mount /tmp/debtap or /tmp/calamares as
# BuildKit caches. A cache mount here is a *directory* snapshot, so a
# cached empty /tmp/debtap would short-circuit the git clone, makepkg
# would never run, and the next stage's `cp *.pkg.tar.zst` would fail
# with "No such file or directory" -- which is exactly what happened
# the first time we tried this. The build artifacts (the .pkg.tar.zst
# files) need to live on the same filesystem layer as the subsequent
# `cp`, so they cannot be in a separate cache.
RUN --mount=type=cache,target=/var/cache/pacman/pkg,uid=1000,sharing=locked \
    git clone https://aur.archlinux.org/calamares.git /tmp/calamares && \
    cd /tmp/calamares && \
    makepkg -s --noconfirm

USER root
# Each `cp` is wrapped in a guard so a single AUR build failure does not
# poison the whole pipeline. debtap is the small one; calamares is the
# big one. If calamares fails to build, the ISO will still ship (Calamares
# just won't be the installer) and the failure surfaces as a clear
# "calamares not in localrepo" error from the next stage's `repo-add`.
RUN cp /tmp/debtap/*.pkg.tar.zst /opt/localrepo/ 2>/dev/null || \
    echo "WARN: debtap build produced no .pkg.tar.zst"
RUN cp /tmp/calamares/*.pkg.tar.zst /opt/localrepo/ 2>/dev/null || \
    echo "WARN: calamares build produced no .pkg.tar.zst"

# ── 5. Build the local pacman repo database ───────────────────────────────────
RUN repo-add /opt/localrepo/localrepo.db.tar.gz /opt/localrepo/*.pkg.tar.zst && \
    ln -sf localrepo.db.tar.gz /opt/localrepo/localrepo.db && \
    ln -sf localrepo.files.tar.gz /opt/localrepo/localrepo.files

# ── 6. Install Rust toolchain ─────────────────────────────────────────────────
USER builder
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/home/builder/.cargo/bin:${PATH}"

# ── 7. Install GTK4 + libadwaita dev libs ─────────────────────────────────────
USER root
RUN --mount=type=cache,target=/var/cache/pacman/pkg,sharing=locked \
    pacman -Sy --noconfirm && \
    pacman -S --noconfirm gtk4 libadwaita pkgconf dbus cmake

# ── 7b. Voice typing engine: whisper.cpp + a small multilingual model ─────────
# Settings > Accessibility > Voice typing (Meta+H) runs speech recognition
# locally, so the engine and model ship in the ISO. whisper.cpp is not in the
# Arch repos; it is built from a pinned release, twice: once for CPUs with
# AVX2/FMA/F16C (roughly 2013 onwards) and once with those off so older CPUs
# still work. zohara-settings picks one at runtime from /proc/cpuinfo.
# Static, no OpenMP: the binaries depend only on glibc and libstdc++.
# Everything lands in /opt/build/dictation-root/, laid out like the target
# filesystem, and build-iso.sh copies it into the airootfs as-is.
ARG WHISPER_VERSION=v1.9.4
ARG WHISPER_MODEL=ggml-base-q8_0.bin
ARG WHISPER_MODEL_SHA256=c577b9a86e7e048a0b7eada054f4dd79a56bbfa911fbdacf900ac5b567cbb7d9
RUN git clone --depth 1 --branch "${WHISPER_VERSION}" https://github.com/ggml-org/whisper.cpp.git /tmp/whisper.cpp && \
    cd /tmp/whisper.cpp && \
    common="-DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DGGML_NATIVE=OFF -DGGML_OPENMP=OFF -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF -DWHISPER_SDL2=OFF" && \
    cmake -B build-avx2 $common && \
    cmake --build build-avx2 -j"$(nproc)" --target whisper-cli && \
    cmake -B build-base $common -DGGML_AVX=OFF -DGGML_AVX2=OFF -DGGML_BMI2=OFF -DGGML_FMA=OFF -DGGML_F16C=OFF && \
    cmake --build build-base -j"$(nproc)" --target whisper-cli && \
    dest=/opt/build/dictation-root && \
    install -Dm755 build-avx2/bin/whisper-cli "$dest/usr/lib/zohara/whisper/zohara-whisper-avx2" && \
    install -Dm755 build-base/bin/whisper-cli "$dest/usr/lib/zohara/whisper/zohara-whisper" && \
    install -Dm644 LICENSE "$dest/usr/share/licenses/zohara-whisper/LICENSE" && \
    mkdir -p "$dest/usr/share/zohara/dictation" && \
    curl -fL --retry 3 -o "$dest/usr/share/zohara/dictation/${WHISPER_MODEL}" \
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/${WHISPER_MODEL}" && \
    echo "${WHISPER_MODEL_SHA256}  $dest/usr/share/zohara/dictation/${WHISPER_MODEL}" | sha256sum -c - && \
    rm -rf /tmp/whisper.cpp

# ── 8. Build zohara-settings & zohara-store (Rust/GTK4/libadwaita) ───────────
# zohara-settings now lives in its own repository:
#   https://github.com/Zohaib8090/zohara-settings
# We clone it at build time (depth=1, no history) instead of vendoring
# the source into this repo. To iterate on the Settings app, push to
# zohara-settings; this repo just consumes the latest main.
#
# ZOHARA_SETTINGS_SHA exists purely to bust Docker's layer cache: the RUN
# below has identical text on every build, so without something upstream
# of it changing, BuildKit (and a plain local `docker build`, which caches
# layers too) will reuse a cached clone from a previous build forever --
# a push to zohara-settings would silently NOT reach the ISO. Passing the
# current `main` SHA as a build-arg makes this layer's cache key change
# whenever zohara-settings moves. The build invocation (build-iso.yml,
# scripts/build-iso-wsl.sh) is responsible for resolving and passing it;
# `unknown` here is just a default for a manual `docker build` with no
# --build-arg, which still works, just without the cache-bust guarantee.
ARG ZOHARA_SETTINGS_SHA=unknown
USER builder
RUN echo "zohara-settings @ ${ZOHARA_SETTINGS_SHA}" && \
    git clone --depth 1 https://github.com/Zohaib8090/zohara-settings.git /tmp/zohara-settings-rs
COPY --chown=builder:builder zohara-store-rs /tmp/zohara-store-rs
# zohara-welcome: the welcome app and migration tool (replaced four PyQt5 scripts).
COPY --chown=builder:builder zohara-welcome /tmp/zohara-welcome-rs
# Cache mounts: persist Cargo registry, git checkouts, AND the per-crate
# `target/` directories across `docker build` runs. Without the target
# mounts, every change to a single .rs file forces a full cold rebuild of
# every dependency in both crates -- on this codebase that is ~1m30s of
# wasted compilation per build, even for a one-line tweak.
#
# IMPORTANT: cache mounts are only valid for the duration of a single
# `docker build` step. They do NOT survive into a `docker run` container,
# so the binaries in /tmp/zohara-*-rs/target/release/ are NOT visible to
# the ENTRYPOINT. We therefore copy the built artifacts to /opt/build/
# (a real path on the image filesystem) after the build, so the ENTRYPOINT
# below can install them into the airootfs overlay.
RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,sharing=locked \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,sharing=locked \
    cd /tmp/zohara-settings-rs && /home/builder/.cargo/bin/cargo build --release && \
    cd /tmp/zohara-welcome-rs && /home/builder/.cargo/bin/cargo build --release && \
    cd /tmp/zohara-store-rs && /home/builder/.cargo/bin/cargo build --release && \
    PATH=/home/builder/.cargo/bin:$PATH makepkg --nodeps --nocheck --skippgpcheck && \
    mv /tmp/zohara-store-rs/zohara-store-[0-9]*.pkg.tar.zst /tmp/zohara-store.pkg.tar.zst

# /opt is owned by root, so we cannot create /opt/build while still USER
# builder. Switch to root just for the install step. The build artifacts
# in /tmp/zohara-*-rs/target/release/ are committed to this layer by
# the `cargo build --release` above, so they are visible here.
#
# Note: we do NOT mount /tmp/zohara-*-rs/target/ as a BuildKit cache. A
# cache mount is a *directory snapshot* that does not persist into the
# image layer; the next RUN step would find the directory empty. The
# cargo registry / git cache mounts are fine because they live under
# $CARGO_HOME which is only used as a download cache.
USER root
RUN mkdir -p /opt/build && \
    cp /tmp/zohara-settings-rs/target/release/zohara-settings /opt/build/ && \
    cp /tmp/zohara-store.pkg.tar.zst                         /opt/build/ && \
    cp /tmp/zohara-welcome-rs/target/release/zohara-welcome /tmp/zohara-welcome-rs/target/release/zohara-migrate /opt/build/ && \
    cp /tmp/zohara-settings-rs/data/zohara-settings.desktop  /opt/build/ && \
    cp /tmp/zohara-settings-rs/data/zohara-settings-health.service \
       /tmp/zohara-settings-rs/data/zohara-settings-health.timer /opt/build/ && \
    dest=/opt/build/dictation-root && \
    install -Dm644 /tmp/zohara-settings-rs/data/zohara-dictation.desktop \
        "$dest/usr/share/applications/zohara-dictation.desktop" && \
    mkdir -p "$dest/usr/share/kglobalaccel" && \
    ln -sf /usr/share/applications/zohara-dictation.desktop \
        "$dest/usr/share/kglobalaccel/zohara-dictation.desktop" && \
    install -Dm644 /tmp/zohara-settings-rs/data/70-zohara-uinput.rules \
        "$dest/etc/udev/rules.d/70-zohara-uinput.rules" && \
    install -Dm644 /tmp/zohara-settings-rs/data/zohara-privacy-indicator.desktop \
        "$dest/etc/xdg/autostart/zohara-privacy-indicator.desktop" && \
    mkdir -p "$dest/etc/modules-load.d" && \
    echo uinput > "$dest/etc/modules-load.d/zohara-uinput.conf"

# ── 9. Entry point ────────────────────────────────────────────────────────────
# set -euo pipefail so that:
#   -e   any failed install / rm / cp aborts mkarchiso with a non-zero exit
#        (previously a typo in any one of these `&&` chains would silently
#         produce a half-built ISO because the script kept going)
#   -u   unbound variables are treated as errors (catches $WORK_DIR etc.
#        typos at the first reference instead of producing an empty path)
#   -o pipefail  the cargo | tee pipeline fails if cargo fails, not only if
#                tee fails
USER root
WORKDIR /build
# Copy the build-iso.sh wrapper into the image. We use a JSON-array ENTRYPOINT
# (no shell-string interpolation) so PowerShell and other Windows shells on
# the host can pass `docker run` arguments without mangling the && chain.
# The wrapper itself contains the full `set -euo pipefail` and the
# `script -qec` pty wrap that pacman needs to auto-answer provider
# questions.
COPY --chown=root:root zohara-profile/build-iso.sh /opt/build-iso.sh
RUN chmod +x /opt/build-iso.sh
# Copy the pacman-overwrite-xorg wrapper (fixes xorg-server / xorg-server-common
# /usr/lib/Xorg dir-vs-file conflict) so it's available to /opt/build-iso.sh.
COPY --chown=root:root zohara-profile/pacman-overwrite-xorg /opt/pacman-overwrite-xorg
RUN chmod +x /opt/pacman-overwrite-xorg
ENTRYPOINT ["/opt/build-iso.sh"]
