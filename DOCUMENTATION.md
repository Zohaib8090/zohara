# Zohara OS - Technical Documentation & Reference Manual

## Table of Contents
1. [Overview & Project Goals](#1-overview--project-goals)
2. [Git Repositories & Remote Ecosystem](#2-git-repositories--remote-ecosystem)
3. [Zohara Settings Architecture (`zohara-settings-rs`)](#3-zohara-settings-architecture-zohara-settings-rs)
4. [Zohara Store & Version Management (`zohara-packages`)](#4-zohara-store--version-management-zohara-packages)
5. [ISO Generation & Docker Build Pipeline](#5-iso-generation--docker-build-pipeline)
6. [Troubleshooting & Common Fixes](#6-troubleshooting--common-fixes)

---

## 1. Overview & Project Goals
Zohara OS is a modernized, Arch Linux-based desktop operating system powered by KDE Plasma 6. It features custom, native GTK4/Libadwaita applications, a dedicated repository network, and an automated ISO compilation pipeline.

---

## 2. Git Repositories & Remote Ecosystem

| Repository | GitHub URL | Description |
| :--- | :--- | :--- |
| **Main OS** | `https://github.com/Zohaib8090/zohara.git` | Base OS profile, build scripts, Dockerfile, PKGBUILDs |
| **Settings** | `https://github.com/Zohaib8090/zohara-settings.git` | Rust GTK4 / Libadwaita standalone settings application |
| **Packages** | `https://github.com/Zohaib8090/zohara-packages.git` | Store catalog index (`apps.json`) with versioning schema |

---

## 3. Zohara Settings Architecture (`zohara-settings-rs`)

### Multi-Threading & Async GLib Architecture
To prevent GLib main-thread deadlocks or panics when calling async backend services, `zohara-settings` uses a global Tokio runtime:

```rust
use std::sync::OnceLock;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn main() {
    gtk::init().unwrap();
    TOKIO_RUNTIME.get_or_init(|| Runtime::new().expect("Failed to initialize Tokio runtime"));
    // ...
}
```

Async backend calls (D-Bus invocations for NetworkManager, BlueZ, Power Profiles, etc.) execute inside `TOKIO_RUNTIME.spawn(...)`, and UI updates are safely returned to the main thread via `glib::idle_add`.

### UI Pages & Functionality:
- **Network**: Manages WiFi AP discovery, signal strength, WPA enterprise/personal connections, and Ethernet interface status via `org.freedesktop.NetworkManager`.
- **Bluetooth**: Scans, pairs, connects, and unpairs Bluetooth peripherals via `org.bluez`.
- **Display**: Resolution, refresh rate, scaling factor, and orientation settings via Wayland/KScreen D-Bus.
- **Sound**: Volume levels, input/output device selection, and default profile routing via PipeWire D-Bus interface.
- **Power**: Toggles system power profiles (Performance, Balanced, Power Saver) via `org.freedesktop.UPower.PowerProfiles`.
- **Appearance**: Controls Global Theme, GTK theme, Plasma theme, Icons, Cursor, and Accent Colors.
- **Users**: Integrates Polkit with `accountsservice` D-Bus interface for user management.
- **Updates**: Manages OTA updates for Zohara OS packages.
- **About**: Hardware summary (CPU, GPU, RAM, Kernel, OS Version).

---

## 4. Zohara Store & Version Management (`zohara-packages`)

`zohara-packages` acts as the package store backend index (`apps.json`).

### Version Control Schema:
```json
{
  "store_name": "Zohara Packages",
  "version": "1.1",
  "featured": ["zohara-settings", "firefox", "vscodium", "lutris"],
  "apps": [
    {
      "id": "zohara-settings",
      "name": "Zohara Settings",
      "publisher": "Zohara OS Team",
      "description": "System settings application for Zohara OS",
      "category": "System",
      "icon_url": "https://raw.githubusercontent.com/Zohaib8090/zohara-settings/main/data/icons/scalable/apps/zohara-settings.svg",
      "type": "pacman",
      "package": "zohara-settings",
      "current_version": "0.1.0",
      "versions": [
        {
          "version": "0.1.0",
          "release_date": "2026-08-18",
          "download_url": "https://github.com/Zohaib8090/zohara-settings/releases/download/v0.1.0/zohara-settings-0.1.0-1-x86_64.pkg.tar.zst",
          "changelog": "Initial release with Network, Bluetooth, Display, Sound, and Power settings"
        }
      ]
    }
  ]
}
```

### Upgrade & Rollback (Downgrade) Protocol:
- **Upgrade**: `sudo pacman -U <download_url_of_newer_version>`
- **Downgrade**: `sudo pacman -U <download_url_of_older_version>`

---

## 5. ISO Generation & Docker Build Pipeline

### Pipeline Architecture:
1. `Dockerfile` pulls `archlinux:latest`.
2. Clones `https://github.com/Zohaib8090/zohara-settings.git`.
3. Builds release binaries with Rust `cargo build --release`.
4. Executes `makepkg` on `packages/zohara-settings/PKGBUILD`.
5. Adds package to `/opt/localrepo/localrepo.db`.
6. Executes `mkarchiso` to generate `out/zohara-os-*.iso`.

### Commands to Run Build:
```bash
# 1. Clean stale work and output directories
sudo rm -rf ~/Documents/my/zohara/work/ ~/Documents/my/zohara/out/

# 2. Build Docker container image
docker build -t zohara-builder ~/Documents/my/zohara

# 3. Execute ISO build inside container
sudo docker run --rm --name zohara-build \
  --privileged \
  -v ~/Documents/my/zohara:/build \
  -v ~/Documents/my/zohara/pkg-cache:/var/cache/pacman/pkg \
  zohara-builder
```

---

## 6. Troubleshooting & Common Fixes

### Issue 1: Pacman Lock Error (`unable to lock database`)
**Cause**: Interrupted previous `mkarchiso` run left a lock file in `work/`.
**Fix**: Run `sudo rm -rf ~/Documents/my/zohara/work/` before running `docker run`.

### Issue 2: Large Docker Context Upload
**Cause**: Missing exclusions in `.dockerignore`.
**Fix**: Ensure `out`, `work`, `pkg-cache`, `.git`, and `**/target` are present in `.dockerignore`.
