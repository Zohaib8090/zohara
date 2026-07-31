FROM archlinux:latest
RUN pacman -Syyu --noconfirm && pacman -Sy --noconfirm archiso git base-devel sudo

# Create builder user and bypass PAM
RUN useradd -m builder && \
    echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers && \
    printf "auth sufficient pam_permit.so\naccount sufficient pam_permit.so\npassword sufficient pam_permit.so\nsession sufficient pam_permit.so\n" > /etc/pam.d/system-auth && \
    sed -i 's/#MAKEFLAGS="-j2"/MAKEFLAGS="-j$(nproc)"/' /etc/makepkg.conf

# Pre-compile AUR packages into /opt/localrepo
RUN mkdir -p /opt/localrepo && chown builder:builder /opt/localrepo && \
    sudo -u builder bash -c 'cd /tmp && git clone https://aur.archlinux.org/debtap.git && cd debtap && makepkg -s --noconfirm --nocheck && cp *.pkg.tar.zst /opt/localrepo/' && \
    sudo -u builder bash -c 'cd /tmp && git clone https://aur.archlinux.org/calamares.git && cd calamares && makepkg -s --noconfirm --nocheck && cp *.pkg.tar.zst /opt/localrepo/' && \
    sudo -u builder bash -c 'cd /tmp && git clone https://aur.archlinux.org/brave-origin-bin.git && cd brave-origin-bin && makepkg -s --noconfirm --nocheck && cp *.pkg.tar.zst /opt/localrepo/' && \
    cd /opt/localrepo && repo-add localrepo.db.tar.gz *.pkg.tar.zst

WORKDIR /build
