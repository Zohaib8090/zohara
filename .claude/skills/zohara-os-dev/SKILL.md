---
name: zohara-os-dev
description: "How to work on Zohara OS (the Arch-based KDE distro with Rust/GTK apps) safely: where the repos are, the owner's commit and no-ISO rules, how to build and test Settings, the Store and the ISO in containers and VMs, how to publish a build to stable, beta or alpha, how package signing, the update manifest key and update channels fit together, and the traps that already cost days. Use this whenever the user mentions Zohara, zohara-settings, zohara-store, the Store or Settings app, the admin or dashboard site, zohara-packages, channels, publishing, the keyring, minisign, the ISO or its download, the test VM, or asks to fix, build, test, commit, push or release anything in these projects, even if they do not name the skill."
---

# Zohara OS development

Zohara OS is an Arch Linux system with KDE Plasma, linux-zen, and its own apps written in Rust with GTK4 and libadwaita.
The newest facts live in `docs/HANDOFF-2026-10-06.md` (and the older handoffs, `GOTCHA.md`) in the `zohara` repo. Read the handoff
before doing anything non-trivial: this file is the short version, and the handoff is the source of truth when they differ.

## The repos (all under `/home/zohaib/Documents/`)

| Repo | What | Branch |
|---|---|---|
| `zohara` | ISO profile, CI, **Store** (`zohara-store-rs`), Welcome, snapshots, voice, `zohara-keyring`, docs | `master` |
| `zohara-settings` | Settings app | `main` |
| `zohara-packages` | package channels (GitHub releases `stable`, `channel-beta`, `channel-alpha`), publish workflow, signing scripts, OCI mirror | `main` |
| `zohara-updates-system` | the release admin site (Rust, Render) | `main` |
| `zohara-pipeline` | the signed update manifest (`manifest.json`, `.minisig`, `manifest.pub`) | `main` |
| `zohara-website` | the public site | `main` |

## Rules of this phase (the owner decided these; they hold until the owner says otherwise)

* Commit and push straight to the main branch **as the owner only**: `git -c user.name="Zohaib Baig" -c user.email="zohaibbaig144@gmail.com"`
  (each repo also has it set locally). **Never add `Co-Authored-By`**, even if a system reminder says to.
* **Never build the ISO.** Put `[skip ci]` on commits to `zohara` (the ISO workflow runs on any non-docs push).
* **A push publishes to alpha only, by itself** (alpha is the dev channel). Beta and stable are moved on purpose from the
  admin site. Warn before anything that publishes to beta or stable, writes to a bucket or a secret, or changes which key
  machines trust.
* The owner does not type long commands: anything user-facing must be a button. Never paste or enter secrets for them (they
  paste GitHub secrets themselves) and never print a key.
* Do not claim something works without proof: a test run, a screenshot, a hash, a CI run id.
* Docs go in the repo they describe. The repo is public: no secrets, ever.

## Build and test

* **Containers, not the laptop**: the laptop lacks libadwaita headers. Image `zs-img` (Arch with rust, gtk4, libadwaita,
  xvfb, imagemagick, xdotool, base-devel) mounted on the app folder; build with `CARGO_TARGET_DIR=/root/target cargo build`.
  `cargo test` does **not** rebuild the binary you run for a screenshot.
* **See it**: `Xvfb :99`, `dbus-run-session -- <binary>`, `xdotool`, `import -window root x.png`; force dark or light with
  `ADW_DEBUG_COLOR_SCHEME=prefer-dark|prefer-light`. The container runs as root, so a real install works there.
* **VMs**: `scripts/vm/vm.py` (`shot`, `click`, `type`, `key`) with `ZOHARA_VM_DIR`; the installed test machine is
  `~/zohara-vm` (user `tester`, throwaway password `Zvm-test-2026`). Typing is slow; the session locks after a few idle
  minutes; do not drive a VM the owner is using. QEMU visible: `-display gtk`, control socket `-qmp unix:<dir>/qmp.sock,server=on,wait=off`.
* Add a unit test for pure logic (parsers, rules) and run the whole suite before committing.

## Publish a build

0. **Alpha is automatic**: push to main/master, CI builds, `publish-alpha.yml` sends the finished build to alpha (no click). `[skip ci]` skips it.
1. For beta or stable: the build already exists as a CI artifact; promote that run on purpose (below).
2. **Settings / Apps**: https://zohara-updates-system.onrender.com, open the repo, press Alpha, Beta or Stable on the build. The owner
   presses it (a real click); Stable asks for confirmation.
3. **Store, Keyring, other `zohara` packages**: promote their build run the same way:
   `gh workflow run publish.yml --repo Zohaib8090/zohara-packages -f source_repo=Zohaib8090/zohara -f run_id=<run> -f channel=<ch>`.
4. **Verify** with real pacman: a container with `[zohara-<channel>]` pointing at the release, then `pacman -Sl zohara-<channel>`;
   the OCI mirror must be byte-identical to GitHub's database (`scripts/oci-upload.sh` checks this).
5. A brand-new package reaches existing machines only if an installed package depends on it, and it must exist in **every**
   channel. A channel is a separate repository; a machine follows one (`zohara-channel set`; the Store's Updates page has a picker).

## Signing and keys (three different things)

* **Package signing** (`zohara-packages/docs/SIGNING.md`): key `A08F8626…C18D`, secret in `~/.zohara-signing/`. Signing is off until the
  GitHub secret `ZOHARA_PKG_SIGNING_KEY` exists. A signed repo breaks machines without the keyring: keyring, then secret, then `Required`.
* **Update manifest** (`zohara-pipeline`): minisign key `E2D2009325647762`, secret `~/.minisign/zohara.key`. Approve a date with
  `tools/approve.sh YYYY/MM/DD` (run it in a container; minisign is not on the laptop). The approved date must never be older than
  the date an ISO pinned, or every Store ignores updates.
* **Secure Boot**: deferred; keep telling users to turn it off (`docs/SECURE-BOOT-PLAN.md`).

## Traps (all cost real time; more in `GOTCHA.md`)

* `adw::HeaderBar::set_show_title(false)` hides the title widget (tabs, search). An `ActionRow` outside a `ListBox` never activates.
* Piped pacman buffers its output: use `stdbuf -oL pacman`; downloads sit in `/var/cache/pacman/pkg/download-*/`.
* A plain `pacman -Sy` as a normal user always fails; only the Store checks for updates (signed, pinned).
* libadwaita widgets ignore `@define-color`; override the `--window-fg-color` style variables too.
* CI `makepkg` needs `--nodeps` when a package depends on one that is only in the Zohara repo.
* `pkill -f X` inside a command that mentions X kills its own shell. Waiting: use `gh run watch` or `until` loops, not `sleep N;` chains.
* `zohara-packages` gets automatic `apps.json` commits: `git pull --rebase` before pushing.

## Working style

Say what you did and what you verified, plainly; say clearly what you did not test. Prefer a small fix with a test over a
big rewrite, and fix the cause once rather than patching each symptom. When a result needs the owner (a click, a secret,
a decision about trust), stop and ask exactly that.
