FROM archlinux:latest
RUN pacman -Syyu --noconfirm && pacman -Sy --noconfirm archiso git base-devel sudo

# Create builder user and bypass PAM
RUN useradd -m builder && \
    echo 'builder ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers && \
    printf "auth sufficient pam_permit.so\naccount sufficient pam_permit.so\npassword sufficient pam_permit.so\nsession sufficient pam_permit.so\n" > /etc/pam.d/system-auth && \
    sed -i 's/#MAKEFLAGS="-j2"/MAKEFLAGS="-j$(nproc)"/' /etc/makepkg.conf

# Pre-compile AUR packages (including brave-origin-bin)
RUN mkdir -p /build/localrepo && chown builder:builder /build/localrepo && \
    sudo -u builder bash -c 'cd /tmp && git clone https://aur.archlinux.org/brave-origin-bin.git && cd brave-origin-bin && makepkg -s --noconfirm --nocheck && cp *.pkg.tar.zst /build/localrepo/' && \
    cd /build/localrepo && repo-add localrepo.db.tar.gz *.pkg.tar.zst

WORKDIR /build
