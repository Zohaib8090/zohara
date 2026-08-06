# Zohara OS - Project Context

This file serves as a summary of the current state of the Zohara OS project to provide context for new AI sessions.

## 1. Project Overview
Zohara OS is a custom, Arch-based Linux distribution built using `mkarchiso`. The project includes custom scripts, UI tools built in PyQt5, and a customized KDE Plasma 6 desktop environment.

## 2. Recent Major Accomplishments
- **Zohara Settings App:** We completely replaced the default KDE System Settings with a unified `zohara-settings` app (written in PyQt5). It features sections for:
  - **Updates:** Asynchronous multi-tier updates (Zohara OTA OS updates, regular system packages, Linux-zen kernel, and hardware drivers).
  - **Appearance:** 1-click layout switching between Win11, Win10, macOS, and KDE Native layouts.
  - **Users:** Inline user creation, deletion, and admin rights management.
  - **About:** Live system information.
- **Custom Icons & Desktop Files:** Created a custom Samsung One UI style SVG icon for `zohara-settings` and integrated it into the system.
- **Plasma Taskbar Pins:** We modified `customize_airootfs.sh` to dynamically strip out KDE Discover and KDE System Settings from the default taskbar pins and replace them with `zohara-store.desktop` and `zohara-settings.desktop`.
- **Executable Permissions Fix:** Git sometimes strips execution bits from scripts. We added a safety net in `customize_airootfs.sh` (`chmod +x /usr/local/bin/zohara-* 2>/dev/null`) to ensure all custom Zohara binaries are always executable in the final ISO.

## 3. Important File Locations
- **Settings App:** `zohara-profile/airootfs/usr/local/bin/zohara-settings`
- **Store App:** `zohara-profile/airootfs/usr/local/bin/zohara-store`
- **Customization Script:** `zohara-profile/airootfs/root/customize_airootfs.sh`
- **ISO Build Pipeline:** `.github/workflows/build-iso.yml`
- **OTA Update Pipeline:** `.github/workflows/build-update.yml`

## 4. Current State
- The ISO is currently being built locally using a dockerized `zohara-builder`.
- The build incorporates the fixes for the `zohara-settings` app crash, the new SVG icon, the executable permissions fix, and the taskbar pin replacements.

## 5. Next Steps for New Session
1. **Verify the ISO:** Boot the newly built ISO locally to verify that:
   - `zohara-settings` launches correctly without crashing.
   - The taskbar pins display Zohara Store and Zohara Settings.
   - The custom SVG icon is rendering.
2. **OTA Testing:** Test the OTA update flow via the Zohara Settings > Updates panel to ensure the `zohara-system` metapackage updates pull correctly from GitHub.
3. Continue polishing the UI or adding missing functionality as requested by the user.
