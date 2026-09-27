# Zohara for phones (Termux + proot)

Zohara OS running on a 64-bit ARM Android phone, inside [Termux](https://termux.dev),
through [proot](https://proot-me.github.io) — no root, no bootloader unlock,
and the phone's own Android is untouched.

This is a **separate build** from the x86_64 ISO. It shares Zohara's apps
(Settings, Store, Welcome, Voice) but ships its own system image, its own
package repository, and its own release. See **[Branch model](#branch-model)**
below for exactly how the two stay apart.

## Contents

| Path | What |
|---|---|
| `build.sh` | Builds everything: runs on GitHub's ARM runners |
| `packages.txt` / `packages-optional.txt` | The phone's package list (Plasma, apps, fonts) |
| `zohara-phone/` | A pacman package: touch/speed tuning + the session script |
| `termux/` | What actually runs on the phone: the installer and `zohara` launcher |

## How someone installs it

In Termux, on a 64-bit ARM phone:

```bash
curl -fsSL https://github.com/Zohaib8090/zohara/releases/download/proot-latest/install.sh | bash
```

`termux/install.sh`:
1. Installs Termux's own prerequisites: `proot-distro`, `termux-x11-nightly`,
   `pulseaudio`, and (optionally) `virglrenderer-android` for GPU mode.
2. Downloads `zohara-rootfs-aarch64.tar.xz` (~870 MB) with resume support,
   and checks it against a published SHA-256 (`.sha256` file next to it).
3. Installs it with `proot-distro install --name zohara <archive>` —
   **proot-distro 5 has no plugin/definition-file system**; it only takes a
   Docker image reference or a local archive/OCI layout. (proot-distro 4 and
   older are supported as a fallback, using `termux/zohara.sh.in`, which is
   still built for that case.)
4. Installs the `zohara` command into Termux (`termux/zohara`).

The person then installs the Termux:X11 app (shows the desktop) and types
`zohara` to start it, or `zohara gpu` for GPU acceleration, or `zohara shell`
for a terminal inside Zohara without starting the desktop.

## What `build.sh` does

Runs as root on an aarch64 Linux machine (CI: `ubuntu-24.04-arm`). Two stages,
both using real Linux `chroot` (not proot — this is the build side):

**1. Build Zohara's packages for aarch64**, in a throwaway chroot with
`rust`/`cmake`/etc installed:
- `zohara-settings`, `zohara-store` (`zohara-store-rs`), `zohara-welcome`:
  built from source, same as the x86 build, `cargo test --release` run first.
- `zohara-voice` (whisper.cpp): built with NEON (ARM's equivalent of AVX2);
  see the `arch=('x86_64' 'aarch64')` branch in its `PKGBUILD`.
- `zohara-voice-model`: architecture-independent, unchanged.
- `zohara-phone`: this directory's own package (below).
- All five are collected into a pacman repository named `zohara-stable`
  (`repo-add`), published under both `zohara-stable.db` and `zohara.db` so
  older installs and newer ones both find it.

**2. Build the phone system**, in a second chroot:
- Extracts a fresh Arch Linux ARM.
- Removes the ARM board kernel/firmware (a phone has its own kernel; proot
  never boots one).
- Installs `packages.txt` (Plasma on X11, apps, fonts) plus whatever matches
  from `packages-optional.txt` (package names that move between Plasma
  releases — installed only if `pacman -Si` finds them, so a rename doesn't
  fail the whole build).
- Installs Zohara's own packages, **including `zohara-phone`** but
  **excluding `zohara-voice*`** — voice typing needs `/dev/uinput` to type
  into other apps, which proot cannot reach, and Settings hides that page on
  phones anyway (see zohara-settings' `docs/PHONE.md`).
- Adds `[zohara-stable]` to `pacman.conf`, pointed at the ARM release, so the
  phone's own Zohara Store can update these packages later.
- Packs the result as `zohara-rootfs-aarch64.tar.xz`, after force-unmounting
  everything under the chroot and confirming nothing is left mounted
  (`unmount_tree`/`mounts_under` in `build.sh`) and excluding `/proc` and
  `/sys` from the archive (`--one-file-system`) — both were real failures hit
  while building this; see the commit history on `arm-main` for the log
  output that diagnosed each one.

## The `zohara-phone` package

Everything phone-specific that isn't Settings/Store/Welcome/Voice lives here,
as its own pacman package (`zohara-phone/PKGBUILD`), so it can be updated
through Zohara Store without rebuilding the whole system image.

It **never overwrites a file another package owns.** Instead
`zohara-phone/files/usr/bin/zohara-session` (what `zohara`/`zohara shell`
runs) puts its own directories first on the search path:

```
XDG_CONFIG_DIRS = /usr/share/zohara-phone/xdg : /etc/xdg
XDG_DATA_DIRS   = /usr/share/zohara-phone/share : /usr/local/share : /usr/share
PATH            = /usr/lib/zohara-phone/bin : $PATH
```

What that carries:

| File | Effect |
|---|---|
| `xdg/kwinrc` | Compositing and effects off |
| `xdg/kwinrulesrc` | Every window opens full-screen |
| `xdg/kdeglobals` | Animations off |
| `xdg/baloofilerc` | File indexing off |
| `xdg/kwalletrc` | The wallet service off |
| `share/applications/systemsettings*.desktop` | The "Settings" menu entry launches `zohara-settings`, not KDE's own |
| `bin/systemsettings`, `bin/systemsettings6` | The *commands* also redirect to `zohara-settings`, in case anything invokes them directly |
| `bin/pkexec` | proot is already root and has no polkit daemon: runs the command directly instead of asking |
| `/etc/zohara-phone.conf` | `SCALE` (1–4, default 2), `QUICK` (`software`/`gl`), `EFFECTS` (`on`/`off`) — copy to `~/.config/zohara-phone.conf` to override per user |
| `/etc/zohara-proot` | The marker file Settings and the Store check for (see below) |

`zohara-session` also:
- Sets `QT_SCALE_FACTOR`/`GDK_SCALE` from `SCALE`, since a phone screen needs
  everything larger than a laptop does.
- Uses Qt's software renderer by default (`LIBGL_ALWAYS_SOFTWARE=1`,
  `GALLIUM_DRIVER=llvmpipe`) — proot has no GPU access, so this is the fastest
  correct option.
- With `ZOHARA_GPU=virgl` (set by `zohara gpu`), instead points at Termux's
  `virgl_test_server_android`, which forwards OpenGL to the phone's real GPU.
  Faster, but less tested/stable — off by default.
- Runs `/usr/lib/zohara-phone/tune` in the background once Plasma is up: a
  one-time (marker-file-guarded) D-Bus call that makes the panel finger-sized
  (52px), since Plasma has no first-class "phone panel" preset.

## How Settings and the Store know they're on a phone

Both check for `/etc/zohara-proot` (or `ZOHARA_PROOT=1` in the environment,
which `zohara-session` also sets, for before the package is installed):

- **zohara-settings**: `src/backend/platform.rs` — `is_phone()`,
  `PHONE_HIDDEN_PAGES`. Full list and reasoning in that repo's
  `docs/PHONE.md`.
- **zohara-store-rs**: `updates::is_phone()` in `src/updates.rs`. Hides
  Flatpak apps and Timeshift from the catalog (`app_info.rs`), hides restore
  points (no Btrfs/GRUB in proot), and never reports a restart as pending
  (`updates::needs_restart`, `restart_pending` — proot has no kernel of its
  own to restart into).
- The **signed update-manifest / pinned-Arch-date system** (`manifest.rs`)
  stays x86_64-only (`manifest::enabled()`): it pins to a dated snapshot of
  the Arch Linux Archive, which carries no ARM packages. Phones follow Arch
  Linux ARM directly instead.

## Branch model

**Every Zohara repo that ships to phones has an `arm-main` branch**, kept
separate from the branch that ships the x86 OS:

| | x86 OS | Phone OS |
|---|---|---|
| `zohara` | `master` | `arm-main` |
| `zohara-settings` | `main` | `arm-main` |

- `.github/workflows/build-arm.yml` in this repo triggers **only** on pushes
  to `arm-main` (or a manual run) — never `master`.
- It always clones `zohara-settings` at *its* `arm-main`
  (`SETTINGS_BRANCH=arm-main` in the workflow; see `build.sh`'s
  `SETTINGS_REPO`/`SETTINGS_BRANCH`).
- Nothing on `arm-main` is merged into `master`/`main` automatically. A fix
  made on one side (e.g. a Store bug fixed on `master`) needs a deliberate
  merge or cherry-pick into `arm-main` to reach phones, and vice versa. There
  is currently no automation for that — it's a manual step, on purpose, so
  each side's build is only ever changed by an explicit push to its branch.

## Known gaps / not yet tested on a real phone

- Nothing here has booted on a real device yet. The build succeeds in CI
  (packages compile and pass tests on real aarch64 hardware; the system image
  packs cleanly), but whether Plasma actually starts under Termux:X11, at what
  speed, and whether `tune`'s panel-resize script works, is unverified.
- `packages-optional.txt` covers the X11-session package rename risk, but if
  Arch Linux ARM's Plasma packaging changes more than that, `zohara-session`
  will report "The Plasma X11 session isn't installed" and exit.
- The `zohara.sh.in` / `zohara.sh` proot-distro **plugin** path (for
  proot-distro ≤ 4) is untested; only the archive-install path (proot-distro
  5) has been exercised, and only in CI's asset-checksum step, not by an
  actual `proot-distro install`.
- Touch input (tap-to-click via Termux:X11) has not been tuned yet — this doc
  will be updated once that's built.
