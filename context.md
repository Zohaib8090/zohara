# Zohara OS - Master Context & System Architecture

This document serves as the **Single Source of Truth** for Zohara OS. It details the complete system state, architecture, component breakdown, store version control schema, build pipeline, and GitHub repositories.

---

## 1. Executive Summary & Repositories

Zohara OS is an independent, custom Arch Linux-based distribution built with `mkarchiso` inside a containerized Docker environment (`zohara-builder`).

### GitHub Ecosystem
1. **Main OS Repository**: [`Zohaib8090/zohara`](https://github.com/Zohaib8090/zohara.git)
   - Contains `zohara-profile`, `Dockerfile`, `localrepo`, build configuration, and ISO scripts.
2. **Settings Application**: [`Zohaib8090/zohara-settings`](https://github.com/Zohaib8090/zohara-settings.git)
   - Standalone Rust + GTK4 / Libadwaita system settings application (`zohara-settings-rs/`).
3. **Package Index & Version Store**: [`Zohaib8090/zohara-packages`](https://github.com/Zohaib8090/zohara-packages.git)
   - Contains `apps.json` with multi-version metadata and upgrade/downgrade URL endpoints.

---

## 2. Core Components Breakdown

### A. Zohara Settings App (`zohara-settings-rs`)
* **Framework**: Rust, GTK4 (`gtk4`), Libadwaita (`libadwaita`), `zbus` (D-Bus IPC), `tokio` multi-threaded runtime.
* **Architecture Fix**: A global Tokio runtime (`TOKIO_RUNTIME` static `OnceLock<Runtime>`) handles all async D-Bus operations (WiFi rescan, D-Bus property fetching, etc.) off the main GTK GLib main loop. Results are dispatched back to the UI thread using `glib::idle_add`.
* **Pages** (one module per file in `zohara-settings-rs/src/pages/`):
  - `home`: Landing/overview page.
  - `network`: Wi-Fi scanning/connecting and Ethernet status (currently driven via `nmcli`).
  - `bluetooth`: Device discovery and pairing (currently driven via `bluetoothctl`).
  - `personalization`: Theme, dark mode, accent colors, and desktop layout.
  - `accounts`: User creation/deletion, admin toggles, and password changes via Polkit.
  - `system`: System specs, kernel/build metadata, and storage cleanup.
  - `updates`: System OTA & package updates panel.
  - `apps`: Installed application management.
  - `time_language`: Clock, timezone, and locale settings.
  - `privacy`, `accessibility`, `gaming`, `advanced`: Additional settings sections.
* **Packaging & CI**:
  - `.github/workflows/build-iso.yml`: Builds the ISO via the `zohara-builder` Docker image, validates the UEFI boot payload, and publishes a Release on `v*` tags.
  - `.github/workflows/build-update.yml`: Builds the `zohara-system` OTA package and `zohara.db` repository database, published as Release assets.
  - `packages/zohara-settings/PKGBUILD`: **Currently orphaned** — it clones a standalone
    `zohara-settings` repo, but the real pipeline builds `zohara-settings-rs/` in-tree with
    `cargo build --release` and installs the binary directly. Do not treat it as the source of truth.

### B. Zohara Store & Package Index (`zohara-packages`)
* **Location**: `https://github.com/Zohaib8090/zohara-packages.git` (`apps.json`).
* **Native Version Control System**:
  Tracks full version histories for all Zohara and third-party software, enabling version selection, direct upgrades, and rollbacks/downgrades via native `pacman -U <download_url>`.

#### `apps.json` Version Control Schema:
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

---

## 3. ISO Build Pipeline & Docker Setup

The ISO is built using `mkarchiso` inside a custom Arch Linux Docker container (`zohara-builder`).

### Key Docker Configurations
* **`Dockerfile`**:
  - Pre-builds AUR/binary packages into an offline local repo at `/opt/localrepo`, registered with
    `repo-add`: `brave-bin` via `pacman -Sw` from Chaotic-AUR, and `debtap` + `calamares` compiled
    from AUR source with `makepkg -s` (calamares exists in no binary repo — see the note in §4).
  - `COPY`s the in-tree `zohara-settings-rs/` and `zohara-store-rs/` directories into the image (it does **not** clone them from GitHub) and compiles both with `cargo build --release`.
  - The `ENTRYPOINT` installs the two release binaries straight into
    `zohara-profile/airootfs/usr/bin/` with `install -Dm755` (no PKGBUILD involved), then runs
    `mkarchiso` and `create_update_bundle.sh`.
* **`.dockerignore`**: Excludes `out`, `localrepo`, `work`, `pkg-cache`, `*.iso`, `.git`, and `**/target` to optimize Docker build context transfer.

### Step-by-Step ISO Build Commands
If you encounter a `failed to synchronize all databases (unable to lock database)` error, clean the stale `work/` directory first.

```bash
# Step 1: Clean build artifacts and stale pacman locks
sudo rm -rf ~/Documents/my/zohara/work/ ~/Documents/my/zohara/out/

# Step 2: Rebuild the Docker builder image
docker build -t zohara-builder ~/Documents/my/zohara

# Step 3: Run the ISO build container
sudo docker run --rm --name zohara-build \
  --privileged \
  -v ~/Documents/my/zohara:/build \
  -v ~/Documents/my/zohara/pkg-cache:/var/cache/pacman/pkg \
  zohara-builder
```
> The generated ISO is written to `~/Documents/my/zohara/out/`.

---

## 4. Key File Locations

| File / Folder | Purpose |
| :--- | :--- |
| `zohara-settings-rs/` | Local workspace for GTK4/Rust Settings app |
| `packages/zohara-settings/PKGBUILD` | Package build spec for `zohara-settings` |
| `zohara-profile/` | Archiso profile (packages, desktop configs, `customize_airootfs.sh`) |
| `Dockerfile` | Multi-stage Docker builder script for Zohara OS ISO |
| `.dockerignore` | Context exclusion rules for Docker build |
| `build command.txt` | Quick reference commands for building the ISO |
| `/media/zohaib/KINGSTON/zohara-packages/` | Local clone of `Zohaib8090/zohara-packages` |

> **Verified facts that contradict older docs** (checked against the tree and the built ISO on 2026-08-23).
> Note `.gsd/ARCHITECTURE.md` and `.gsd/STACK.md` are **gitignored local state** regenerated by `/map`;
> if they disagree with this list, they are stale, not this file.
>
> - **Binaries live in `/usr/bin`, not `/usr/local/bin`.** The Dockerfile `ENTRYPOINT` installs
>   `zohara-settings` and `zohara-store` to `/usr/bin/` and then `rm -f`s the `/usr/local/bin/` copies.
> - **`zohara-appearance` does not exist.** No file of that name anywhere in the repo, and nothing in
>   `zohara-profile/` references it. Older docs listed it as a shipped first-party tool.
> - **The store has no catalog loader.** `zohara-store-rs/src/` contains no `apps.json`, no remote URL,
>   and no `~/.config/zohara-store` state. `ENTRYPOINT` also `rm -rf`s
>   `airootfs/usr/share/zohara-store/`, so there is no bundled offline catalog either. Everything in
>   `zohara_store_packages_guide.md` sections 2–4 is a target design, not shipping behaviour.
> - **Only two crates are built.** The Dockerfile compiles `zohara-settings-rs` and `zohara-store-rs`
>   only — `zohara-migrator` and `zohara-theme-engine` are never built or installed.
> - **Four shipped tools are PyQt5**, not PySide6: `zohara-welcome` (autostarted every login),
>   `zohara-usermgr`, `zohara-migrate`, `zohara-update`. They need `python-pyqt5` in
>   `packages.x86_64`; it was missing, so all four died with `ModuleNotFoundError` on a booted ISO.
> - **Windows checkouts cannot materialize the 18 systemd symlinks** under
>   `airootfs/etc/systemd/` (no `SeCreateSymbolicLinkPrivilege`). They are held with
>   `git update-index --skip-worktree`. Committing them as deleted would ship an ISO with no
>   `display-manager.service` (no GUI login) and no `sshd.service`.
> - **`[zohara]` must stay disabled** in `pacman.conf` and `customize_airootfs.sh` until a release
>   actually publishes `zohara.db` — pacman treats an unreachable repo DB as fatal and every
>   `pacman -Sy`/`-Syu` fails.
> - **`calamares` has no binary repo any more, and this makes the ISO unbuildable.** Chaotic-AUR
>   dropped it between 2026-08-18 and 2026-08-23 (its DB still lists `brave-origin-bin`, `debtap` and
>   `latte-dock-ng`, but zero `calam*`, and `calamares-3.4.2-2-x86_64.pkg.tar.zst` now 404s on the
>   mirror). Calamares has **never** been in official Arch repos, so the `Dockerfile` comment reading
>   "calamares is in packages.x86_64 — mkarchiso installs it from official mirrors" was always wrong.
>   `mkarchiso` now aborts with `error: target not found: calamares`. It must be built from AUR into
>   `/opt/localrepo` (AUR still carries **3.4.2-2**, the same version the 2026-08-17 ISO shipped;
>   `depends`: `kcoreaddons kpmcore libpwquality qt6-declarative qt6-svg yaml-cpp`).
> - **`qt6-declarative` is not removable.** It is a hard `depends=` of `calamares`. Older notes listed
>   it as dead weight from the retired PySide6 app; dropping it would break the installer.
> - **The host `localrepo/` directory is not used by the Docker build.** It is both `.gitignore`d
>   (`.gitignore:24`) and `.dockerignore`d, so its hand-built July packages — including a
>   `calamares-3.4.2-2` — never reach the image. `[localrepo]` resolves to `file:///opt/localrepo`,
>   which only ever contains what the `Dockerfile` itself puts there.
> - **Never symlink a desktop ID that `HIDE_APPS` also hides.** `customize_airootfs.sh` used to
>   `ln -sf` `systemsettings.desktop` → `zohara-settings.desktop` and *also* list `systemsettings` in
>   `HIDE_APPS`. Both `[[ -f ]]` and `>>` follow symlinks, so `echo NoDisplay=true >>` wrote through
>   into `zohara-settings.desktop` — the 2026-08-23 ISO therefore ships Zohara Settings **hidden from
>   the application launcher** (verified: `NoDisplay=true` is the last line of that file inside
>   `airootfs.sfs`). The aliases are now real files carrying their own `NoDisplay=true`, and the hide
>   loop skips symlinks outright. A symlink was also a second menu entry with the same `Name=`, so it
>   produced duplicate "Settings"/"Software Store" tiles.
> - **Never put a host machine's `/etc/resolv.conf` in the overlay.** The tracked
>   `airootfs/etc/resolv.conf` was a verbatim copy of the build machine's systemd-resolved stub and
>   carried a private VPN `search` domain, so the 2026-08-23 ISO shipped that internal network name to
>   every user and forced a guaranteed-to-fail DNS lookup on every single-label hostname. DNS still
>   *worked* (`systemd-resolved.service` is enabled and `nsswitch.conf` uses
>   `resolve [!UNAVAIL=return]`, so `127.0.0.53` has a real listener) — the bug was the leak plus the
>   wasted round trip, not an outage. **Verified in the shipped 2026-08-23 `airootfs.sfs`:
>   `/etc/resolv.conf` was a 936-byte regular file whose last line was `search tail444ae0.ts.net`** —
>   the leak really did reach users, it was not merely a repo-side mistake. The file is now a sanitized
>   placeholder, and the handoff to resolved happens via `airootfs/etc/tmpfiles.d/zohara-resolv.conf`
>   (`L+ /etc/resolv.conf … /run/systemd/resolve/stub-resolv.conf`).
> - **An `L+` tmpfiles rule takes effect during the build, not just at boot.** Arch's
>   `30-systemd-tmpfiles.hook` runs `systemd-tmpfiles --create` on every pacman transaction, and `L+`
>   is a runtime line (systemd's own `/usr/lib/tmpfiles.d/systemd-resolve.conf` uses `L!`, which is
>   boot-only and therefore never fired here). Because the overlay is copied *before* pacstrap, the rule
>   is already in place when that hook first runs — so the 2026-08-24 `airootfs.sfs` ships
>   `/etc/resolv.conf` as an actual symlink to the stub, and the rule then re-asserts it on every boot.
>   That is why the sanitized placeholder does not appear in the image at all; it only covers the case
>   where that hook ordering ever changes.
> - **`customize_airootfs.sh` can never write `/etc/resolv.conf`.** `arch-chroot` — which mkarchiso
>   uses to run that script — bind-mounts the *build host's* `/etc/resolv.conf` over the chroot's copy
>   so pacman can resolve names during install, so the path is an active mountpoint for the whole run.
>   `ln -sfn` there fails with `ln: failed to create symbolic link '/etc/resolv.conf': Device or
>   resource busy`, and since the script uses `set -e` that **aborts mkarchiso and produces no ISO**.
>   This was tried on 2026-08-24 and killed the build after ~35 min, right after
>   `-> Enabling System Services...`. Use `tmpfiles.d` (applied at boot, where nothing is bind-mounting
>   the path) — not the chroot, and not an overlay symlink, since Windows checkouts cannot create one.
> - **`packages.x86_64` must carry `mkinitcpio-nfs-utils` and `nbd`.** `mkinitcpio-archiso` enables
>   the PXE hooks by default (the built initramfs has
>   `HOOKS="udev memdisk archiso archiso_loop_mnt archiso_pxe_common archiso_pxe_nbd archiso_pxe_http archiso_pxe_nfs"`),
>   but the binaries those hooks need lived in packages that were never listed. Every build ended
>   pacstrap with `ERROR: file not found: '/usr/lib/initcpio/ipconfig'`,
>   `ERROR: binary not found: 'nbd-client'`, `ERROR: file not found: '/usr/lib/initcpio/nfsmount'` and
>   `error: command failed to execute correctly`. **USB/DVD/VM boot was never affected** — that path
>   uses `archiso` + `archiso_loop_mnt`, and both are intact in the image — so this only broke PXE
>   netboot, but it made every build log look like a failed build. Upstream archiso `releng` ships both.
> - **The ISO initramfs is a two-segment image.** `file` reports "ASCII cpio archive" and `cpio -t`
>   exits 0 after ~3.2k members, because segment 1 is the *uncompressed* early cpio (`early_cpio`
>   marker, microcode, and the already-`.zst` firmware/modules). The real init lives in a second,
>   **zstd-compressed** segment that starts after segment 1's `TRAILER!!!` and its zero padding. To
>   inspect it: `tail -c +<offset> img | zstd -dc | cpio -t`. Anchoring greps on `^init$` or
>   `^hooks/archiso$` against segment 1 alone returns 0 and looks alarming but proves nothing.
>   Note `lsinitcpio` is **not installed** in the `zohara-builder` image.
> - **There are three delivery channels for `airootfs/`, and each needs its own guard.** (1) the ISO —
>   mkarchiso deletes `/root/customize_airootfs.sh` from the image after running it; (2) the GitHub OTA
>   package — `build-update.yml` `rm -f`s both `/root/` and `/usr/local/bin/` copies from `$pkgdir`;
>   (3) the local self-extracting bundle from `create_update_bundle.sh`, whose embedded
>   `install_update.sh` does `cp -a "$DIR/airootfs/." /` and strips only the two legacy
>   `/usr/local/bin/zohara-{settings,store}` binaries. Channel 3 had no guard, so the 2026.08.23 bundle
>   installed `/root/customize_airootfs.sh` (9249 bytes, 0755) onto users' root filesystems — a script
>   that rewrites `/etc/os-release`, appends a duplicate `[zohara]` block to `/etc/pacman.conf` every
>   run, `pacman -Rdd`s discover/packagekit-qt6 and overwrites the SDDM config. Fixed 2026-08-24 by
>   stripping it at bundle-staging time. **When adding anything build-only to the overlay, check all
>   three.**
> - **The OTA bundle is a self-extracting script with a gzip tar payload after `__ARCHIVE_BELOW__`
>   (line 10).** `grep`ping the `.sh` for a path returns 0 whether or not the file is in there — it is
>   compressed. Inspect it with
>   `tail -n +11 zohara-update-*.sh | tar -tzf -`; entries are `./`-prefixed.
> - **`install_update.sh` does `cp -a airootfs/. /`, and `airootfs/` is a *live ISO* overlay** — so
>   before 2026-08-24 an update copied archiso's live scaffolding onto a real machine. Verified in the
>   2026.08.23 bundle: `etc/passwd` (`root:x:0:0:root:/root:/usr/bin/zsh`) **deletes every user
>   account**, `etc/shadow` (`root::14871::::::`) **leaves root with an empty password**,
>   `getty@tty1.service.d/autologin.conf` + `sddm.conf.d/99-autologin.conf` (`User=root`) **autologin
>   root**, `mkinitcpio.conf.d/archiso.conf` makes the next kernel update build a live-media initramfs
>   (**machine stops booting**), `journald.conf.d/volatile-storage.conf` moves the journal to tmpfs, and
>   `pacman-init.service` — which `install_update.sh` step 4 then `systemctl enable --now`s —
>   reinitializes the user's pacman keyring. Fixed by `scripts/ota-exclude.txt`, applied by **both**
>   OTA channels; each aborts if `etc/passwd`, `etc/shadow`, `root/customize_airootfs.sh` or the getty
>   drop-in survives stripping. Validated: 113 payload files → 64, 49 stripped, Zohara payload intact.
>   The list is derived mechanically — `comm -12` of the overlay against
>   `/usr/share/archiso/configs/releng/airootfs/` — not by judgement. **The ISO is unaffected; these
>   files are correct there.**
> - **`cp -a` writes *through* an existing symlink — when the *source* is a regular file.** Verified
>   with GNU coreutils 9.7: copying a regular file over `/etc/localtime -> /usr/share/zoneinfo/Asia/
>   Karachi` left the symlink intact and overwrote **the zoneinfo database file**, so every program
>   resolving that zone silently got the overlay's contents. A dangling destination instead gives
>   `cp: not writing through dangling symlink` and exit 1, which under `set -e` aborts
>   `install_update.sh` part-way through. This is why a *regular file* where a symlink belongs is worse
>   than a mere config leak. When the source is itself a symlink, `cp -a` (which implies `-d`) replaces
>   the destination link and does **not** write through.
> - **CORRECTION (2026-08-24): `airootfs/etc/localtime` is a *symlink* in the repo**, mode `120000`,
>   blob = the 23-byte string `/usr/share/zoneinfo/UTC` — identical to upstream archiso. Two earlier
>   revisions of this file were wrong about it: first that it baked the build machine's timezone, then
>   that it was a 114-byte regular file byte-identical to `/usr/share/zoneinfo/UTC`. The 114-byte file
>   was the *resolved* copy inside the built image, not the repo entry. Consequences: the write-through
>   hazard above never applied to it on a clean Linux clone, and **deleting it was the wrong fix — that
>   deletion was reverted.** It is correct on the ISO and `cp -a`-safe as a symlink. It stays in
>   `scripts/ota-exclude.txt` regardless, because `cp -a`-ing even a *symlink* to UTC onto an installed
>   machine replaces the user's `localtime -> …/Asia/Karachi` and resets their timezone.
> - **Upstream archiso `releng` ships `etc/localtime` and `etc/resolv.conf` — as *symlinks*** (to
>   `/usr/share/zoneinfo/UTC` and `/run/systemd/resolve/stub-resolv.conf`). An earlier revision of this
>   file said it ships neither; that was wrong. This repo tracks both as symlinks too; the problem is
>   only that Windows checkouts have no `SeCreateSymbolicLinkPrivilege`, so the *worktree* copy degrades
>   to a regular file. Hence the `tmpfiles.d` route for resolv.conf on the booted system.
> - **ROOT CAUSE of the regular-files-in-`.wants/` problem: only 18 of the repo's 42 committed symlinks
>   were `skip-worktree` pinned.** `scripts/build-iso-wsl.sh` excludes only the pinned set from its
>   rsync overlay, so the other 24 (22 of which Windows had materialized as regular files holding the
>   link target as text) were copied straight over the Linux clone's real symlinks. That is the
>   mechanism behind every "regular file in `.wants/`" symptom below, including `systemd-networkd`
>   shipping enabled. Fixed 2026-08-24: all 42 are now pinned, and `build-iso-wsl.sh` **asserts** that
>   every `120000` entry in `HEAD` is pinned, failing the build with the exact repair command if not.
> - **`systemctl disable` in `customize_airootfs.sh` is a silent no-op, and always was.** It only
>   removes *symlinks* from `.wants/` directories, but this profile is checked out on Windows, so git
>   materializes archiso's `.wants/` entries as **regular files holding the unit text**. Verified in the
>   2026-08-23 image: of the 20 entries in `/etc/systemd/system/multi-user.target.wants/`, **8 are
>   regular files** (`ModemManager`, `NetworkManager`, `bluetooth`, `choose-mirror`, `pacman-init`,
>   `power-profiles-daemon`, `systemd-networkd`, `systemd-resolved`) and 12 are symlinks; the
>   `systemctl disable systemd-networkd.service` produced **no `Removed …` output at all** (build log
>   line 3196ff). A regular unit file in a `.wants/` dir still creates the dependency — only the
>   filename matters there — so **`systemd-networkd` shipped enabled alongside NetworkManager**. The
>   same mechanism explains the harmless `Failed to enable unit: File … already exists` lines for
>   NetworkManager and bluetooth. Fixed by `rm -f`ing the four networkd enablement paths, which works
>   whichever form the entry takes. Confirming networkd actually *starts* needs a booted ISO; the
>   static evidence above is what has been checked.
> - **VERIFIED IN THE 2026.08.24 ARTIFACTS (both fixes landed).** Inspected with
>   `unsquashfs -ll` on `airootfs.sfs` from each ISO, and by unpacking each bundle's gzip payload:
>
>   | check | 2026.08.23 | 2026.08.24 |
>   |---|---|---|
>   | regular files in `multi-user.target.wants/` | 8 | 2 |
>   | `multi-user.target.wants/systemd-networkd.service` | regular file, 2428 B | **absent** |
>   | `sockets.target.wants/systemd-networkd.socket` | regular file, 682 B | **absent** |
>   | `network-online.target.wants/systemd-networkd-wait-online.service` | regular file, 785 B | **absent** |
>   | `etc/localtime` | regular file, 114 B | **symlink → `/usr/share/zoneinfo/UTC`** |
>   | OTA bundle payload entries | 220 | **154** (66 stripped) |
>   | `etc/passwd`, `etc/shadow` in the bundle | present (`root::14871::::::`) | **absent** |
>   | getty + SDDM autologin drop-ins in the bundle | present | **absent** |
>   | `root/customize_airootfs.sh` in the bundle | present | **absent** |
>
>   `display-manager.service → /usr/lib/systemd/system/sddm.service` and all 11 spot-checked Zohara
>   payload paths survive in both. `systemd-resolved.service` is still enabled in the new ISO, now as a
>   proper symlink — that one is *wanted* (`/etc/resolv.conf` points at its stub), only networkd was not.
> - **The 2 remaining regular files in `multi-user.target.wants/` were Zohara-authored full unit copies,
>   not enablement stubs — FIXED 2026-08-24.** `bluetooth.service` (759 B, dated 2025-10-08) and
>   `power-profiles-daemon.service` (989 B), plus `bluetooth.target.wants/bluetooth.service`, were
>   verbatim copies of the vendor units. A real file at that path *shadows*
>   `/usr/lib/systemd/system/<unit>`, so a future `bluez` / `power-profiles-daemon` update would ship a
>   unit systemd never reads. Two consequences were verified in the 2026.08.24 image:
>   `systemctl enable bluetooth.service` in `customize_airootfs.sh` failed with "File already exists"
>   and only created the `dbus-org.bluez.service` alias, and `power-profiles-daemon` was enabled
>   **solely** by its overlay copy (no `systemctl enable` for it existed anywhere).
>
>   Fix: deleted all three from the overlay and moved enablement into `customize_airootfs.sh`, which
>   runs under `arch-chroot` on Linux and *can* create symlinks — the pattern this repo already used for
>   `zohara-sync.service` (which is why that one appears in the ISO as a symlink but not in the repo).
>   Added `systemctl enable power-profiles-daemon.service` and
>   `systemctl add-wants multi-user.target bluetooth.service`; the latter preserves the extra
>   boot-time bluetooth enablement the deleted file provided, so this cannot regress the bluetooth fix
>   it came from. All 33 remaining `.wants/`+`.requires/` entries in the index are now mode `120000`.
>   Static validation only — `bash -n` passes and no dangling references remain, but this has **not**
>   been through a build yet.
> - **`default.target -> graphical.target`** (systemd's own default; nothing in this repo overrides it or
>   calls `systemctl set-default`), and `graphical.target` carries `Wants=display-manager.service`, with
>   `/etc/systemd/system/display-manager.service -> /usr/lib/systemd/system/sddm.service` present in the
>   2026.08.24 squashfs. That is the complete static dependency chain for SDDM starting at boot. It is
>   not a substitute for booting the ISO, which still has not been done.
> - **`systemd-resolved` is enabled only via the overlay's `.wants/` entry, not by calamares.**
>   `etc/calamares/modules/shellprocess_services.conf` enables NetworkManager, sddm, bluetooth and
>   `fstrim.timer` — **not** resolved. Installed systems get it because calamares `unpackfs` copies the
>   squashfs wholesale and `multi-user.target.wants/systemd-resolved.service` comes along. Verified
>   present in the 2026-08-24 image. This matters because `tmpfiles.d/zohara-resolv.conf` points
>   `/etc/resolv.conf` at resolved's stub — if resolved were ever not enabled, that symlink would
>   dangle and DNS would fail completely. Do not strip that `.wants/` entry from the ISO.
> - **Nothing may live at `airootfs/usr/local/bin/customize_airootfs.sh`.** A stale Aug-5 copy of the
>   chroot script (Plymouth watermark + pixmap only — a strict subset of the real one) sat there,
>   invoked by nothing, and shipped `0755` into every user's `PATH` via *both* the ISO and the OTA
>   `zohara-system` package — `build-update.yml` stripped only the `/root/` copy. Removed 2026-08-24
>   along with its `profiledef.sh` `file_permissions` entry. mkarchiso deletes
>   `/root/customize_airootfs.sh` from the image itself, which is why only the duplicate leaked.
>   (A stale `file_permissions` path is only a `_msg_warning` in the current archiso, not fatal.)
> - **`iso_version="$(date +%Y.%m.%d)"` is evaluated inside the container, which runs in UTC.** The
>   host is UTC+5, so any build started before 05:00 local time stamps the *previous* day: a build
>   started 04:25 local logged `Build date: 2026-08-23T23:25+0000` and named its output
>   `zohara-os-2026.08.23-x86_64.iso`. A rebuild can therefore produce a file with the **same name** as
>   the prior ISO. `scripts/build-iso-wsl.sh` moves prior artifacts to `out/previous/` before building —
>   without that, a rebuild silently overwrites the old image and the "did an ISO appear?" success
>   check proves nothing.
> - **`docker run` exit codes do not propagate** in this WSL + `docker.io` 29.1.3 setup, so build
>   success must be judged by artifacts (`out/*.iso`), never by `$?`. `scripts/build-iso-wsl.sh` does
>   exactly this and prints `!! no ISO produced` when the artifact is missing.

---

## 5. Instructions for New AI Sessions

When opening a new session:
1. **Always reference this `context.md`** file for full architectural alignment.
2. **Settings modifications**: Make changes in `zohara-settings-rs/`, push to `Zohaib8090/zohara-settings.git`, and run `docker build` to include the latest release in the ISO.
3. **Store package additions / versioning**: Modify `apps.json`, push to `Zohaib8090/zohara-packages.git`.
