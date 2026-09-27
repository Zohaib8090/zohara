# Zohara OS

A Linux distribution built to feel like home for people coming from Windows.

## What is Zohara?

Zohara is an Arch-based Linux distribution that trades Linux's usual friction for a desktop you'll already know how to use. It ships the KDE Plasma desktop with a Windows-flavored visual language on top, the Zen kernel for desktop and gaming performance, and working NVIDIA support out of the box — so the first boot looks and feels familiar, and the second boot is fast.

## Highlights

- **A desktop that looks and behaves like Windows.** The Zohara Settings app is a custom GTK4 + libadwaita application built to mirror the Windows 11 Settings layout — left rail navigation, card groups, and a Windows 11 visual theme throughout.
- **Settings that change real system state.** Every page is backed by Plasma 6 (Wayland), NetworkManager, BlueZ, CUPS and friends: display layout, sound with left/right speaker test, Bluetooth pairing, network with a built-in speed test, storage, accounts, default apps, gaming (Game Mode, MangoHud), printers, offline maps (Organic Maps) and websites as apps.
- **Voice typing, entirely offline.** Press Meta+H, speak, and the text is typed where your cursor is. Speech recognition runs locally with whisper.cpp and a multilingual model shipped in the ISO.
- **Problems reported, not hidden.** Settings keeps a log and crash reports, checks system health in the background (failed services, full disk, pending reboot) and offers one-click fixes on the Troubleshoot page.
- **Linux Zen kernel.** Tuned for desktop and gaming workloads rather than server throughput.
- **Working NVIDIA out of the box.** The ISO ships `nvidia-open-dkms`, so supported NVIDIA GPUs work on first boot with no manual driver installation.
- **OTA updates for Zohara software.** Settings, the Zohara Store, and future first-party apps update through a pacman-style package repository with stable, beta, and alpha channels — no waiting for the next ISO.
- **A real software store and a real first-run experience.** `zohara-store` and `zohara-welcome` (autostarted on first login) are first-party, not placeholders.
- **A "link to your phone" companion app in development.** An Android companion pairs with the OS over the local network to share clipboard, send files, and mirror notifications, backed by the `zohara-linkd` Rust daemon on the OS side (in [zohara-link](https://github.com/Zohaib8090/zohara-link)), which the Zohara Link page in Settings talks to.

## Screenshots

Screenshots will land here once the project has a stable public release.

## Download

ISOs are built by GitHub Actions on every change. The image (about 3.7 GB) is larger than GitHub allows for a release file (2 GB), so it isn't on the Releases page (the `v1.0.0` release there is an old August 2026 build). To get the current one:

1. Open [Actions → Build Zohara OS ISO](../../actions/workflows/build-iso.yml) and pick the latest successful run (you need to be signed in to GitHub).
2. Download the `zohara-os-x86_64` artifact (a zip containing `zohara-os-<date>-x86_64.iso`). Each build is kept for 30 days.
3. Flash the ISO to a USB drive (e.g. `dd`, balenaEtcher, Ventoy).
4. Boot the USB. The boot menu lets you **Install Zohara** (launches Calamares) or **Try Zohara** (live session).

Before booting on a PC:

- **Turn off Secure Boot** in the firmware settings. Zohara's bootloaders aren't signed yet.
- If the installer shows no disks, set the storage mode to **AHCI** instead of "RAID On" / Intel RST (VMD).
- The installer doesn't pick "Erase disk" for you. On a computer with Windows, choose **Install alongside** or **Manual partitioning**; Windows then appears in the boot menu.

## System requirements

| | Minimum |
|---|---|
| Architecture | x86_64 |
| RAM | 4 GB |
| Disk | 20 GB free |
| Firmware | UEFI or legacy BIOS |
| GPU | Intel, AMD, or modern NVIDIA (open kernel modules) |

## Building from source

This repository builds the Zohara OS ISO. The build runs in a Docker image based on Arch Linux with the Chaotic-AUR repository added for pre-built AUR packages, and is driven by GitHub Actions on every push to `master` and on every `v*` tag. The exact build order — Docker image build, ISO build via `rebuild_fast.sh`, EFI boot validation, and release publish — lives in [`.github/workflows/build-iso.yml`](.github/workflows/build-iso.yml).

The Settings app and the OTA package repository live in their own repos (see below) and are pulled in as part of the build pipeline; you don't build them by hand to produce an ISO.

To build an ISO locally you can use the same Docker image CI uses. Read the workflow file first — the order of steps matters, and a few steps (notably the EFI boot validation) assume xorriso and `mtools` are available on the host.

## Repositories

Zohara OS is split across these repositories:

- **[Zohaib8090/zohara](https://github.com/Zohaib8090/zohara)** — this repo. The ISO build profile, Docker build environment, GitHub Actions CI, and Zohara's own apps other than Settings (Store, Welcome/migration, voice typing, snapshots).
- **[Zohaib8090/zohara-link](https://github.com/Zohaib8090/zohara-link)** — `zohara-linkd`, the phone-link daemon.
- **[Zohaib8090/zohara-pipeline](https://github.com/Zohaib8090/zohara-pipeline)** — the signed update manifest: which Arch snapshot date Zohara systems update to.
- **[Zohaib8090/zohara-settings](https://github.com/Zohaib8090/zohara-settings)** — the GTK4 + libadwaita Windows-style Settings app.
- **[Zohaib8090/zohara-packages](https://github.com/Zohaib8090/zohara-packages)** — the OTA package repository: every Zohara package (Settings, Store, Welcome, voice typing, snapshots, Link), for x86_64 and for phones (aarch64).

## Companion app

A Zohara Companion app for Android is in active development. It pairs with the OS over the local network (mDNS discovery) and, once installed and paired, provides:

- Shared clipboard between phone and desktop
- File transfer in both directions
- Notification mirroring on the desktop

The OS-side daemon is `zohara-linkd` (in [zohara-link](https://github.com/Zohaib8090/zohara-link)). `zohara-connectd` in this repository is an earlier, unfinished prototype: it parses its options and waits, nothing more, and isn't shipped. The Android app itself is a separate project and not part of this repo. No release date is being promised yet — when it's ready, it will be linked from this repository.

(`VERSION` and `CHANGELOG.md` at the top of this repository belong to the bundled GSD development tooling, which reads and updates them itself; they are not Zohara's version. Zohara's version is the image's build date, shown in `/etc/os-release`.)

## Contributing

Pull requests are welcome. The workflow is straightforward:

1. Open an issue describing the change, or pick one up from the issue tracker.
2. Branch off `master`, make the change, push.
3. CI builds the ISO on every push, so you'll see whether your change broke the build before a review is even requested.

For first-party components (Settings, the OTA repo), open the PR in the appropriate repository — this one is for the ISO, the build pipeline, and OS-side daemons.

## License

Zohara OS is dual-licensed under **MIT** and **GPL-3.0-or-later**. Per-file and per-component license headers are the project default for now; this can be tightened once the project's overall license choice is finalized. Third-party packages shipped on the ISO retain their own upstream licenses.

## Maintainer and community

- Maintainer: [@Zohaib8090](https://github.com/Zohaib8090)
- Project repository: [github.com/Zohaib8090/zohara](https://github.com/Zohaib8090/zohara)
- Issues, feature requests, and discussion: use the GitHub issue tracker on this repository
