# VM test checklist

CI proves the code compiles. It does not prove the Plasma, CUPS or D-Bus calls
behave on a real install. Boot the ISO in a VM and go through this list.
Note what fails, plus the log from `~/.local/state/zohara/settings.log`.

## Setup

- VM with UEFI, 4 GB RAM, 30 GB disk, a virtual microphone/speaker, and a
  **second virtual disk** (used for the migration test).
- Boot the live ISO, run the installer, reboot into the installed system.
  (Checks: welcome app on the live USB, Calamares launches from it.)
- Run `zohara-settings --health-check` once. It should print nothing alarming.

## Settings pages (installed system)

| Page | Try | Expect |
|---|---|---|
| Display | change scale, rotation, arrange two monitors | applied immediately; survives sign-out |
| Sound | play the left / right speaker test | tone from the correct side only (use headphones) |
| Sound | change default output, per-app volume | follows pactl |
| Bluetooth | toggle adapter, scan | list updates; pair a phone if the VM has a dongle |
| Network | speed test | shows down/up/ping and server location |
| Network | hotspot, proxy | proxy shows in `kreadconfig6 --file kioslaverc` |
| Keyboard | add a layout, key repeat | layout appears in the tray switcher |
| Shortcuts | rebind a shortcut, clear one | works in Plasma right away |
| Mouse / Touchpad | speed, scrolling | changes felt immediately |
| Notifications | Do Not Disturb | Plasma DND toggles |
| Power | screen off / sleep timers per AC/battery | `powerdevilrc` updated |
| Time & language | timezone, NTP | `timedatectl` agrees |
| Privacy | camera / mic / location, firewall | `ufw status`, mic muted in pactl |
| Privacy > Firewall rules | Load rules, add `8080` allow, block a range, delete a rule | `sudo ufw status numbered` matches; list reloads after each change |
| Privacy > Access indicators | open a camera app (e.g. `cheese`); record with a voice recorder | camera icon appears in the tray while open, gone after; Plasma's own mic icon shows (Zohara's is off by default); Recent activity lists both |
| Privacy > Access indicators | turn on the Microphone switch | a second, Zohara microphone icon appears while recording |
| Privacy > Right now | open the page while the camera is on | shows the app name within a few seconds |
| Storage | disks, cleanup, Storage Sense | sizes real; cleanup frees space |
| Accounts | add user, change password, autologin | new user can sign in |
| Apps | installed list, startup toggles, Flathub | uninstall works; startup switch persists |
| Apps > Websites as apps | add a site, startup, background | window opens as its own app; startup entry created |
| Apps > Offline maps | pick a region, download | files under `~/.var/app/app.organicmaps.desktop/data/Organic Maps/OMaps/` |
| Default apps | change browser, open a link | link opens in the chosen browser |
| Gaming | Game Mode, MangoHud, power mode, Vulkan check | overlay shows after re-login; `vulkaninfo` OK |
| Accessibility | high contrast, cursor size, visual bell, screen reader | applies at once |
| Troubleshoot | health list, save report | report saved; kill a service to see a fix offered |
| Zohara Update | check for updates | reaches the repo, lists updates |

## Voice typing

1. Settings > Accessibility > Voice typing is on; language set.
2. Open a text editor, click in it, press **Meta+H**.
3. Say a sentence. Expect "Listening…", then "Writing it down…", then the text in the editor.
4. Press Meta+H, wait, press Meta+H again: it should finish early.
5. Stay silent for 8 seconds: "Didn't hear anything".
6. `systemctl --user status ydotool` is active. If the text only lands on the
   clipboard, `ydotool` or `/dev/uinput` access is the problem.
7. Try Hindi or Urdu with the language set to match.

## Printers

- `systemctl status cups.socket avahi-daemon` are active.
- In the VM, add a CUPS-PDF or a network printer (a phone/printer on the same
  network) and check it shows under "Printers on your network".
- Add by address with the printer's IP. Set default, change paper size, print a
  test page, pause and resume, cancel a queued document, remove.

## Migration tool

Prepare the second disk: create an ext4 partition, install a small Debian or
Ubuntu (or just copy a fake root: `etc/`, `usr/`, `var/lib/dpkg/status`, and a
`home/<user>/` with files, symlinks, and a file whose name already exists in the
new `/home`).

1. Welcome app > Migrate. The disk should be listed; the system disk should not.
2. Start with "Remove the old system" **off**. Expect files in `/home`, the same
   names kept intact, clashes saved as `name (old system).ext`, symlinks still
   links, ownership correct (`ls -l`).
3. `sudo cat /var/log/zohara-migrate.log` lists the run.
4. Repeat on a fresh copy with "Remove the old system" **on**: old `/etc`, `/usr`
   and friends are gone afterwards, but only when nothing failed.
5. Press Stop mid-run: it stops after the current file and reports what's left.

## Welcome app

- Live: Install, Migrate, Try. Installed: Migrate, Manage users, Update, Settings.
- "Manage users" opens Settings on the Accounts page; "Update Zohara" on Zohara Update.

## Report

For anything that fails: page, what you clicked, what happened, and the last
lines of `~/.local/state/zohara/settings.log` (Troubleshoot > Save report does
this for you).
