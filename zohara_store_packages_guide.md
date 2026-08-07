# Zohara Store & Packages Management Guide

This document is a comprehensive guide to how **Zohara Store** manages, packages, and automates software installation on Zohara OS. It outlines the store architecture, package resolution strategies for complex software (e.g. DaVinci Resolve, Windows binaries, AppImages), JSON catalog specifications, and AI prompting guidelines.

---

## 1. Architecture Overview

Zohara Store uses a hybrid desktop architecture:
- **Frontend UI:** HTML5/CSS3/JavaScript rendered via `PyWebView` with a modern dark theme.
- **Backend API:** Python (`StoreAPI`) bridging system package managers, flatpak, WINE runtime, and custom scripts.
- **Online Catalog URL:** `https://raw.githubusercontent.com/Zohaib8090/zohara-packages/master/apps.json`
- **Local Fallback Catalog:** `/usr/share/zohara-store/apps.json`
- **User Library State:** `~/.config/zohara-store/library.json`

---

## 2. Package Types & Installation Strategies

Zohara Store supports 5 distinct package installation strategies to ensure users get 1-click installations without encountering terminal errors or missing dependency issues:

### 1. `pacman` (Official Arch & Chaotic-AUR Repositories)
- **Use Case:** Core open-source Linux software available directly in Arch repos or Chaotic-AUR.
- **Execution:** `pkexec pacman -Sy --noconfirm <package>`
- **Verification:** `pacman -Q <package>`

### 2. `flatpak` (Flathub Integration)
- **Use Case:** Desktop apps, proprietary utilities, or sandboxed software (Spotify, Discord, VS Code).
- **Execution:** `flatpak install -y flathub <package>`
- **Verification:** `flatpak info <package>`

### 3. `aur` (Arch User Repository via `yay`)
- **Use Case:** Software built from AUR scripts or pre-built AUR binaries.
- **Execution:** `yay -S --noconfirm <package>`
- **Verification:** `pacman -Q <package>`

### 4. `custom_script` / `appimage` (Complex Apps & Automated Installers)
- **Use Case:** Complex software requiring specific dependencies, proprietary drivers, or custom directory setup (e.g., **DaVinci Resolve**, **AppImages**, **Standalone Tarballs**).
- **Strategy for DaVinci Resolve / Heavy Apps:**
  1. Install hardware/driver prerequisites automatically (e.g., `opencl-driver`, `mesa`, `cuda` for NVIDIA, `rocm` for AMD, `glu`, `lib32` glibc).
  2. Download/Extract the installation bundle silently into `/opt/<app>` or `~/.local/share/<app>`.
  3. Automatically generate a clean `.desktop` file in `/usr/share/applications/` or `~/.local/share/applications/`.
  4. Place high-resolution PNG/SVG icons in `/usr/share/icons/hicolor/scalable/apps/`.

### 5. `windows` (WINE Compatibility Layer)
- **Use Case:** Windows-only executables (`.exe`).
- **Execution:** Creates an isolated WINE prefix (`~/.wine_zohara_<app_id>`) and runs the installer automatically.

---

## 3. Catalog Manifest Schema (`apps.json`)

To add a new application to Zohara Store, append an entry to `apps.json` in the `Zohaib8090/zohara-packages` repository following this schema:

```json
{
  "id": "davinci-resolve",
  "name": "DaVinci Resolve",
  "publisher": "Blackmagic Design",
  "description": "Professional 8K editing, color correction, visual effects and audio post-production solution.",
  "category": "Media",
  "icon_url": "https://raw.githubusercontent.com/Zohaib8090/zohara-packages/master/icons/davinci-resolve.png",
  "type": "custom_script",
  "package": "davinci-resolve",
  "prerequisites": ["opencl-icd-loader", "glu", "mesa", "lib32-glibc"],
  "custom_script": "https://raw.githubusercontent.com/Zohaib8090/zohara-packages/master/scripts/install-davinci.sh"
}
```

---

## 4. How to Prompt AI for Packaging & Store Automation

When asking an AI agent (or setting up automated CI/CD) to add or troubleshoot a complex package for Zohara Store, use the following prompt template:

```text
[TASK: Add Package to Zohara Store]
Package Name: <App Name>
Source Type: <pacman | flatpak | aur | custom_script | windows>
Special Requirements: <e.g., needs GPU OpenCL drivers, custom desktop file, dependencies>

Instructions:
1. Verify if the package exists in Arch repos or Chaotic-AUR.
2. If custom dependencies are required (e.g. DaVinci Resolve needing OpenCL/Mesa/CUDA), write an automated shell installer script that installs prerequisites first.
3. Ensure a valid .desktop file and icon are registered in /usr/share/applications/.
4. Format the entry as a JSON block matching the Zohara Store apps.json schema.
```

---

## 5. Directory & Repository Map

- **Catalog Repository:** `https://github.com/Zohaib8090/zohara-packages`
- **Store Application Script:** `/usr/local/bin/zohara-store`
- **Bundled Offline Catalog:** `/usr/share/zohara-store/apps.json`
- **Custom Desktop Files:** `/usr/share/applications/`
- **App Icons Location:** `/usr/share/icons/hicolor/scalable/apps/`
