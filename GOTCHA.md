# Gotchas

Real traps in this project that already cost real time once. If you hit
one of these again, it means this file wasn't read, not that the problem
is new. Add to it whenever something takes more than a few minutes to
figure out and isn't obvious from the code alone.

## Calamares / the installer

**Calamares' `mount` module can crash with SIGPIPE on Btrfs.**
`src/modules/mount/main.py`'s `mount_partition()` runs
`subprocess.check_call(["umount", "-v", root_mount_point])` after setting
up subvolumes, and that verbose `umount` sometimes dies with `SIGPIPE`
instead of exiting cleanly, failing the whole install. This is a six-year-old
unresolved upstream bug — other distros hit it too, no fix has landed.
Confirmed on real hardware (Dell Latitude, 2026-09-28).
Worked around in `zohara-profile/airootfs/root/customize_airootfs.sh`,
which `sed`s the `-v` flag out of the installed `mount/main.py` after
pacstrap. It's defensive: if the sed pattern stops matching (Calamares
changed the line), the build prints a warning instead of silently no-op'ing.

**`extraMountsEfi` in `mount.conf` is dead. Calamares 3.3+ wants `efi: true`
on an entry inside `extraMounts` instead.** Our config used the old key,
so `efivarfs` was never mounted into the target — Calamares 3.4.2 just
ignored the whole unknown key with no warning. Every UEFI install failed
at the bootloader step: `grub-install` needs `/sys/firmware/efi/efivars`
to register a boot entry, and without it: `EFI variables are not supported
on this system`. Fixed in `zohara-profile/airootfs/etc/calamares/modules/mount.conf`.
If you're editing Calamares module configs, check the *installed* Calamares
version's own schema/docs, not memory of an older one — keys get silently
dropped, not rejected.

**A stale `work/` directory makes `mkarchiso` skip the entire build, silently,
exit 0, no ISO.** mkarchiso gates its whole build mode behind one sentinel
file in `work/`, not just the squashfs step. `zohara-profile/build-iso.sh`
now always `rm -rf`s `work/` before building and explicitly fails if no ISO
exists afterward, rather than trusting the exit code.

**GRUB's Btrfs reader can't read a file that ends in a sparse hole.** The
kernel image ends in zero padding; the copy unpacked from the squashfs
stores that tail as a hole. Plain `cp` keeps the hole, Linux reads it back
as zeros (so `cmp` says the files are identical), but GRUB 2.16 stops at
the last stored block and fails with `premature end of file
/@/boot/vmlinuz-linux-zen`. It looked like a Btrfs reflink/compression
problem and wasn't -- a standalone `--reflink=never` copy still failed.
Copy boot files with `cp --sparse=never --reflink=never`. Found by
installing in the QEMU test VM and reproducing the Dell's exact error.

**A failed install can leave the Windows EFI partition with a corrupted
`EFI/Zohara_OS` folder**, and grub-install then fails on the next try. Fix
from Windows (admin): `mountvol S: /s`, `chkdsk S: /f`, `mountvol S: /d`.

**Dell SupportAssist hijacks boot after failed boots.** After two failures
the firmware auto-launches SupportAssist OS Recovery, and picking Zohara
just shows its scan screen; if the boot entry is gone it says "no bootable
device". Turn off BIOS > SupportAssist System Resolution > Auto OS
Recovery Threshold before testing installs, and use
`scripts/repair-boot-entry.sh` if the Zohara_OS entry is missing.

## Build pipeline

**A Docker BuildKit cache mount is a directory *snapshot*, not just a cache
for files that get written there anyway.** `Dockerfile` deliberately does
NOT cache-mount `/tmp/debtap` or `/tmp/calamares`: a cached empty directory
there would short-circuit the `git clone` into it (the clone sees a non-empty
mount point from a previous layer and may behave oddly), and the AUR build
artifacts need to live on the same filesystem layer as the later `cp`
anyway. Only mount pacman's own package cache dir as a BuildKit cache.

**`scripts/create_update_bundle.sh` used to extract package payloads directly into
the live checked-out `zohara-profile/airootfs`.** That's untracked output,
so `git reset --hard` never cleans it — it silently accumulates and causes
"exists in filesystem" conflicts on a reused clone. It now extracts into
the bundle's own scratch copy instead.

**pacman fetches `<section name>.db`, not `zohara.db`.** Installed systems
have `[zohara-stable]` in `/etc/pacman.conf`, so `pacman -Sy` asks the
`zohara-packages` release for `zohara-stable.db`. The publish workflow used to
upload only `zohara.db`, so on every fresh install `pacman -Sy` failed with a
404 and no system update (kernel included) could be installed. Found by running
an update in the QEMU test VM (2026-09-30); the publish workflow now uploads
both names. If the section name and the release's file name ever differ again,
this comes back. The same run showed that two packages published within seconds
of each other can overwrite each other's entry in the database (`zohara-voice-model`
was in the release but not in `zohara.db`).

**`makepkg` sets `SOURCE_DATE_EPOCH`, and that silently changes some CMake projects.** whisper.cpp's
ggml turns every SIMD option (AVX, AVX2, FMA, ...) off by default when it sees that variable. The
`zohara-voice` "AVX2" build set no flags of its own, so under `makepkg` it came out byte-identical to the
baseline build: same hash, zero AVX instructions, and every machine ran the slow path (4.4x slower). A
plain `cmake` run does not show it (no `SOURCE_DATE_EPOCH`), which is why it looked fine when tried by hand.
Pass the flags explicitly, and check with `objdump -d BINARY | grep -c ymm` on the built package, not on a
hand build. Found 2026-09-30.

**Brave Origin and Brave Browser are different products.** `brave-origin-bin` is Origin (the one Zohara
ships), `brave-bin` is Browser. The `Dockerfile` cached `brave-bin` for a while by mistake; nothing installed
it, it only cost a 192 MB download. Brave Origin is free on Linux but asks once on first launch: the user
must click "Proceed with Origin for free on Linux".

**A single 503 from `cdn-mirror.chaotic.cx` fails the whole ISO build.** The `Dockerfile` fetches
`chaotic-keyring` and `chaotic-mirrorlist` with no retry. It happened on 2026-09-29 (run at 06:50 UTC, failed
after 3 minutes at "Build Docker builder image"). Re-running the build is enough.

## Release admin site (zohara-updates-system) and Render

**Render "Resume" restarts the last build that went live, not the newest commit.** Suspending the old open service and resuming
it later put the old, login-free code back on the internet for minutes (its newer commit had been cancelled by the suspend).
Add the environment variables first, then resume, then press "Deploy latest commit". Check with `curl -I` that `/` answers
303 to `/login`, never 200, before leaving it. Suspending needs a typed phrase (`sudo suspend web service <name>`).

**`/publish` on the old service trusted the repository named in the form.** Any repo the GitHub App was installed on, and any
run id, could be published to the stable channel, and installed systems accept unsigned packages from `[zohara-stable]`
(`SigLevel = Optional TrustAll`). The new workflow lists the allowed source repos and re-checks the run itself
(`zohara-packages/scripts/`). Keep both checks when changing either side.

**Never name a file after a secret, and never attach key files or screenshots of secret fields to a chat.** A client secret was
saved as `<secret>.txt` and a Render screenshot showed the start of a private key: both ended up in a transcript. Rotate
anything that was shown (see open item 1 in `HANDOFF-2026-10-04.md`).

**The GitHub App repository picker freezes Chrome** (the whole renderer hangs). Submit the settings form directly instead:
`install_target=selected` plus one `repository_ids[]` hidden input per repo id. Also: a Chrome-extension click on "Generate a
private key" did nothing; a human click works.

**GitHub Actions: only one run per concurrency group waits, the others are cancelled.** That is why `publish.yml` has no
`concurrency:` block: with several packages dispatched at once, the middle ones would be dropped. It re-reads the published
database after upload and merges again instead.

## Linux dev machine (Ubuntu / Zorin)

**`docker.io` alone cannot build the ISO image.** The `Dockerfile` uses BuildKit cache mounts, and Ubuntu's
`docker.io` ships without `buildx`: "BuildKit is enabled but the buildx component is missing". Install
`docker-buildx` as well. If `docker.service` fails with "no sockets found via socket activation" after a
remove and reinstall, run `daemon-reload`, `reset-failed docker.socket docker.service`, then restart
`docker.socket`. Zorin's libadwaita is 1.5, but the apps ask for `v1_6`, so they do not compile on the host:
build in an Arch container (that is also what makes the result an Arch binary).

**The QEMU test VM drops and sticks keys if you type fast.** `vm.py type` at its default speed left a key
repeating (a wall of `NNNN`) and the guest stopped answering Ctrl-C. Slow the typing down (0.15 s between
keys, 120 ms hold), and for anything long serve a script from the host and type one short `curl | bash`.
See `scripts/vm/README.md`.

## Windows dev machine

**Windows checkouts can't create real symlinks** (no
`SeCreateSymlinkPrivilege`). Any `.wants/` systemd unit symlink or
`/etc/localtime`-style symlink checked out on Windows becomes either
missing or a regular file containing the link-target text — and a regular
file there *shadows* the real unit forever, even after a package update
ships a fixed one. Pinned symlinks are excluded from the Windows-side
rsync overlay with `git update-index --skip-worktree`; `build-iso-wsl.sh`
builds from a Linux-side clone instead, where the symlinks are real, and
layers uncommitted Windows edits on top. If a service mysteriously won't
enable or a stale config keeps winning, check whether it's a
skip-worktree-pinned path that's supposed to be a symlink.

**A backgrounded WSL process dies when the `wsl -e ...` invocation that
started it returns**, even with `nohup` and `&`. Use `setsid nohup ... &
disown` from inside the `wsl -e bash -lc "..."` command, not just `nohup`,
or the "background" build silently vanishes the moment the launching
command completes. Follow the launch with a `sleep` in the same command
(a few seconds is enough) or the detached job can still be killed.

## Git / line endings

**The signed update manifest broke because of CRLF vs LF, not a signing
bug.** It was signed on Windows (CRLF), then git normalized it to LF on
push — the signature never matched the served bytes, for every real
client, for as long as it existed. `manifest.json` and its `.minisig` are
now marked `-text` in `.gitattributes` so git never touches their line
endings, and CI verifies the signature against the committed file on
every push so this can't silently regress again.

## Terminal / shell

**ssh to SourceForge can crawl at ~17 KB/s while the connection is fast; use `-o IPQoS=throughput`.** From Pakistan
the round trip to `frs.sourceforge.net` is ~400 ms, and with ssh's default traffic marking an upload of the ISO
ran at about 17 KB/s (about 60 hours) although the raw uplink is ~6 MB/s. Marked as bulk traffic it ran at
about 5.6 MB/s (the whole ISO in ~12 minutes). `scripts/upload-iso.sh` sets it. When a transfer is slow, measure
the raw link to a neutral server first (`curl -X POST --data-binary @file https://speed.cloudflare.com/__up`), and
do not trust rsync's own rate display for the first minute. SourceForge usernames are lowercase (`zohaib-baig`).

**SourceForge cannot give a browser a direct download; the website uses OCI Object Storage for that.** Both
`sourceforge.net/projects/.../download` and `downloads.sourceforge.net/project/...` show SourceForge's "Your download
will start shortly" page to browsers (the second one behind a Cloudflare "Just a moment" check first). `curl` gets a
plain 302 to a mirror, which is why testing with `curl` looked fine. Do not "fix" the button by switching SourceForge
link styles. The ISO is on a public OCI bucket (`ap-mumbai-1`, namespace `bm27e3oxmp04`, bucket `zohara-os`) and
SourceForge stays as a mirror. Details and the upload commands are in `docs/HANDOFF-2026-09-30.md` (open item 7).

**Oracle's OCI CLI installer fails on Python 3.12, and its browser login needs the password every time.**
`install.sh` bundles virtualenv 20.6, which crashes on 3.12, and `python3 -m venv` needs `python3-venv` (not
installed). What worked: `python3 -m venv --without-pip DIR`, `get-pip.py`, `DIR/bin/pip install oci-cli`. For
uploads use `oci session authenticate` (no API key left on disk; the token lasts about an hour, so start the upload
right after logging in) and `oci os object put --part-size 128 --parallel-upload-count 6`. A browser extension's file
upload is capped at 10 MB, so the ISO cannot go through the console from an automated browser.

**`pkill -f PATTERN` (and `pgrep -f`) can match the very shell that runs it.** If PATTERN appears in your own
command line (for example a script name you are also typing), the shell kills itself and the tool reports exit
code 144. Anchor the pattern (`pgrep -f '^python3 srv.py'`), use the process id, or stop the thing through its
own interface (`vm.py quit`). Also: a wait loop that greps a log for ` Install$` matches `click Install` too;
wait on a file or a line that only the target produces.

**zsh (the default live-USB shell) does not treat pasted lines starting
with `#` as comments the way bash does.** A script comment containing an
apostrophe (`"the installer's..."`) opened an unterminated quote and hung
the paste at a `quote>` prompt. Any script meant to be copy-pasted into a
terminal whose shell isn't guaranteed to be bash should wrap itself as
`bash <<'EOF' ... EOF` so the receiving shell just hands the whole block
to bash, comments and all. Reproduced by feeding the script to an
interactive `zsh -f -i` inside a real pseudo-terminal (`script -qfec`);
piping into non-interactive zsh doesn't reproduce it.

## Packages, signing, the Store and Settings (2026-10-06)

* **A new package never arrives with `pacman -Syu`.** Upgrades only touch installed packages. To ship a brand-new package
  (like `zohara-keyring`) to existing machines, make an installed package depend on it (Settings and the Store do). Then
  publish it to **every** channel, or installing from alpha/beta fails on the unresolved dependency. CI containers do not
  have it either: build with `makepkg --nodeps`.
* **A signed repository breaks machines that do not have the key yet** ("unknown key", tested). Order: keyring package
  first, signing secret second, `SigLevel = Required` last. `Optional TrustAll` still checks a signature that is there.
* **The Store refuses a manifest older than the date the machine is pinned to** ("older than what this computer already
  has"). The ISO build pinned 2026/09/26 while the signed approval said 09/25, so no Store could update. Keep the signed
  `approved_date` at or above the ISO's pin. Signing needs the minisign secret (`~/.minisign/zohara.key`); the old one was
  lost in 2026-10.
* **`adw::HeaderBar::set_show_title(false)` hides the centre `title_widget` too.** It hid the Store's tabs and Settings'
  search box for months. Do not call it when a title widget is set.
* **An `adw::ActionRow` outside a `ListBox` highlights on hover and does nothing when clicked.** Put it in a
  `PreferencesGroup`, or wrap it with `pages::in_list` (Settings). Rows in a plain `Box` next to expander rows were dead.
* **pacman writes its output in blocks when piped.** Run it as `stdbuf -oL pacman ...` or a progress bar stays at 0 until
  the end. Downloads sit in `/var/cache/pacman/pkg/download-XXXXXX/*.part` (several at once), not in the cache folder.
* **A plain `pacman -Sy` from a normal user always fails** ("you cannot perform this operation unless you are root"). Settings
  did exactly that for its update check. Only the Store does the checking (signed, pinned, `fakeroot` + private db).
* **GTK/libadwaita theme**: `@define-color window_fg_color` does not reach libadwaita's own widgets, which use
  `--window-fg-color` and friends. Override the variables too (`theme.rs`), or text comes out black on a dark window.
* **Do not run `pkill -f PATTERN` from a command that contains PATTERN**: it kills its own shell (exit 144). Same for a
  `sleep N; ...` chain: the harness blocks it. Use `until` loops or `gh run watch`.
* **`git pull --rebase` / `commit` fail with "Author identity unknown"** on this laptop (no global identity). Each repo now
  has a local name/email; if a rebase stops, `git commit` inside it and `GIT_EDITOR=true git rebase --continue`.
  `zohara-packages` gets automatic `apps.json` commits, so rebase before pushing.
* **`cargo test` does not rebuild the app binary.** Run `cargo build` before launching it for a screenshot.
* **Pushing a build workflow can publish.** `build-store.yml`, `build-snapshots.yml`, `build-voice.yml`,
  `build-welcome.yml` (zohara) and `zohara-apps` CI still publish to stable on a push. Use `[skip ci]` on commits that
  touch their paths, or remove the publish steps (done for `zohara-settings`).
* **The ISO's Dockerfile step that downloads `brave-origin-bin` fails with a Chaotic-AUR 404** when its mirrors lag behind
  its database. It now retries (6 tries).

## Settings and the VM (2026-10-06, second half)

* **A row outside a `ListBox` never reacts to clicks.** This silently killed Display's pickers, Sound's pickers, the Wi-Fi and
  Personalization expanders, Home's buttons. `pages::adopt_orphan_rows` (zohara-settings, run on every page in `build_page`)
  now moves orphan rows into a list. If a new control "highlights but does nothing", check this first.
* **`&` in a row title is read as markup** and the whole title vanishes ("Power & battery"): call `set_use_markup(false)`.
* **A `ComboRow`'s selected-item box reports `position() == 0`**, so looking the app up by position shows the first entry for
  whatever is selected (made the default-browser row look unchanged after a successful change). Match on the item's own label.
* **Calamares treats `$name` in a module command as its own variable** and refuses to run it ("Bad variables"): no dollar signs in
  `shellprocess_*.conf` commands.
* **`pacman -Sy` under `fakeroot` fails on real installs** ("Landlock ruleset could not be applied", "switching to sandbox user
  'alpm' failed"): pass `--disable-sandbox` for the private check.
* **`libinput-tools` is a separate Arch package**; `libinput debug-events` does not exist without it.
* **The test VM boots the ISO before its disk** (`bootindex=1`): after installing, "restart" shows the live system again. Boot
  the disk alone (`run-disk.sh`) to test the installed system.
* **`/tmp` scripts vanish** between sessions/reboots: keep the VM start script inside the VM folder.
* **A workflow that only builds must not publish**: publishing is `publish-alpha.yml` (workflow_run) so a failed build cannot
  publish; its channel is fixed to alpha.

## 2026-10-07

* **`L+` in a tmpfiles rule forces a path back at every boot and after package updates** (Arch's `21-systemd-tmpfiles.hook`). The old
  `zohara-localtime.conf` (`L+ /etc/localtime ... UTC`) reset every machine's time zone. Use a real symlink made at build time instead.
* **A running Settings keeps its old code after an update.** Closing the window is not always enough; after "Update all" make sure the old window is
  gone (`X`, check the taskbar) before relaunching, or you will test the previous build and think a fix failed.
* **A modal pop-up (Themes, Text input, Add a game...) blocks clicks to the window behind it**, including the sidebar. Close it first.
* **`vm.py type` cannot type `|` and drops characters on long lines** (a stuck key can fill the terminal). Serve a script with
  `scripts/vm/srv.py` and fetch it with `curl -so /tmp/f.sh 10.0.2.2:8000/NAME; bash /tmp/f.sh`. Key names are lowercase QMP names (`ret`, `esc`).
  The QMP socket path must be under 108 bytes: use a short dir like `/tmp/zt`, not the scratchpad.
* **The Arch archive server resets HTTP/2 streams** ("stream reset by server") during the ISO build and kills it. The profile `pacman.conf` now uses
  `XferCommand = curl --http1.1 ... --retry`. A new package that Settings depends on (`libinput-tools`) must also be in `packages.x86_64`,
  or it is fetched late and can fail the build.
* **GitHub release files are limited to 2 GB** and the ISO is 3.9 GB: the GitHub release `iso-2026.10.07` has only notes and links. Do not name a tag
  `v*` unless you want it to start an ISO build (`build-iso.yml` runs on `v*` tags).
* **The OCI CLI session expires**; renew it with the owner's browser sign-in (`oci session authenticate --profile zohara`, region 14 =
  `ap-mumbai-1`), then use `--profile zohara --auth security_token`. Pick the file to publish by its hash (CI build and local build differ).
* **A `sed`/python text replace that silently does not match leaves the old text in the file**: after editing, grep for the new text (a stale
  description string made a good build look like the old one).
* **`ComboRow`/`DropDown` search needs an expression** (`PropertyExpression` on `StringObject`'s `string`), or typing does nothing.
* **`gh run download` writes nothing until the whole artifact has arrived** (about 15 minutes for the ISO); the folder looks empty meanwhile.
