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
**Verified 2026-10-20 on a fresh install of the fixed ISO** (CI run `39401706577`, commit `586c0b7`, file `zohara-os-2026.10.20-x86_64.iso`, SHA-256 `686e3499c1f98a2c5c5d17b24ebeaef2c0aeed1850fb2285c4758eedd6479dcc`, kept in `~/Documents/zohara-iso-fixed/`):
`systemd-networkd` and its socket are **masked** and inactive, NetworkManager active, one IPv4 address; `zram0` (1.9 G, zstd, priority 100) is active in front of the disk swap;
`/etc/hostname` is `zohara` (the Users step suggested `zohara` as soon as a name was typed); `/etc/locale.conf` holds only `LANG=en_US.UTF-8` and the clock shows 10/20/26 (US format).
Not clicked: Settings' zram switch showing "on" (it reads the live swap list, so it should). The installer's Summary page still *previews* "numbers and dates: Urdu"; the new step removes it afterwards.
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

## Swap and hibernation test (same day, fixed ISO, VM 3.8 GB RAM)

**What the two installer choices do.** Partition step swap choices: "No swap", "Swap (no Hibernate)" (default, `small`), "Swap (with Hibernate)" (`suspend`).
Both installed systems have the **same hibernate setup**: `resume=UUID=<swap>` on the kernel line, the `resume` hook in `mkinitcpio.conf` (`... plymouth resume filesystems ...`, initramfs rebuilt after), `logind CanHibernate = yes`, a Hibernate button on the sign-in screen.
The only difference is the swap size: **5.95 GiB** (no Hibernate, about 1.5 x RAM) versus **8.37 GiB** (with Hibernate) on this 3.8 GB VM; the root partition shrinks from 53.55 to 51.12 GiB. With zram on (new default) the VM shows 7.85 / 10.5 GiB of swap in total.
So "no hibernate" is only a smaller swap partition, and the Hibernate button is still offered.

**Hibernate in the VM (nothing on the owner's laptop was hibernated).**
| Way | Result |
|---|---|
| `systemctl hibernate` (what the Hibernate button runs), 5 tries (default mode, `HibernateMode=shutdown`, zram on and off, fresh install of either swap choice) | image written to the swap partition (header `S1SUSPEND`, kernel log "Image saving done", "Power down"), but at boot: `Unable to resume from device ... (259:3)`, `PM: Image not found (code -22)`; **cold boot, session lost** |
| direct kernel path `echo shutdown > /sys/power/disk; echo disk > /sys/power/state`, 5 tries | 4 of 5 resumed with the whole session back (`PM: Image signature found, resuming`, `Image successfully loaded`); the one failure was the first direct try |
Header bytes: systemd default mode writes flags 0x5 (platform + CRC), `HibernateMode=shutdown` and the direct path write 0x4; both resumed or failed independently of that, so the flag is not the cause. The kernel command line (serial console, `quiet splash`) did not decide it either.
**Cause not found.** It may be specific to QEMU (NVMe emulation, ACPI) or real; it needs a test on real hardware before anyone relies on Hibernate. A failed hibernate here looks like a normal boot (cold boot, login screen), so unsaved work would be lost.

**Owner's laptop (read only, 2026-10-20):** `resume=UUID=` points at the 8.8 GiB swap partition; RAM 15.3 GiB; also zram 7.7 GiB (priority 100) and a 16 GiB swap file (priority 10); `CanHibernate = yes`. systemd chooses the hibernation target by swap priority, but the initramfs here is the udev kind: its `resume` hook reads only `resume=` from the kernel line, so an image written to the swap file or zram would not be found at boot.

## Problems to look at (not fixed)
1. **Hibernate button may lose the session** (above). Needs a real-hardware test; options: point systemd and the kernel at the same device (`resume=` partition, and exclude zram/swap file), or hide Hibernate until a test passes.
2. **"Swap (no Hibernate)" still offers Hibernate** everywhere (same `resume=` and hook). Either do not set `resume=` for that choice, or hide the button.
3. **Settings' hibernate check adds up all swap** (`backend/hibernate.rs`, `swap_kib` sums every line of `/proc/swaps`, zram and swap file included): the laptop shows 33 GiB although the resume partition is only 8.8 GiB. It should count only the swap `resume=` points at, and never zram.
4. With zram now default at priority 100, hibernation could pick zram as the target (not proven: zram on or off, the systemd path failed in the same way).

## Fixes for problems 2 and 3 (2026-10-20, after the test above)
* **3 (Settings)**: `zohara-settings/src/backend/hibernate.rs` now counts only the swap that `resume=` names and never zram; 9 unit tests; on the owner's laptop it now answers "cannot hibernate" (resume partition 8.8 GiB < RAM 15.3 GiB), the old check said yes.
* **2 (installer)**: new Calamares job `hibernatefix` (`zohara-profile/airootfs/usr/lib/calamares/modules/hibernatefix/`, listed after `grubcfg` and before `bootloader` in `settings.conf`). Calamares keeps the choice in global storage as `partitionChoices.swap`
  (`none`, `small`, `suspend`, `reuse`, `file`). For `none` and `small` it removes `resume=` from `/etc/default/grub` and writes `/etc/systemd/sleep.conf.d/10-zohara-no-hibernate.conf` (`AllowHibernation=no`, also suspend-then-hibernate and hybrid) so the sign-in screen and power menu do not show Hibernate;
  "Swap (with Hibernate)" is left alone. It never stops an install (errors are logged and skipped). Checked with a fake `libcalamares` and temporary roots for every choice; **not yet run in a real install** (needs a new ISO).

