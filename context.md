# Zohara OS - Comprehensive Context & State Document

This document contains the entire state, context, secrets, and configuration details for the Zohara OS project. It is intended to allow any future AI or developer to instantly resume work with 100% context.

## 1. Project Overview
- **Name:** Zohara OS
- **Base:** Arch Linux
- **Architecture:** x86_64
- **Goal:** Create a polished, user-friendly, Arch-based Linux distribution with a seamless migration path from Debian/Ubuntu-based systems (like Zorin OS).

## 2. Secrets & Credentials
- **Host OS `sudo` password:** `admin144`
- *(Used extensively for Docker commands, mounting, and clearing build caches).*

## 3. Core Components & Custom Scripts

### Zohara Welcome App (`zohara-profile/airootfs/usr/local/bin/zohara-welcome`)
- **Tech Stack:** Python 3, PyQt5
- **Behavior:** Auto-starts on the Live USB (via `etc/xdg/autostart/zohara-welcome.desktop`).
- **Options:** 
  1. **Install Zohara OS:** Launches `calamares` installer.
  2. **Migrate from another OS:** Launches `zohara-migrate`.
  3. **Try Zohara OS:** Closes the app.
- **Key Fixes Applied:** 
  - On the Live USB, it runs directly without `pkexec` because the user is already `root` (UID 0) and the polkit agent isn't running, which previously caused a silent crash.
  - Logs all activity to `/tmp/zohara-welcome.log`.

### Zohara Migration Tool (`zohara-profile/airootfs/usr/local/bin/zohara-migrate`)
- **Tech Stack:** Python 3, PyQt5
- **Purpose:** Safely migrate data from a Debian/Ubuntu partition to Zohara OS on heavily constrained storage.
- **Logic (Incremental Move-Verify-Delete):**
  1. Identifies the old partition and mounts it.
  2. Scans `/var/lib/dpkg/status` to find installed `apt` packages.
  3. Translates Debian packages to Arch packages using a hardcoded `DEB_TO_ARCH` dictionary.
  4. Iterates through `/home` **file-by-file**: 
     - Copies file to destination.
     - Computes SHA-256 hash of source and destination.
     - **If match:** Deletes the source file immediately to free up space.
     - **If mismatch:** Deletes the bad copy, keeps the source, logs the error.
  5. Installs the mapped Arch packages via `pacman -S`.
  6. Reports any unmapped packages to the user so they can manually install them later.
  7. Deletes the old OS system directories (`/bin`, `/lib`, `/usr`, etc.) from the source partition.

### Brave Browser Wrapper (`zohara-profile/airootfs/usr/local/bin/brave-origin`)
- **Issue:** Chromium-based browsers refuse to run as `root` (which is the default on the Arch Live USB).
- **Fix:** A bash wrapper that detects if `UID == 0`. If true, it appends the `--no-sandbox` flag to allow Brave to launch. If false (normal user), it runs normally.

## 4. Boot Menu Configurations
- **Frameworks:** `systemd-boot` (UEFI) and `syslinux` (Legacy BIOS).
- **Entries:**
  1. **Default:** Standard boot.
  2. **Open Source:** `modprobe.blacklist=nvidia,nvidia_drm,nouveau`
  3. **Nvidia Drivers:** `nvidia-drm.modeset=1`
  4. **Safe Mode:** `nomodeset single`
- **Safety:** All entries include `rd.systemd.gpt_auto=0` to prevent the live USB from automatically mounting internal host drives.

## 5. Pre-configured Theming & Appearance
- **Desktop Environment:** KDE Plasma 6 (`plasma-desktop`).
- **Display Server:** Supports both Wayland and X11 (added `kwin-x11`, `xorg-server`, `xorg-xwayland`).
- **Global Settings (`etc/skel/.config/kdeglobals`):** Defaults to `BreezeDark`, `Fluent-dark` icons, and `kvantum-dark` widget style.
- **Kvantum (`etc/xdg/kvantum/kvantum.kvconfig`):** Hardcoded to use `MateriaDark` to skip annoying first-time setup prompts.
- **Terminal:** Uses `fastfetch` via `.bashrc` for a custom Zohara OS system info splash.

## 6. Build Environment
- **Tool:** `mkarchiso` running inside a privileged Docker container.
- **Container Name:** `zohara-build`
- **Image:** `zohara-builder`
- **Paths Mounted:**
  - Code: `/home/zohaib/Documents/my/zohara` -> `/build`
  - Pacman Cache: `/home/zohaib/Documents/my/zohara/pkg-cache` -> `/var/cache/pacman/pkg`
- **Command to Clean Build:**
  ```bash
  sudo rm -rf out/ work/ && sudo docker rm -f zohara-build && sudo docker run --name zohara-build --privileged -v $(pwd):/build -v $(pwd)/pkg-cache:/var/cache/pacman/pkg zohara-builder
  ```

## 7. Package List Highlights (`packages.x86_64`)
- **Audio Fix:** Added `wireplumber` (replaces old pipewire-media-session) to fix the "Sound Service Lost" issue.
- **Nvidia Drivers:** Added `nvidia-dkms`, `nvidia-settings`, and `linux-zen-headers` to ensure the proprietary Nvidia boot option works correctly on the user's RTX 3050 Ti.
- **X11 Support:** Added `kwin-x11` (Plasma 6 dropped `plasma-workspace-x11`).

## 8. Current Known Status
- A 100% clean build was triggered to ensure the Nvidia drivers and X11 support are correctly baked into the ISO.
- **Git Push Failure:** The user recently tried to run `git push`, which failed with an `HTTP 500` / `unexpected disconnect` error. This is because the `out/` folder (containing a 3.0GB ISO file) and/or `pkg-cache/` are likely being tracked by git, exceeding GitHub's file size limits. A `.gitignore` needs to be updated to ignore `out/`, `work/`, and `pkg-cache/`.
