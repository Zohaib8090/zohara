# ISO install test, 2026-10-20 (fresh VM, new ISO)

ISO: `zohara-os-2026.10.20-x86_64.iso`, 4,184,604,672 bytes, SHA-256 `210dc4146a98096cf78ffe51cac5cfc46fcd49dad3f0ddf071295370df14250b`, built by CI run `39387162744`
(commit `6b5653f` of `zohara`, with `zohara-settings` main `304d98d`). Downloaded to `~/Documents/zohara-iso-new/`. Not published anywhere (no `v*` tag).
A local build was tried first and failed: the Docker image build could not download `libstemmer` from `archive.archlinux.org` ("Operation too slow", 6 tries); a network problem, not a code problem.

VM: all old VMs were deleted (`~/zohara-vm` was the only one) and a new one made: UEFI, 60 GB NVMe, KVM, 4 CPUs, 4 GB, no window, driven by `vm.py` over QMP
(`run.sh [ISO]`, `hub.py` serves `srv/` and takes posts in `out/`, `wheel.py N` scrolls, `drag.py x1 y1 x2 y2` drags). Test account `tester` (throwaway password in this VM only).

## Install walkthrough (all steps seen on screen)
Boot -> "Welcome to Zohara" (Install / Save a problem report / Migrate / Try) -> Calamares: Welcome -> Location (auto Karachi) -> Keyboard (English US) -> Partitions
(Erase disk: EFI 512 MiB, Zohara OS btrfs 53.55 GiB, swap 5.95 GiB, "Swap (no Hibernate)", encryption off by default) -> Users -> Summary -> "Continue with Installation?" -> progress ->
"All done" in about 3 minutes -> reboot from disk -> sign-in screen -> desktop with the first-run Welcome (Migrate / Manage users / Update Zohara / Open Settings).

## Problems found (not fixed yet)
1. **`systemd-networkd` is enabled and running next to NetworkManager on a fresh install** (the ISO notes said this was fixed; it is not, in this ISO). Same double management that
   made the owner's Wi-Fi laggy. Fix in the profile: mask `systemd-networkd` and its four sockets in `customize_airootfs.sh` (a plain `disable` does not stick: the sockets restart it),
   and have the update turn it off on installed machines (the owner's laptop was fixed by hand with `fix-network.sh`).
2. **No zram on a fresh install** (only the 5.9 GB swap partition). Settings has a zram switch (Power & battery > Swap) but it is off.
3. The Users step pre-fills the computer name with `derp-x8664`; typed text is appended to it. Choose a sane default (for example `zohara`).
4. With region Pakistan and language English, the installer sets numbers and dates to Urdu (`LC_NUMERIC=ur_PK`), so the lock screen shows an Urdu date. Surprising; consider following the language.
5. The Calamares window is taller than an 800 px screen: the Install button is off-screen until the window is maximised (double-click the title bar).
6. `hostname` is not installed (minor).
7. The installer and `os-release` still say "Zohara OS 1.1".

## Fixes made the same day (in the next ISO; to be verified by installing it)
1. **networkd**: cause found: the image did not have it enabled, but Arch's `90-systemd.preset` says `enable systemd-networkd.service`, so a package install or preset run in the new system turned it back on (and four socket units start it on demand).
   Now `etc/systemd/system-preset/00-zohara.preset` disables it and its sockets, and `customize_airootfs.sh` masks the service and the four sockets.
2. **zram on by default**: `zram-generator` added to `packages.x86_64` and `etc/systemd/zram-generator.conf.d/zohara.conf` (the same file Settings' switch writes, so the switch shows on).
3. **computer name**: `etc/calamares/modules/users.conf` (copy of the packaged default) with `hostname.template: "zohara"` instead of the packaged `derp-${cpu}`.
4. **dates and numbers**: new Calamares step `shellprocess@localefix` after `localecfg` removes the `LC_*` lines from `/etc/locale.conf`, so formats follow the language.
Not fixed: the installer window height (5), `hostname` command (6), the "1.1" labels (7). Existing installed machines are not changed by the ISO; the owner's laptop was fixed by hand (`fix-network.sh`).

## Checked and fine
Installed packages are the current alpha (`zohara-settings` / `zohara-store` / `zohara-welcome` `0.1.0.202610200745-1`); `wine 11.18`, `wine-mono`, `wine-gecko`, `winetricks`, `cabextract` present; no failed system or user services;
the right-click menu entry from `zohara-archive.desktop` is installed.

## Features tested with real clicks in the installed system
* **Display** with the fake two-screen `kscreen-doctor` (`zohara-settings/scripts/fake-kscreen`, put first in `PATH`): the brightness slider dragged to 26 % sent `brightness.26`;
  HDR on showed "Keep these display settings?" with the countdown and, left alone, sent `hdr.enable` then `hdr.disable` (reverted); HDR on + "Keep changes" stayed on and showed the "Brightness of normal content" slider (300 nits, read from the fake).
  Not clicked: VRR, wide gamut, RGB range, colour profile (their commands are unit-tested), mirror / on-off / main display.
* **Apps from archives**: right-click an archive in Dolphin > Zohara > "Install as an app" installed the app (notification "Hello is installed"); the same on a source-code archive gave the plain refusal;
  Settings > Apps > "Install from an archive…" opened the file chooser, then the "Install Hello?" box (name, size, explanation), installed a second copy, listed both, "Remove" asked for confirmation and removed it; the app is found in the Start menu search.
  Not tested: launching the installed app from the menu (it has no window), real large archives.
