# Rebuilds the "zs-img" developer container (cargo test / app screenshots for Settings and the Store).
# RECONSTRUCTED on 2026-10-08 from the notes in docs/HANDOFF-2026-10-06.md ("Arch + rust, gtk4, libadwaita, xvfb, imagemagick,
# xdotool, base-devel"). The original image was made by committing a container and was backed up with `docker save`
# (see docs/MOVING-TO-ZOHARA.md). This file has NOT been built to confirm it matches the original: if a build fails for a
# missing header, add the package it names.
#   docker build -t zs-img -f scripts/dev/zs-img.Dockerfile .
FROM archlinux:latest
RUN pacman -Syu --noconfirm --needed rust gtk4 libadwaita pkgconf base-devel git cmake dbus \
        xorg-server-xvfb imagemagick xdotool kconfig jq curl libinput && pacman -Scc --noconfirm
WORKDIR /w
