# Running macOS and iOS apps on Zohara: findings and plan (2026-10-19)

Status: **research and plan only. Nothing is built or installed yet.** Owner's goal: "run anything you throw at it", including macOS and iOS apps.
This page records what was checked, what is realistic, and the order to build it. It sits next to `docs/HANDOFF-2026-10-18.md` (Windows programs,
Zohara Link) and the Windows notes in `zohara-settings/docs/WINDOWS-GAMES.md`.

## 1. The honest answer in one table

| Kind of app | Can Zohara run it? | How | State |
|---|---|---|---|
| Linux apps (pacman, Flatpak, AUR) | yes | native | done |
| Windows `.exe` `.bat` `.msi` | yes, mostly | Wine / Proton-GE, per-game folders, Safety mode, log doctor | done (see WINDOWS-GAMES.md); PowerShell installers still fail (section 6) |
| Java (`.jar`, Minecraft Java) | yes | Java management in Settings | done |
| Android apps (`.apk`) | yes | Waydroid (installed on the owner's laptop for tests) | not wired to double-click; not checked |
| **macOS apps (`.app`, `.dmg`)** | **partly, to be built** | Darling (macOS translation layer) + a checker + double-click handler | this plan |
| **iOS apps (`.ipa`)** | **no** | no open UIKit/Metal; apps are encrypted | explain and point to an alternative |
| Programs in archives (`.tar.gz`, `.tar.xz`, `.tar.bz2`, `.tar.zst`) | yes, for ready-to-run apps | Settings > Apps > Apps from archives, and right-click > Zohara > Install as an app; source code is pointed to the Store | built 2026-10-20 (section 8) |
| Kernel-level anti-cheat games | no | needs the real Windows kernel | explain |

"Anything" cannot be literal. The goal for the rest is a clear, plain reason and the best alternative, never a silent failure.

## 2. Why macOS is hard (checked 2026-10-19)

REA (the reverse-engineering tools) opened a real macOS binary and a short script read its load commands. Sample: `libgdk-quartz-2.0.0.dylib`
(from the Mono for Mac folder inside the owner's `~/Downloads/Cities-Skylines`; 32-bit Intel Mach-O). It needs:

* Apple's closed frameworks: `Foundation`, `Cocoa`, `AppKit`, `Carbon`, `CoreFoundation`, `CoreServices`, `ApplicationServices`;
* Apple's runtime and C library: `libobjc.A.dylib`, `libSystem.B.dylib` (macOS system calls differ from Linux's kernel);
* ordinary open libraries (`glib`, `cairo`, `pango`, `libpng`, ...) shipped beside it: these are not the problem.

So an app is only as portable as its use of Apple's frameworks. The owner's `Cities.app` is **not** a program: it is a data folder (80 MB of radio audio, no `Contents/MacOS`, no `Info.plist`);
the folder around it is the Windows build of the game. Nothing in `~/Downloads` is a runnable macOS app.

REA limits found: `inspect_macho` and the native macOS tools need **macOS**; the other back-ends (Ghidra, Hopper, IDA) are **not installed** on this laptop
(`GHIDRA_INSTALL_DIR` unset, `/opt/hopper` missing, no IDA config). For Mach-O on Linux use a small reader of our own (the script used here parsed `LC_LOAD_DYLIB` in about 30 lines of Python)
or install Ghidra 12.1.x and set `GHIDRA_INSTALL_DIR`.

## 3. What exists: Darling

Darling is the open-source Darwin/macOS translation layer for Linux (like Wine for Windows). Checked on the AUR on 2026-10-19:

* `darling-bin` 0.1.20260608 (updated 2026-06-11), built from the project's release; depends only on ordinary libraries (`cairo dbus ffmpeg freetype2 fuse2 giflib glibc glu libgl libglvnd libjpeg-turbo libpng libpulse libtiff libx11 libxcursor libxext libxkbfile libxrandr`), **no kernel-module package listed**, so it should suit the linux-zen kernel.
* `darling-*-git` split packages (core, cli, gui stubs, extra) updated 2026-08-13; `darling-dmg-git` (mounts `.dmg`) is old (2022).
* License GPL-3.0. Package is `x86_64` only.

**Not tested**: nothing was installed or run. What Darling really runs (command-line tools well, graphical apps partly) is from general knowledge, not from a test here. First job of the plan is to measure it.

## 4. Where REA fits, and the legal line

REA and our own Mach-O reader are for **observing what apps ask for**, not for copying Apple's code.

* Allowed and useful: which frameworks and functions an app imports; which of them Darling already provides; a ranked "most wanted" list for what to rebuild.
* **Not allowed**: decompiling Apple's frameworks and reusing the code. Missing pieces are rebuilt from Apple's public documentation and open-source Darwin, the way Darling does it (clean room).
* **iOS apps are encrypted** (App Store copy protection). Removing that protection circumvents it: **Zohara will not build that**. We read what an `.ipa` declares in its plist and unencrypted metadata only.

## 5. Plan, in order

All tests in the VM (`~/zohara-vm`), never on the owner's laptop (rule in `HANDOFF-2026-10-18.md`).

1. **Mac app checker** (Settings, `zohara-settings`): drop or double-click a `.app` or `.dmg` (or `.ipa`); read `Info.plist` and the Mach-O load commands (pure Rust, tested with fixtures such as the `.dylib` above); answer "can run / might run / cannot", naming the missing frameworks. For `.ipa` say why it cannot run and point to the Android, web or Linux version.
2. **Darling in Zohara**: decide `darling-bin` (fast, prebuilt) or a Zohara-built package; add it to the repo channels (alpha first; remember a new package reaches existing machines only if an installed package depends on it, and it must exist in every channel). Measure in the VM with a real command-line program and a simple graphical `.app`; write down what works.
3. **Double-click `.app` / `.dmg`**: a handler like the Windows one: one Darling prefix per app (like the per-game Windows folders), `.dmg` unpacked first, the **log doctor** extended to explain Darling failures in plain words.
4. **Close the gaps** the failures show, most common first (the ranked list from section 4).

Each step ends with: what was tested, what was not, and a doc update here.

## 6. Related finding: PowerShell under Wine (Hermes installer), 2026-10-19

The Hermes Windows installer failed with `install.ps1 -Manifest produced no parseable JSON payload`: it downloads and runs a PowerShell script. Tested in a hidden desktop with GE-Proton11-9 and a throwaway folder:

* `winetricks -q powershell_core` installs PowerShell 7.4.11 (exit 0). Wine's own `powershell.exe` is a stub that exits 0 and prints nothing (this is the reason for the empty JSON); `winetricks -q powershell` (wrapper by Synchro, from codeberg.org) installs a real `powershell.exe` that forwards to `pwsh`.
* `pwsh.exe` first fails with `Cannot get symbol u_charsToUChars from libicuuc`; setting `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1` fixes that.
* Then it **crashes in `ConsoleHost.Start` (NullReferenceException) or does nothing** whenever it has **no terminal**; run under a **pseudo-terminal** (`script -qec "wine pwsh.exe ..."`) it worked and wrote its test file.
* Not tested: the real Hermes installer with these changes (it pipes PowerShell's output, which may still fail), the wrapper under a pseudo-terminal, double-click launches.
* Mistake to avoid when scripting: running the system `wineboot` before Proton's `wine` starts the wrong Wine server (`version mismatch 962/938`); use `wine wineboot`.
* Practical answer for Hermes: it has a Linux build, `hermes desktop`.

Possible fix (not built): a "PowerShell" entry in Windows parts that installs `powershell_core` + `powershell`, and Windows launches inside a pseudo-terminal with the invariant-globalization variable set.

## 8. Apps from archives (`.tar.gz` and friends), built 2026-10-20

The owner asked how an OS installs an app that comes as a `.tar.gz`. Short answer given: an archive is not an installer. Three cases: a ready-to-run app (extract and
start it, plus a launcher), source code (must be built, and a hand-built program is invisible to pacman), and an Arch package or recipe (pacman and the AUR).
Zohara now looks inside the archive, says which of these it is, installs ready-to-run apps into their own folder with a Start menu entry and a Remove button, and points
source code to the Store. Details, safety rules and what was and was not tested: `zohara-settings/docs/ARCHIVE-APPS.md` (settings commit on alpha).

## 7. Open questions for the owner

* Start with steps 1 and 2 (checker, Darling in the VM)? (asked 2026-10-19, not answered yet)
* Install Ghidra for REA's deeper Mach-O/PE analysis (a large download), or keep using small readers of our own?
* Should `.apk` double-click via Waydroid be checked next (it is already on the laptop)?
