# Store: the AUR and Flathub (2026-10-12)

The owner reported that the Store "doesn't get packages from the AUR" and, a moment later, "also Flathub". What was found
and what was done. Code: `zohara-store-rs/src/aur.rs`, `backend.rs`, `ui.rs`.

## What the Store searched before

* **pacman**: the official repositories **and Chaotic-AUR** (about 3,100 prebuilt AUR apps: Chrome, Spotify, VS Code, Zoom ...).
* **Flathub**, from Flatpak's local list, but only if the `flathub` remote existed.
* The real **AUR** (about 90,000 build-it-yourself packages) was never wired in (the guide called it "not yet implemented").

## AUR (owner's decision: add it, with a warning)

* A search of two letters or more still searches the local catalog. From three letters, a second search goes to
  `aur.archlinux.org/rpc/v5/search` (the AUR is too big to copy) and shows a separate **Community (AUR)** group under the
  results. It leaves out what the repositories already carry, packages with no maintainer, packages flagged out of date and
  library-like names, most popular first, at most 6.
* **Get** on an AUR package first shows a warning ("community-made, not reviewed by Zohara, runs the author's build script")
  with **Cancel** (the default), **View script** (the real PKGBUILD in a window; *Build and install* stays off until it has
  loaded) and **Build and install**. Removing needs no warning.
* The build is `yay -S --noconfirm --needed --removemake --cleanafter --sudo pkexec --answer... -- <name>` run as the person;
  the administrator password is asked through `pkexec` only for installing. `yay` is in `packages.x86_64` already (with
  `base-devel` and `git`); the Store package now has `curl` as a dependency and `yay` as an optional one. Package names are
  checked (`valid_name`) before they reach a URL or a command, and the name goes after `--`.
* A package built from the AUR is a normal installed package afterwards (`pacman -Q`). **Nothing updates it**: Settings
  updates the repositories and the Store updates Flathub, so AUR-built apps stay at the version they were built.
  Open item: an "AUR updates" check (`pacman -Qm` against the AUR) if the owner wants it.

## Flathub

* The installer adds the remote with `flatpak remote-add ... || true`; if that step runs offline it silently adds nothing, and
  the Store then lists no Flathub apps. The Store now checks at start (`ensure_flathub`) and adds Flathub for the current
  person (`--user`, no password) when it is missing. If it cannot (offline), or the list cannot be read, a line above the
  results says so and the catalog is read again every minute, five times.
* **Not fixed:** Flathub cards say "No description available", because `flatpak remote-ls` has no description column and the
  local appstream catalog is not downloaded. A description would need `flatpak update --appstream` or the Flathub web API.

## What was verified

* 36 unit tests run by default plus 3 live ones (9 AUR, 2 for Flathub setup and AUR error wording).
* Live, against the real servers: AUR search finds `yay`, `pick` keeps the list sane, the PKGBUILD downloads, bad names are
  refused; `ensure_flathub` adds Flathub in an empty throwaway Flatpak setup (the real remotes were not touched) and
  more than 500 apps are listed.
* Clicked in a test copy of the Store: a search for "ventoy" shows the normal result and the Community (AUR) group; **Get**
  on an AUR package opens the warning; **View script** shows the real PKGBUILD; Cancel closes it. A Flathub-only app
  (Flatseal) shows with its Flathub card.
* `yay` accepts every option in the install command (run on a package that does not exist: "No AUR package found").

**Not verified:** an actual AUR build and install through the Store (it needs the owner's password, which the owner types:
pick a small package, press Get, read the script, Build and install); the retry messages with the network off; a live ISO.
