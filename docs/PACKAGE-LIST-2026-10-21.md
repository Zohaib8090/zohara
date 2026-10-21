# Zohara OS package list, and what Plasma 6.8 (no X11 session) and the new kernel mean for it (2026-10-21)

Two lists: **what the ISO installs** (`zohara-profile/packages.x86_64`, 165 packages) and **what the owner's laptop has explicitly installed** (`pacman -Qe`, 205 packages, 1393 in total with dependencies). Plus the facts checked on 2026-10-21 about Plasma, X11 and the kernel.

## Facts checked today
| Thing | Our systems | Arch right now (2026-10-21) |
|---|---|---|
| Plasma (`plasma-desktop`, `kwin`) | 6.7.5 | **6.8.0** (extra, since 2026-10-15) |
| `kwin-x11`, `plasma-x11-session` | `kwin-x11` 6.7.5 is in the ISO list and on the laptop | **no longer in the Arch repositories** |
| Kernel `linux-zen` | 7.2.7 (laptop) | 7.2.11; upstream Linux **7.3 was released 2026-10-18** (linux-zen 7.3 will follow) |
| `xorg-server` | 21.1.24 | 21.1.25-2; still required by `sddm` (0.21.0-7) |
| `xorg-xwayland` | 24.1.13 | 24.1.14 (this stays: it runs X11 programs inside the Wayland session) |
| Approved update date (`zohara-pipeline`) | machines follow `2026/09/26` | **6.8 is after that date**, so no machine can get it until the owner moves the date |

KDE announced (2025-11-26) that the Plasma X11 *session* ends with Plasma 6.8; X11 *programs* keep working through Xwayland. Our own default is already Wayland: `/etc/sddm.conf` says `Session=plasma`, and the laptop has no X11 session file.

## What this means for Zohara (in order of importance)
1. **The next ISO build breaks if the Arch date moves past 2026-10-15**: `kwin-x11` (line in `packages.x86_64`) no longer exists, so `pacstrap` stops with "target not found". Remove `kwin-x11` (and the comment block about the X11 session and `xf86-input-libinput`) before moving the date. The current ISOs are safe: they use the 2026/09/26 snapshot.
2. **Installed machines**: `kwin-x11` is installed on the laptop and on every ISO install. When the approved date passes 2026-10-15, the Update must remove `kwin-x11` first or pacman will complain about a package that has left the repository and still depends on the old libraries. Needs a step in Settings' update (`src/sysupdate/`) that removes it, tested in the VM before the date moves.
3. **Plasma 6.8 itself is untested here.** Things that are built on Plasma's inside parts and could break: the cube effect, live wallpaper, dynamic desktops, the `zohara` window decoration and round buttons, night light, `latte-dock-ng` (built for Plasma 6.7, from chaotic-aur), `plasma-keyboard`. Do an ISO build at a date after 2026-10-15 and run the same VM tests before approving the date.
4. **`xorg-server` stays** while SDDM needs it. SDDM can use a Wayland greeter instead (`DisplayServer=wayland`), a later cleanup, not needed now.
5. **Our code does not depend on X11**: Settings uses `kscreen-doctor` first and `xrandr` only as a last fallback (`pages/display.rs`); the "X11 programs" scaling switch is about Xwayland and stays valid. `xf86-video-*` and `xf86-input-libinput` in the ISO list exist only for an X11 session and can go with it.
6. **Kernel 7.3**: nothing of ours is known to break. Watch: `nvidia-open-dkms` (rebuilds for each kernel; needs `linux-zen-headers`, which the ISO has), `waydroid` (needs kernel binder, which zen has), hibernate (7.3 changes the swap subsystem: re-test Hibernate after the update). Not tested; linux-zen 7.3 is not in Arch yet.

## ISO list (165), `zohara-profile/packages.x86_64`
```
alsa-firmware alsa-utils amd-ucode arch-install-scripts ark attica base base-devel bluedevil bluez 
bluez-utils brave-origin-bin breeze breeze-gtk btrfs-progs cabextract ca-certificates calamares 
chaotic-keyring chaotic-mirrorlist cups cups-filters cups-pk-helper curl debtap distrobox dolphin dosfstools 
e2fsprogs edk2-shell efibootmgr espeak-ng exfatprogs f2fs-tools fastfetch flatpak flatseal 
fluent-cursor-theme-git fluent-gtk-theme-git fluent-icon-theme-git fuse2 fwupd gamemode geoclue git gptfdisk 
grub grub-btrfs gtk4 gwenview haruna iio-sensor-proxy inotify-tools intel-media-driver intel-ucode iwd kate 
kdialog knewstuff konsole kscreen kvantum kvantum-theme-materia kwallet kwallet-pam kwin-x11 latte-dock-ng 
libadwaita libinput libinput-tools libnotify libreoffice-fresh linux-firmware linux-zen linux-zen-headers 
mangohud memtest86+ memtest86+-efi mesa mkinitcpio mkinitcpio-archiso mkinitcpio-nfs-utils nano nbd 
networkmanager networkmanager-openvpn nordzy-icon-theme-git noto-fonts noto-fonts-emoji nss-mdns ntfs-3g 
nvidia-open-dkms nvidia-settings okular orca os-prober p7zip pacman-contrib pciutils pipewire pipewire-alsa 
pipewire-jack pipewire-pulse plasma-desktop plasma-integration plasma-keyboard plasma-nm plasma-pa 
plasma-systemmonitor playerctl plymouth podman polkit-kde-agent powerdevil power-profiles-daemon 
qt6-declarative qt6-multimedia qt6-multimedia-ffmpeg qt6-quick3d qt6-virtualkeyboard rsync sddm snap-pac 
snapper sof-firmware spectacle speech-dispatcher squashfs-tools sudo switcheroo-control syslinux 
ttf-liberation udisks2 ufw unzip upower usbutils v4l-utils vulkan-intel vulkan-radeon waydroid wayland 
webkit2gtk-4.1 wget wine wine-gecko wine-mono winetricks wireplumber wl-clipboard xdg-desktop-portal-kde 
xdg-user-dirs xdg-utils xf86-input-libinput xf86-video-amdgpu xf86-video-intel xf86-video-nouveau xorg-server 
xorg-xinit xorg-xwayland yay ydotool zenity zram-generator zsh 
```

## Explicitly installed on the owner's laptop (205)
```
7zip alsa-firmware alsa-utils amd-ucode android-tools antigravity-ide arch-install-scripts ark attica base 
base-devel bluedevil bluez bluez-utils brave-origin-bin breeze breeze-gtk btrfs-progs ca-certificates 
chaotic-keyring chaotic-mirrorlist claude-desktop cmake cups cups-filters cups-pk-helper curl debtap 
distrobox docker docker-buildx dolphin dosfstools e2fsprogs edk2-shell efibootmgr ente-auth espeak-ng 
exfatprogs extra-cmake-modules f2fs-tools fastfetch flatpak flatseal fluent-cursor-theme-git 
fluent-gtk-theme-git fluent-icon-theme-git fwupd gamemode geoclue git github-cli google-chrome gptfdisk grim 
grub grub-btrfs gst-plugin-pipewire gst-plugins-bad gst-plugins-ugly gst-plugin-va gtk4 gwenview haruna 
iio-sensor-proxy inotify-tools intel-media-driver intel-ucode iwd jdk-openjdk jq kate kdeplasma-addons 
kdialog knewstuff konsole kscreen kvantum kvantum-theme-materia kwallet kwalletmanager kwallet-pam kwin-x11 
latte-dock-ng lib32-gamemode lib32-mangohud lib32-mesa lib32-nvidia-utils lib32-vulkan-intel 
lib32-vulkan-radeon libadwaita libinput libinput-tools libisoburn libreoffice-fresh linux-firmware linux-zen 
linux-zen-headers llama.cpp-cuda-aidock-bin lutris mangohud memtest86+ memtest86+-efi mesa minisign 
mkinitcpio mkinitcpio-archiso mkinitcpio-nfs-utils nano nbd netflix networkmanager networkmanager-openvpn 
nodejs nordzy-icon-theme-git noto-fonts noto-fonts-emoji npm nss-mdns ntfs-3g nvidia-open-dkms 
nvidia-settings okular orca os-prober pacman-contrib pciutils pipewire pipewire-alsa pipewire-jack 
pipewire-pulse plasma-desktop plasma-integration plasma-keyboard plasma-nm plasma-pa plasma-systemmonitor 
plymouth podman polkit-kde-agent powerdevil power-profiles-daemon prismlauncher protonup-qt python-pipx 
qemu-full qt6-declarative qt6-virtualkeyboard rsync rustup sddm slurp snap-pac snapper sof-firmware spectacle 
speech-dispatcher steam sudo switcheroo-control syslinux tailscale ttf-liberation ufw unzip upower usbutils 
v4l-utils visual-studio-code-bin vlc vulkan-intel vulkan-radeon waydroid wayland webkit2gtk-4.1 wget wine 
winetricks wireplumber wl-clipboard xdg-desktop-portal-kde xdg-user-dirs xdg-utils xf86-input-libinput 
xf86-video-amdgpu xf86-video-intel xf86-video-nouveau xorg-server xorg-xinit xorg-xwayland yay ydotool zenity 
zohara-keyring zohara-linkd zohara-settings zohara-snapshots zohara-store zohara-terminal 
zohara-terminal-debug zohara-voice zohara-voice-model zohara-welcome zram-generator zsh 
```

## On the ISO list but not explicitly installed on the laptop
```
cabextract calamares fuse2 libnotify p7zip playerctl qt6-multimedia qt6-multimedia-ffmpeg qt6-quick3d 
squashfs-tools udisks2 wine-gecko wine-mono 
```

## Explicitly installed on the laptop but not on the ISO list
```
7zip android-tools antigravity-ide claude-desktop cmake docker docker-buildx ente-auth extra-cmake-modules 
github-cli google-chrome grim gst-plugin-pipewire gst-plugins-bad gst-plugins-ugly gst-plugin-va jdk-openjdk 
jq kdeplasma-addons kwalletmanager lib32-gamemode lib32-mangohud lib32-mesa lib32-nvidia-utils 
lib32-vulkan-intel lib32-vulkan-radeon libisoburn llama.cpp-cuda-aidock-bin lutris minisign netflix nodejs 
npm prismlauncher protonup-qt python-pipx qemu-full rustup slurp steam tailscale visual-studio-code-bin vlc 
zohara-keyring zohara-linkd zohara-settings zohara-snapshots zohara-store zohara-terminal 
zohara-terminal-debug zohara-voice zohara-voice-model zohara-welcome 
```
