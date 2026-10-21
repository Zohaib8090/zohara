# Release test of the Plasma 6.8 ISO (2026-10-22)

ISO built locally (`ZOHARA_ARCH_DATE=2026/10/20`, commit `df9d198`), `out/zohara-os-x86_64.iso`, SHA-256 `16d817f9a6381b351df55f8bcdd17e1ad7715063c291ab2f6454fe8f0e9dcd94`, 4,158,685,184 bytes.
Contents checked inside the image: Plasma 6.8.0, kernel 7.2.11-zen, Qt 6.12.0-3, `kwin-x11` absent, Zohara's own four theme packages (and the Fluent/Nordzy files), installer branding with the light side panel, mirror pinned 2026/10/20, `latte-dock-ng` still present.
All tests in headless VMs on this laptop (`~/zohara-vm/t-new`, `t-old`); throwaway account.

## A. Fresh install (blank 60 GB disk), "Swap (no Hibernate)", Erase disk
| Step | Result |
|---|---|
| Live boot, Welcome screen | OK (after the splash fades). In the owner's windowed VM the splash animation was drawn over the Welcome window; not reproduced headless |
| Installer panel | **white side panel, all steps readable, current step blue** (fixed, was black with dark text) |
| Location, Keyboard, Partitions (swap choice shown), Users (computer name defaults to `zohara`), Summary, Install | OK, install about 4 minutes, "All done" |
| First boot from disk | sign-in screen, then Plasma 6.8.0 Wayland desktop and the first-run Welcome |
| Health check | no failed system/user units, no coredumps, no journal errors (VM noise excluded); `systemd-networkd` masked, NetworkManager active; zram + swap partition; hostname `zohara`, `LANG=en_US.UTF-8` only; **no `resume=`, block file present, `CanHibernate` no, Hibernate greyed out on the sign-in screen**; the four theme files owned by `zohara-*` packages; no Chaotic `-git` theme packages |
| Settings > Display > **X11 support** | shown ("Plasma 6.8 has no X11 session...") |
| Add an X11 desktop > **Xfce** | password prompt, **added in under 20 s**, row says "Ready on the sign-in screen: Xfce" |
| Sign in with **Xfce Session** | **desktop up, `XDG_SESSION_TYPE=x11`, Xfce 4.20.4** |

## B. Update of an OLD install (09/26 ISO: Plasma 6.7.5, `kwin-x11`, Chaotic themes) to the new state
Run with the steps Settings' Update runs, but by hand: Settings' real Update accepts only a date signed with the owner's key, which was not used.
* Moved the machine to the **alpha** channel (the new theme packages are only there), date 2026/10/20.
* The X11 step: `kwin-x11` was no longer in the repositories, so it was removed first (`pacman -Rns`), then `pacman -Syu`: **exit 0, 15 minutes**.
* Result: Plasma 6.8.0, kernel 7.2.11, Qt 6.12.0-3, Settings `...210920` (alpha), **the four Chaotic theme packages replaced by Zohara's own** (same files, same names), `pacman -Dk` clean, snapper pre/post snapshots.
* After a real reboot: sign-in screen, Plasma 6.8.0 session, look preserved (Fluent-dark icons, kvantum-dark, BreezeDark), no failed units, no coredumps, no-hibernate state kept.

## Not good, or not known
1. **Plasma 6.8's sign-in screen shows its on-screen keyboard** over the login box in the VM (the VM has a tablet-type pointer; the 6.7 greeter did not do this). Typing still works, but clicking the session menu or the box while it is up misfires; it cost wrong passwords and a 5-minute lock-out in tests. Cause: `/etc/sddm.conf.d/20-zohara-keyboard.conf` sets `InputMethod=qtvirtualkeyboard`. Unknown on a real laptop; may only appear with a touch screen. A fix would be dropping that file (no touch keyboard at sign-in) or testing on the laptop.
2. The **live-session Welcome window had the splash animation over it** in the owner's windowed VM (3D graphics). Not reproduced headless.
3. **Xfce looks plain** (Adwaita, elementary icons): none of Zohara's themes are applied to it.
4. `latte-dock-ng` is still on the ISO although nothing uses it (owner has not decided).
5. The window decoration in these test machines is Breeze (the owner's laptop uses the Zohara round buttons because it was set there); not a regression.

## Not tested on this ISO
"Swap (with Hibernate)" install and the Hibernate "Set up" button on 6.8 (tested on 6.7), Hibernate itself (still unproven), Cinnamon on this build (tested on the earlier one), the cube / live wallpaper / dynamic desktops animations, round buttons, night light colour, Wine and Windows programs, Waydroid, Zohara Link, NVIDIA on real hardware, a real laptop, the Store, the Mac checker on a real Mac app, the real signed Update button.

## Release facts that matter
* **Stable Settings (`9f076c8`) does not have the X11 button or the "remove `kwin-x11` first" update step.** If the approved date moves to 2026/10/20 before a Settings that has them is on stable, machines on stable hit the `kwin-x11` problem. Those two changes sit in `e871787`, which is built on top of the Mac checker (`0c6863d`); keeping the Mac checker out means a separate build of Settings without it.
* The four theme packages are on **alpha only**; stable machines keep the Chaotic ones until they are promoted too.
* Moving machines to Plasma 6.8 is not a package promotion: it is the signed approved date, which only the owner can sign.
