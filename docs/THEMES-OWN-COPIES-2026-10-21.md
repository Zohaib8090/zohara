# Our own pinned copies of the look-and-feel packages (2026-10-21)

Owner's decision: stop relying on other people's rolling builds. Step 1 of the plan (mirror and pin; trim and our own look come later).

* New source folder `zohara-themes/` (PKGBUILD, one `pkgbase`, four packages) and workflow `build-themes.yml` (manual start; **not** wired to `publish-alpha.yml`, publish on purpose with `zohara-packages/publish.yml`).
  `zohara-fluent-icon-theme`, `zohara-fluent-cursor-theme`, `zohara-fluent-gtk-theme`, `zohara-nordzy-icon-theme`. Each `provides`/`conflicts` the upstream name and **`replaces`** the Chaotic `-git` package, so a machine swaps on its next update.
* Pinned upstream commits (what the owner's laptop had on 2026-10-21): Fluent-icon-theme `c8a244f`, Fluent-gtk-theme `7a49a46`, Nordzy-icon `829c47b`. GPL-3.0. They change only when someone edits the commit in the PKGBUILD and tests it. The GTK package fails the build if `sassc` was missing (the installer only warns).
* Not packaged: the GTK theme's optional wallpapers (`usr/share/backgrounds`, 23 files): nothing in Zohara uses them.
* **Verified:** CI run `39626994570` (45 min) vs the packages installed on the laptop: file lists identical (icons 97,700, cursors 231, Nordzy 702,471; GTK 61,935 vs 61,958 = only the wallpapers) and 300 random files plus the generated style sheets byte-identical. Published to **alpha** (publish run `39635265169`); the alpha database lists the four with `%REPLACES%`.
* ISO: `Dockerfile` now downloads the four packages from the alpha channel into the local repo, and `packages.x86_64` names them (`89f7c58`). **Not tested**: Docker was off, no ISO built with it yet; first test is the next ISO build.
* Not tested: an Update on a machine that has the Chaotic packages (the swap through `replaces`), Plasma/GTK apps showing the themes after the swap.
* Still on Chaotic-AUR: `brave-origin-bin`, `yay`, `debtap`, `claude-desktop`, `google-chrome`, `visual-studio-code-bin`, `ente-auth`, `protonup-qt`, `latte-dock-ng` (unused, breaks off-date), python-inputs/python-steam, chaotic-keyring/mirrorlist. Next: decide which to keep, mirror or drop.
* Next steps: trim the variants we do not offer (Nordzy 743 MiB and Fluent GTK 179 MiB installed), our own Kvantum theme/cursor/accents, our own icons only much later.
