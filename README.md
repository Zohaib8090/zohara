# Zohara OS

A Linux distribution built to feel like home for people coming from Windows.

## What is Zohara?

Zohara is an Arch-based Linux distribution that trades Linux's usual friction for a desktop you'll already know how to use. It ships the KDE Plasma desktop with a Windows-flavored visual language on top, the Zen kernel for desktop and gaming performance, and working NVIDIA support out of the box — so the first boot looks and feels familiar, and the second boot is fast.

## Highlights

- **A desktop that looks and behaves like Windows.** The Zohara Settings app is a custom GTK4 + libadwaita application built to mirror the Windows 11 Settings layout — left rail navigation, card groups, and a Windows 11 visual theme throughout.
- **Linux Zen kernel.** Tuned for desktop and gaming workloads rather than server throughput.
- **Working NVIDIA out of the box.** The ISO ships `nvidia-open-dkms`, so supported NVIDIA GPUs work on first boot with no manual driver installation.
- **OTA updates for Zohara software.** Settings, the Zohara Store, and future first-party apps update through a pacman-style package repository with stable, beta, and alpha channels — no waiting for the next ISO.
- **A real software store and a real first-run experience.** `zohara-store` and `zohara-welcome` (autostarted on first login) are first-party, not placeholders.
- **A "link to your phone" companion app in development.** An Android companion pairs with the OS over the local network to share clipboard, send files, and mirror notifications, backed by the `zohara-connectd` Rust daemon on the OS side.

## Screenshots

Screenshots will land here once the project has a stable public release.

## Download

Stable builds are published on the [GitHub Releases page](../../releases) for this repository. The image file is named `zohara-os-VERSION-x86_64.iso`.

To install:

1. Download the latest ISO from the Releases page.
2. Flash it to a USB drive (e.g. `dd`, balenaEtcher, Ventoy).
3. Boot the USB. The boot menu lets you **Install Zohara** (launches Calamares) or **Try Zohara** (live session).

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

Zohara OS is split across three repositories:

- **[Zohaib8090/zohara](https://github.com/Zohaib8090/zohara)** — this repo. The ISO build profile, Docker build environment, GitHub Actions CI, and the `zohara-connectd` companion daemon.
- **[Zohaib8090/zohara-settings](https://github.com/Zohaib8090/zohara-settings)** — the GTK4 + libadwaita Windows-style Settings app.
- **[Zohaib8090/zohara-packages](https://github.com/Zohaib8090/zohara-packages)** — the OTA package repository. Hosts pacman-style Arch packages for `zohara-settings`, `zohara-store`, and future first-party apps, published across stable, beta, and alpha channels.

## Companion app

A Zohara Companion app for Android is in active development. It pairs with the OS over the local network (mDNS discovery) and, once installed and paired, provides:

- Shared clipboard between phone and desktop
- File transfer in both directions
- Notification mirroring on the desktop

The OS-side daemon is `zohara-connectd`, a Rust binary that ships in this repository. The Android app itself is a separate project and not part of this repo. No release date is being promised yet — when it's ready, it will be linked from this repository.

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
