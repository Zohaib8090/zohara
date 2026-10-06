# What must be true before Zohara OS is called "beta"

Written 2026-10-06. Today the system is an **alpha**: it installs and updates in test VMs, but it has had no real-world use.
"Beta" means: people who are not the author can install it on their own computers, use it for weeks, and it does not lose
their data or leave them unable to boot or update. Tick a box only with proof (a test run, a screenshot, a CI run id, a
tester's report). Items marked **(owner)** need the owner, not Claude.

## 1. Install and boot (the first five minutes)

- [ ] The ISO installs from a USB stick on **at least 3 different real computers** (one Intel laptop, one AMD, one with an
      NVIDIA GPU), and each boots the installed system from its own disk. Today: VMs only.
- [ ] Install works on a disk that already has Windows or another Linux (dual boot), and the other system still boots.
- [ ] Install works on a disk with nothing on it, and on a small disk (about 40 GB).
- [ ] Install works with Wi-Fi only (no cable) and fully offline.
- [ ] The installer's last page restarts into the installed system (the live USB must be removed first; say so on screen).
- [ ] **Welcome** shows on the live USB and on the first sign-in after install, and **not again** on later sign-ins
      (marker `~/.config/zohara/welcome-seen`; checked in the VM 2026-10-06: still to confirm).
- [ ] After install: `zohara-keyring` is installed and Zohara's key is trusted (`pacman-key --list-keys`).
- [ ] Secure Boot: either supported (see `docs/SECURE-BOOT-PLAN.md`) or the install page tells the user plainly to turn it off
      and the ISO page explains why. Today: deferred; the website must say it.

## 2. Updates (the thing that must never break)

- [ ] A fresh install finds updates on the Store's Updates page ("Everything is up to date" or a list), with no error.
- [ ] One full update cycle on a fresh install: publish a new build to alpha, install it from the Store, reboot, still works.
- [ ] **Undo the last update** and **restore points** each tested once on a real install after a deliberately bad update.
- [ ] Switching channel stable, beta, alpha and back works (tested in a VM 2026-10-06) and a machine on alpha receives an alpha build.
- [ ] An update that fails halfway (network cut) leaves a working system and a clear message with a Retry.
- [ ] The approved Arch date in the signed manifest is never older than the date the ISO pinned (rule in `zohara-pipeline/README.md`).
- [ ] Package signing is **on**: `ZOHARA_PKG_SIGNING_KEY` added **(owner)**, machines switched to `SigLevel = Required DatabaseRequired`,
      and a tampered package is refused (test exists: `zohara-packages/scripts/test-signing.sh`).
- [ ] Keys backed up in two places **(owner)**: `~/.zohara-signing/` and `~/.minisign/` (a USB copy was made 2026-10-06; a second copy is not).

## 3. The apps people touch every day

- [ ] **Store**: install, remove and Retry an app from Arch, from Flatpak, and a game; progress is correct for each (Flatpak progress is untested).
- [ ] **Settings**: every page opens and every visible button does something or is hidden (no dead rows); checked page by page on an installed system.
- [ ] Wi-Fi, Bluetooth, sound, display (including 2 monitors), battery and power modes work on the 3 test computers.
- [ ] Printing and a USB drive work. Suspend and resume work.
- [ ] The theme follows the system dark and light setting in Settings, the Store and Welcome.
- [ ] The two packages named `zohara-welcome` (from `zohara-apps` and from `zohara/zohara-welcome`) are merged into one.

## 4. The release pipeline

- [ ] Alpha publishes by itself from a push (done 2026-10-06 for Settings, Apps and the Store); Snapshots, Voice, Welcome and Keyring still to run once.
- [ ] Beta and stable move only from the admin site, and the admin site lists the Store and the other `zohara` packages too.
- [ ] The ISO promote button has run for real, the website link and hash are updated, and a downloaded ISO matches its published hash.
- [ ] The secret cleanup is done **(owner)**: rotate the GitHub App client secret, delete the local `.pem`, the client-secret `.txt` and `~/zohara-hub.env`.

## 5. Real users (the part no test replaces)

- [ ] 5 to 10 testers, on different hardware, run the alpha for **2 weeks** and report: crashes, things that did not work, updates that failed.
- [ ] A place to report problems that testers can find and use (and the "Save a problem report" button on the live USB works).
- [ ] No data-loss bug and no "cannot boot" bug open at the end of the 2 weeks. Every other open bug is written down with a decision.
- [ ] One person other than the author can follow `docs/` and build the ISO and publish a package.

## When to move on

- **Alpha to beta:** every box in sections 1, 2 and 4 ticked, sections 3 and 5 ticked for what testers actually used.
- **Beta to stable:** 4 more weeks of beta without a boot, update or data-loss bug, Secure Boot answered, and a written promise of what stable means
  (how long a release is supported, how a security fix reaches users).
