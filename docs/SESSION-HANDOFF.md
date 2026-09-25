# Zohara OS: session handoff (2026-09-25/26)

Read this first when resuming. It records where everything stands, what is
untested, and what to do next. No secrets are stored here.

## Working rules

- **No Python in the OS itself.** Anything that ships (apps, services, packaged
  tools, CI that builds the OS, the ISO package list) is Rust or shell, with `jq`
  for JSON. Python is fine for throwaway helper scripts on the dev machine only.
- **Everything should be clickable, not typeable.** Any flow that sends a user to a
  terminal is a bug to fix.
- **No local GTK/Rust toolchain on the dev machine.** Verification is GitHub
  Actions only. Never claim something compiled unless a CI run shows it. Nothing
  below has been run on a booted system yet.
- Commit trailer: `Co-Authored-By: Claude <model> <noreply@anthropic.com>` (as set
  by the session's attribution reminder).
- Docs-only pushes to `zohara` `master` (`*.md`, `docs/`, `.gsd/`, `.agent/`) do
  not start an ISO build. Any other push does (about 25-30 minutes).

## Repositories

| Repo | What | CI |
|---|---|---|
| `Zohaib8090/zohara` | ISO build, Store (`zohara-store-rs`), welcome (`zohara-welcome`), voice (`zohara-voice`, `zohara-voice-model`), snapshots (`zohara-snapshots`), profile | ISO, Store, Welcome, Voice, Snapshots workflows |
| `Zohaib8090/zohara-settings` | Settings app (Rust, GTK4, libadwaita) | Build & Publish Zohara Settings |
| `Zohaib8090/zohara-packages` | Package repo: release `stable` holds `zohara.db` and the `.pkg.tar.zst` files; `publish.yml` rebuilds the db | Publish Zohara Package |

Local clones live in `C:\Users\Zohaib Baig\Documents\`. `zohara-store-rs/` is in
that repo's `.gitignore`, so new files there need `git add -f`.

## What is built (all CI-green at the time of writing)

**Settings** (`zohara-settings`): every page is backed by real system state
(Plasma 6 Wayland, NetworkManager, BlueZ, CUPS, ufw, ...). Recent additions:
Printers, voice typing group (Accessibility), firewall rules and access
indicators (Privacy), system restore points (Storage), Troubleshoot page with a
background health check, Zohara Update page that opens the Store.

**Voice typing:** `zohara-settings --dictate` on Meta+H. Records with `parecord`,
transcribes locally with whisper.cpp, pastes with `wl-copy` + `ydotool`.
Packages: `zohara-voice` (engine, built from source in CI, AVX2 and baseline
variants) and `zohara-voice-model` (base q8_0, 82 MB).

**Privacy indicators:** `zohara-settings --privacy-indicator` (autostart) shows
coloured tray dots: green camera, orange microphone, red location. It hides
Plasma's own camera and microphone tray items through plasmashell scripting.
Activity history in `~/.local/state/zohara/privacy-access.log`.

**Store > Updates** (`zohara-store-rs/src/updates.rs`, `updates_ui.rs`): checks with
`checkupdates` (system, Zohara) and `flatpak remote-ls --updates`; tick what to
update or Update all; live progress; Go back (undo last update from
`/var/log/pacman.log` + pacman cache, per-package older versions, Flatpak
commits); "Restore the whole system" lists Btrfs snapshots. System packages are
updated together (`pacman -Syu`); Zohara apps and Flatpaks can be picked one by one.

**Packages** (all in the `stable` repo, ISO installs the same ones via
`pacman -U`): `zohara-settings`, `zohara-store`, `zohara-welcome` (welcome +
migrate, Rust), `zohara-voice`, `zohara-voice-model`, `zohara-snapshots`,
`zohara-linkd`. Versions are `<cargo>.<UTC build time>` (Settings, Store,
Welcome) so every build is newer; the voice, model and snapshots packages only
update when `pkgver`/`pkgrel` in their PKGBUILD is raised.

**Update path:** push -> CI builds the package -> uploads to the `stable`
release in `zohara-packages` -> dispatches `publish.yml` -> installed systems
see it in Store > Updates. `[zohara-stable]` is enabled in the ISO's
`pacman.conf` (`SigLevel = Optional TrustAll`, so packages are unsigned).
Publishing needs the `PACKAGES_DISPATCH_TOKEN` secret in each source repo
(a GitHub token with write access to `zohara-packages`; it expired once and
was replaced; the user revokes tokens weekly, so expect to reset it with
`gh secret set PACKAGES_DISPATCH_TOKEN -R Zohaib8090/<repo>`).

**Welcome / migrate** (`zohara-welcome`, Rust): replaced four PyQt5 tools.
Migration runs as an unprivileged window that drives a pkexec helper; the
engine never overwrites files, keeps symlinks, restores ownership, and only
removes the old system when every file moved and the user opted in.
`zohara-usermgr` and `zohara-update` were retired (Settings has both).

**System snapshots** (`zohara-snapshots`, Calamares config): installer defaults to
Btrfs with subvolumes `@ @home @snapshots @log @pkg`; snapper (root config,
10 snapshots, 20% free limit), snap-pac snapshots every pacman transaction,
grub-btrfs lists them in the boot menu, `zohara-snapshots restore N` swaps the
system subvolume for a snapshot (Store button, restart to finish).

## In flight when this was written

- ISO run `36184373149` (master, includes snapshots) was still building. Check
  it: `gh run view 36184373149 -R Zohaib8090/zohara`. Artifact: `zohara-os-x86_64`.
- Earlier finished ISO (no snapshots): run `36178012044`.

## Untested (needs a VM; see `docs/VM-TEST-CHECKLIST.md`)

- **The installer's Btrfs layout is the riskiest change.** `partition.conf`,
  `mount.conf` (subvolumes) and `shellprocess_snapshots.conf` are written from
  Calamares documentation, never run. If installing fails, revert the default
  filesystem to ext4 in `partition.conf` first.
- Snapshots: first-boot setup, snap-pac hooks, grub-btrfs boot entries, the
  `restore` script (has a `ZS_DRY_RUN=1` mode), Store restore list.
- Plasma details written from memory: PowerDevil keys, plasmanotifyrc DND,
  kaccessrc, the systray "always hidden" key used to hide Plasma's indicators,
  kglobalaccel signatures, Chromium `--class` on Wayland, Organic Maps paths.
- Voice typing end to end (`ydotool` daemon and `/dev/uinput` access).
- Migration tool disk steps (mount, package install, delete old system).
- Store update flow with a real repo: `checkupdates` against `[zohara-stable]`,
  Go back, Undo last update.

## Known gaps

- Packages in `zohara-packages` are unsigned.
- Zohara packages are built on the newest Arch in CI. Fine today; it becomes a
  real problem once users are pinned to an older date (see below).
- Python still in the OS side: `python3 -c` JSON checks in
  `zohara/.github/workflows/build-iso.yml`; the `apps.json` patch step in
  `zohara-packages/.github/workflows/publish.yml`; `python`, `python-pywebview`,
  `python-gobject`, `python-dbus`, `python-pydbus`, `pyside6` in
  `zohara-profile/packages.x86_64` (check nothing else needs them first).
- Still only via a new ISO: `pacman.conf`, SDDM/boot/desktop configuration.
- `zohara-linkd` version is still fixed at `0.1.0-1`, so it never shows as an update.
- Not started: window rules, removable-storage policies, a configuration
  package for system settings.

## The "Automated Update Pipeline Plan" (user's plan, reviewed)

Upload: `Zohara_OS___Automated_Update_Pipeline_Plan.md` (Phases 1-5: dated
Arch snapshot repo + signed manifest + client updater; diff and risk tiers;
VM upgrade tests; AI monitoring agent; approval bot + bare-metal canary).

Verdict: sound, and it fits what exists. Changes recommended before Phase 1:

1. **Build Zohara's own packages against the pinned date.** CI builds on the
   newest Arch; users on an older approved date would run binaries linked against
   newer libraries and crash. Use the same dated snapshot in the build containers.
2. **One updater, inside the Store.** Every update goes through the Store, so the
   plan's `zohara-updater` should be the Store's engine (or a helper it calls),
   not a second app. Manifest check, keyring refresh and the pre-update snapshot
   run when the user clicks Update. Snapshots and rollback already exist.
3. **Hold the whole date, not single packages.** Holding one package while the
   rest updates is a partial upgrade and pacman may refuse it over dependencies.
   Try the next day's snapshot until everything passes; reserve per-package pins
   for rare cases.
4. Security fast-track moves the whole date too, so it still needs a VM pass.
5. Sign the Zohara repo (packages and database) with pacman keys, in addition to
   the signed manifest.
6. The Arch Linux Archive has only the official repos. Zohara's repo, Flathub and
   Chaotic-AUR are outside the pinned date. Host a snapshot mirror behind a
   caching proxy rather than have every user hit the archive.
7. The self-hosted Nitro runner must only run the owner's scheduled or manual
   jobs (never pull requests from strangers).
8. The ISO must be built with the same `approved_date` baseline.
9. The AI agent (Phase 4) is advice next to VM and canary results, not a gate.
   Reddit/forum scraping is unreliable.
10. Phase 3 and 4 tooling: the plan says Python. Those run on the owner's servers,
    not in the OS, but Rust or shell is preferred for one stack.
11. Reuse Settings' Rust health check (`zohara-settings --health-check`, add a
    JSON output) for the VM tests, the updater and the canary.

## Proposed next step

Plan Phase 1 (snapshot repo, signed manifest, client updater in the Store) with
points 1, 2 and 8 built in. Write the plan first, get approval, then build. For
each "Done when" item, show real command output.

Before that, verify the in-flight ISO, then boot it in a VM and go through the
checklist, starting with the installer (Btrfs) and Store > Updates.

## Update 2026-09-26: Python removed, Phase 1 started

- OS-side Python removed: `build-iso.yml` uses `jq`; `zohara-packages` `publish.yml`
  patches `apps.json` with `jq` (tested locally against the real `apps.json`);
  python packages dropped from `packages.x86_64`.
- Phase 1 built: `zohara-store-rs/src/manifest.rs` (minisign verify, strict date,
  no-backwards, `-Sy` guard, pinned check via temp pacman.conf, `--pin-date` root
  helper); 20 Store tests green in CI. New public repo `Zohaib8090/zohara-pipeline`
  (manifest format, `tools/approve.sh`).
- **Inactive until the owner generates the signing key** (README in zohara-pipeline)
  and commits the public half as `zohara-store-rs/data/manifest.pub`.
- Still to do in Phase 1: build Zohara packages in a container pinned to the
  approved date; build the ISO at the same date; JSON output for the health check;
  VM proof of GRUB entry and rollback.
