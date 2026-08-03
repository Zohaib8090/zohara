FROM archlinux:latest

# ── 1. Base system update ──────────────────────────────────────────────────────
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        archiso \
        base-devel \
        git \
        sudo \
        curl

# ── 2. Add Chaotic-AUR (pre-built AUR binaries: calamares, brave-bin, debtap) ──
# Initialize the pacman keyring first (required inside Docker containers)
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

# ── 3. Download AUR packages from Chaotic-AUR into a local repo ───────────────
RUN mkdir -p /opt/localrepo && \
    cd /opt/localrepo && \
    # Download the packages without installing (we bundle them into the local repo)
    pacman -Sw --noconfirm --cachedir /opt/localrepo \
        brave-bin && \
    # debtap is small enough to build from source with makepkg
    true

# ── 4. Build debtap from AUR source (it's a simple bash script, fast to build) ─
RUN useradd -m builder && \
    echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

USER builder
RUN git clone https://aur.archlinux.org/debtap.git /tmp/debtap && \
    cd /tmp/debtap && \
    makepkg -s --noconfirm

RUN git clone https://aur.archlinux.org/calamares.git /tmp/calamares && \
    cd /tmp/calamares && \
    makepkg -s --noconfirm

USER root
RUN cp /tmp/debtap/*.pkg.tar.zst /opt/localrepo/ && \
    cp /tmp/calamares/*.pkg.tar.zst /opt/localrepo/

# ── 5. Build the local pacman repo database ───────────────────────────────────
RUN repo-add /opt/localrepo/localrepo.db.tar.gz /opt/localrepo/*.pkg.tar.zst && \
    ln -sf localrepo.db.tar.gz /opt/localrepo/localrepo.db && \
    ln -sf localrepo.files.tar.gz /opt/localrepo/localrepo.files

# ── 6. Build entry point ──────────────────────────────────────────────────────
WORKDIR /build
ENTRYPOINT ["bash", "-c", "mkarchiso -v -w ./work -o ./out ./zohara-profile/"]
