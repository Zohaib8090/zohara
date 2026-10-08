# Is Zohara OS ready to be the only system on the owner's laptop? (assessment, 2026-10-08)

Question from the owner: remove Zorin and install only Zohara. Answer: **not yet as the only system. Dual-boot it first.**
This is based on what the docs and test records prove, not on a fresh full test. "Proven" below means a VM click-test, a
CI run or the owner's own photo/report is on record. Nothing here is a guess dressed as a result.

## What is proven (use it with confidence)

* **Installs and boots**: installed from the ISO in a VM many times; installed on a real Dell Latitude (ISO 2026-09-28) after
  the Calamares Btrfs fix. The owner's own laptop runs Zohara updated from the alpha/stable channels (photos 2026-10-08).
* **Updates**: Settings > Zohara Update finds a signed update, installs it with a restore point before and after, and
  reports "up to date" (VM, repeated). Settings builds promoted to stable and installed by update.
* **Settings and Store pages** click-tested in the VM this week: Personalization (themes, taskbar, fonts), Power & battery,
  Apps (web apps with automatic icon and browser choice, Startup apps), Storage cleanup, Privacy (KDE Wallet switch),
  Accounts (passwordless standard users), time zone (automatic, survives reboot), terminal logo, About / OS version 1.1.
* **On the owner's laptop** (reported or photographed): wallet switch, web app icon, new terminal logo, Zohara logo on the
  start button, web apps (WhatsApp, YouTube, Facebook) in the taskbar.

## What is NOT proven (the reasons not to remove Zorin yet)

From `docs/BETA-CHECKLIST.md` (the project's own bar: today it is an **alpha**, no real-world use, no 2-week test):

1. **Suspend and resume**: an open bug on record ("after suspend it logs out and reopens", cause not understood) and the
   Wi-Fi-after-sleep hook is untested on hardware. A laptop that cannot sleep reliably is a daily problem.
2. ~~Wi-Fi on a second laptop is unresolved~~ **Fixed (owner reported, 2026-10-08)** on the owner's dad's laptop; the cause was not recorded. It is still only two machines. Wi-Fi on the owner's own laptop
   works, but that is one chip.
3. **Hibernate**: Settings offers it, but nothing proves the installer creates swap large enough. Test before relying on it.
4. **Never tested on real hardware**: touch-screen keyboard button, sign-in screen keyboard, theme reset restoring the
   taskbar, OpenRGB, Lock screen pickers, real Windows-app launchers (Wine/Proton), lid and low-battery options, a big
   full-system update through the Update page, Flatpak install progress and Retry, Flatpak Chrome as a web-app browser.
5. **Dual boot and existing disks** are on the checklist as untested; only the Dell clean install is on record.
6. **Package signing is off** and **Secure Boot must be turned off** (both deliberate, both deferred).
7. **A broken update is only covered by VM tests.** Restore points (Btrfs snapshots, also in the GRUB menu) are the safety
   net; they have not been used to recover a real laptop after a bad update.
8. **No beta period**: nobody but the owner has lived on it for weeks.

## The other risk: this laptop is also the development machine

Builds of the ISO (Docker, ~30 GB), the repos in `~/Documents`, the Rust toolchain, `gh`, the OCI and SourceForge logins and
the signing keys all live in the current system. Removing it means rebuilding that whole workflow on Zohara first, and a
mistake in the middle would stop development, not just inconvenience a user.

## Recommendation

1. **Dual boot, not replace.** Make the 60 to 100 GB partition from a live USB (GParted), mount the existing EFI partition at
   `/boot/efi` **without formatting it**, Btrfs for `/`, Secure Boot off.
2. **Use Zohara daily for 1 to 2 weeks** and write down every problem (the Settings "Save a problem report" button helps).
3. **Switch fully only when** all of these are true on this laptop: suspend/resume and lid close work for a week; Wi-Fi,
   Bluetooth, sound, display, battery behave; one real system update plus "Undo the last update" has been done;
   Docker, Rust, `gh` and `oci` are set up in Zohara and an ISO builds there; the keys in `~/.zohara-signing/` and
   `~/.minisign/` have **two** backups and the home folder is backed up.
4. If the owner still wants to switch now: back up everything first, keep the Zorin ISO on the Ventoy stick, and expect to
   debug on the machine you also need for work.
