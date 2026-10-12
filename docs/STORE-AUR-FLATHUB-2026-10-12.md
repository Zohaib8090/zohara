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

## Links and files from outside (added 2026-10-12): Flathub "Install" buttons, GitHub release downloads, AUR list

The owner asked for the browser buttons that "ask to open a helper" (Flathub-style *Install*, and apps offered as downloads on
GitHub) to open the Store, and for a list of packages from the AUR.

* **One Store window.** The Store now handles its command line (`HANDLES_COMMAND_LINE`), so a second launch hands its
  arguments to the open window. Code: `requests.rs` (pure parsing, tested), `ui::handle_request`.
* **`appstream://org.example.App`** (the link behind Flathub / app-store *Install* buttons) opens that app's page in the
  Store, waiting for the catalog if it is still loading; unknown ids become a search. The desktop entry
  (`zohara-store.desktop`) now has `Exec=zohara-store %U` and `MimeType=x-scheme-handler/appstream; application/vnd.flatpak.ref;
  application/vnd.appimage; application/x-iso9660-appimage`. No other program on the ISO or the laptop claimed these
  (Discover is not installed).
* **`.flatpakref` files** (Flathub's other install button): the app opens in the Store; a file from a source that is not
  Flathub is refused with the reason.
* **AppImages** (what many GitHub releases offer): opening one shows a warning (a program from the internet, not checked by
  Zohara) with Cancel as the default; Install copies it to `~/.local/share/zohara-store/appimages`, reads its name,
  description, category and icon from **inside** the file with `unsquashfs` (the file is never run for that; links inside it
  are followed only if they stay inside it) and adds a launcher `zohara-appimage-<id>.desktop`. Without `unsquashfs` the name
  comes from the file name and the icon is generic. No administrator password. Remove is in the Library. Only type 2
  AppImages are supported. The Store package now depends on `squashfs-tools` and `fuse2` (an AppImage needs FUSE 2 to run),
  and both are in `packages.x86_64`.
* **`.deb` files** are explained ("made for Debian and Ubuntu") with a button that searches the Store for the same app.
  Installing them directly is not supported.
* **Library** now has an **Installed from the AUR** group (packages `pacman -Qm` calls foreign that have a menu launcher;
  on the owner's laptop there are none, only `zohara-*` without launchers) and an **AppImages** group.
* **GitHub's "Open with GitHub Desktop"** (`x-github-client://`) was not done: it belongs to GitHub Desktop, not to the Store.

Verified (2026-10-12): 50 unit tests plus live tests: a real 15 MB AppImage (`appimagetool`) gives its real name,
description and icon without being run; `gio open appstream://com.github.tchx84.Flatseal` started the Store on Flatseal's page
with a Get button; opening a `.flatpakref` while the Store ran reached the same process (no second one); opening the AppImage
showed the warning, **Install** created the launcher (real name, description, category `Development`, a 128x128 icon), the
installed file ran (`--version`), the launcher passed `desktop-file-validate`, and the Library listed it under AppImages.
**Not verified:** a real browser clicking the buttons (the test used `gio open`, which follows the same default-handler
settings); Remove of an AppImage by clicking (the code path is unit-tested; the click test was stopped, see below); the new
Library group for AUR packages with a real AUR app; Flathub's Install button on a real web page; a live ISO.

An incident worth knowing: during the click test a click from an old screenshot landed on **Discord's Get** button and the
test copy started `pkexec pacman -S discord`. It only waited for a password; I killed it at once, nothing was installed, no
pacman lock was left (see GOTCHA).
