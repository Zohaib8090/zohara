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
    pacman -S --noconfirm gtk4 libadwaita pkgconf dbus

# ── 8. Build zohara-settings & zohara-store (Rust/GTK4/libadwaita) ───────────
# zohara-settings now lives in its own repository:
#   https://github.com/Zohaib8090/zohara-settings
# We clone it at build time (depth=1, no history) instead of vendoring
# the source into this repo. To iterate on the Settings app, push to
# zohara-settings; this repo just consumes the latest main.
USER builder
RUN git clone --depth 1 https://github.com/Zohaib8090/zohara-settings.git /tmp/zohara-settings-rs
COPY --chown=builder:builder zohara-store-rs /tmp/zohara-store-rs
# Cache mounts: persist Cargo registry, git checkouts, AND the per-crate
# `target/` directories across `docker build` runs. Without the target
# mounts, every change to a single .rs file forces a full cold rebuild of
# every dependency in both crates -- on this codebase that is ~1m30s of
# wasted compilation per build, even for a one-line tweak.
#
# Only the registry/git download caches are mounted; target/ stays on the
# image layer so the makepkg step below can package the binaries.
RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,sharing=locked \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,sharing=locked \
    cd /tmp/zohara-settings-rs && /home/builder/.cargo/bin/cargo build --release && \
    cd /tmp/zohara-store-rs && /home/builder/.cargo/bin/cargo build --release

# Package both apps as real pacman packages and add them to the local repo,
# so pacstrap installs them (packages.x86_64 lists zohara-settings and
# zohara-store). They used to be copied into the airootfs overlay as loose
# files, which made every later OTA `pacman -Syu` of those packages fail with
# "exists in filesystem" -- pacman will not overwrite files no package owns.
#
# Both PKGBUILDs package the binary cargo just built in the same directory.
# --nodeps because the image uses rustup, not the pacman `rust` package.
#
# Note: we do NOT mount /tmp/zohara-*-rs/target/ as a BuildKit cache. A
# cache mount is a *directory snapshot* that does not persist into the
# image layer; the next RUN step would find the directory empty.
# The settings PKGBUILD's pkgver() reads _PKGVER (makepkg runs pkgver() inside
# $srcdir, where Cargo.toml is not reachable by a relative path).
RUN cd /tmp/zohara-settings-rs && \
    _PKGVER="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)" \
        makepkg --nodeps --nocheck --noconfirm && \
    cd /tmp/zohara-store-rs    && makepkg --nodeps --nocheck --noconfirm

USER root
RUN cp /tmp/zohara-settings-rs/zohara-settings-*.pkg.tar.zst \
       /tmp/zohara-store-rs/zohara-store-*.pkg.tar.zst /opt/localrepo/ && \
    repo-add /opt/localrepo/localrepo.db.tar.gz \
       /opt/localrepo/zohara-settings-*.pkg.tar.zst \
       /opt/localrepo/zohara-store-*.pkg.tar.zst

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
