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
