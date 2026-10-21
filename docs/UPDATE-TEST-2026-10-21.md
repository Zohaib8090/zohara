# Update test in the VM: Arch date 2026/09/26 -> 2026/10/14 (2026-10-21)

Question: if the owner approves **2026/10/14**, does a machine installed from today's ISO (frozen at 2026/09/26) update cleanly and still work? Tested in the VM (`~/zohara-vm/upd-test`, a throwaway copy-on-write disk), as root with `pacman -Syu` after changing the date in `/etc/pacman.d/mirrorlist` (what Settings' Update does after an approved date; the signed manifest itself was **not** used, since the minisign key is the owner's).

## Result: it works
| Check | Result |
|---|---|
| Update run | `pacman -Syu`: exit 0, **about 11 minutes** (download), 1030 -> 1035 packages |
| Roll-back | snapper made a **pre** and a **post** snapshot (2 and 3) automatically |
| Versions after | `linux-zen` 7.2.7 -> **7.2.9**, `qt6-base` 6.11.2 -> **6.12.0**, `kwin`/`plasma-workspace` 6.7.5-1 -> 6.7.5-3, `libadwaita` 1.9.4 -> **1.10.0**, `gtk4` 4.22.5 -> **4.24.1**, `mesa` 26.2.3 -> 26.2.4, `glibc`, `mkinitcpio` 42 -> 42.2 |
| `.pacnew` files | none |
| Boot image / GRUB | rebuilt by the update; `resume=` kept |
| After a real reboot | sign-in screen (Qt 6.12) fine; **Wayland** session: Plasma 6.7.5 on kernel 7.2.9; **no failed system or user units**; no coredumps; `systemd-networkd` masked, NetworkManager active; zram + swap partition on; `CanHibernate` yes |
| Settings (stable build `...210524`) | opens on Qt 6.12 / libadwaita 1.10; Home, Display, Apps, Gaming pages render; "Up to date" on Home; the update itself pulled the stable Settings build |
| X11 | see below |

Journal errors this boot: only VM noise (`TDX not supported`, one `Pageflip timed out` of the emulated `bochs-drm` video card).

## X11 session
* A fresh ISO install has `kwin-x11` but **not `plasma-x11-session`**, so the sign-in screen has **no X11 choice at all** today.
* `sudo pacman -S plasma-x11-session` (exists at 09/26 and 10/14; tiny; made its own snapshot) adds a **Desktop Session** picker to the sign-in screen ("Plasma (Wayland)" / "Plasma (X11)").
* Signing in with **Plasma (X11)** on the updated system worked: `XDG_SESSION_TYPE=x11`, `kwin_x11` running, "Window Manager: KWin (X11)", Plasma 6.7.5; Settings opens in it (its Display page shows the X11 controls: no brightness or colour profile, which kscreen does not offer on X11).

## Not tested
Cube effect, live wallpaper, dynamic desktops, the `zohara` window decoration and round buttons, night light, `latte-dock-ng` (not in this install), `zohara-terminal` / `zohara-voice` / `zohara-snapshots` (not in this install), Wine / Windows programs, Waydroid, NVIDIA (no GPU in the VM), Hibernate itself on the new kernel, a real machine, the Update button with the signed manifest, the CI container's Arch date (still to check), a 10/14 ISO build.

## Lessons from running it
* `loginctl terminate-user` to log out makes SDDM show a black screen ("Process crashed" in the journal) until `sudo systemctl restart sddm`. It is how the test logged out, not an update problem; a normal Plasma logout is fine.
* The Hermes file hub (`~/zohara-vm/hub.py`) dies when the laptop restarts: start it again before using the VM scripts.

## Suggested next steps
1. Add `plasma-x11-session` to `zohara-profile/packages.x86_64` (next ISO) and offer it as a one-click "X11 session" in Settings for existing machines (needs the owner's decision; see chat).
2. Test the not-tested list above on a 10/14 image, especially the KWin effects and `latte-dock-ng`.
3. Check the CI container's Arch date.
4. Then the owner approves `2026/10/14`.

## Update past 10/14 and keep X11: the hold test (2026-10-21)
Goal: keep updating the rest of the system while Plasma stays at 6.7.5 (so the X11 session stays). From the updated 10/14 VM, the date was moved to **2026/10/20** and the update run with `pacman -Syu --ignore <the 39 Plasma packages>`. Settings' updater already does exactly this for `held_packages` in the signed manifest (`src/sysupdate/updates.rs`: `--ignore a,b,c`), so no code change is needed.

**The 39 packages (every installed package that becomes 6.8.0 after 10/14):**
`aurorae breeze breeze-cursors breeze-gtk kactivitymanagerd kde-cli-tools kdecoration kdeplasma-addons kglobalacceld kmenuedit knighttime kpipewire kscreen kscreenlocker ksystemstats kwallet-pam kwayland kwin layer-shell-qt libkscreen libksysguard libplasma milou ocean-sound-theme plasma-activities plasma-activities-stats plasma-desktop plasma-integration plasma-keyboard plasma-nm plasma-pa plasma-systemmonitor plasma-workspace plasma5support polkit-kde-agent powerdevil qqc2-breeze-style systemsettings xdg-desktop-portal-kde`
(`kwin-x11` and `plasma-x11-session` are not in Arch after 10/14, so they simply stay.) The other **92** installed packages changed between 10/14 and 10/20 normally: kernel, Qt patch, mesa, curl, polkit 128, systemd 262.1, NetworkManager, flatpak, libreoffice, ...

**Result: it works.** pacman resolved all dependencies with the 39 held (70 packages to upgrade, no conflict). The first run failed only on a slow archive download ("Operation too slow"); the second finished with exit 0. After a reboot: `linux-zen` **7.2.11**, `qt6-base` 6.12.0-3, `curl` 8.23, `polkit` 128, `mesa` 26.2.5; `kwin`, `kwin-x11`, `plasma-workspace` still **6.7.5-3**; `pacman -Dk` clean; the sign-in screen remembered "Plasma (X11)"; signed in with **session x11**, Plasma 6.7.5; no failed units; no journal errors but the VM's TDX line; `CanHibernate` yes.

**How long this can last (not tested, judged from the facts):** Plasma 6.7 is held while Qt stays on 6.12.x (a patch update like 6.12.0-2 -> -3 was fine). It will break when Arch moves Qt to the next minor (6.13) or KDE Frameworks change in a way Plasma 6.7 cannot use, so the hold needs re-testing at each approved date, and it ends at the latest in early 2027 when upstream stops 6.7. Security fixes for the held Plasma packages themselves stop at 6.7.5-3. Newly added Arch packages that need Plasma 6.8 would stop the update (pacman says so, nothing is half-installed).

**Crash found and fixed on the way:** `zohara-settings` aborted once in the VM. Cause: my test script ran `zohara-settings --version 2>&1 | head -1`; when `head` closed the pipe, the logger's `eprint!` panicked, and the panic hook logged again (double panic = abort). Fixed in `backend/diag.rs` (`964c0e7`: ignore a closed stderr); on alpha. Not an update problem. Note `--version` opens the GUI.
