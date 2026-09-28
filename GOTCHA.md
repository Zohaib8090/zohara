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

## Build pipeline

**A Docker BuildKit cache mount is a directory *snapshot*, not just a cache
for files that get written there anyway.** `Dockerfile` deliberately does
NOT cache-mount `/tmp/debtap` or `/tmp/calamares`: a cached empty directory
there would short-circuit the `git clone` into it (the clone sees a non-empty
mount point from a previous layer and may behave oddly), and the AUR build
artifacts need to live on the same filesystem layer as the later `cp`
anyway. Only mount pacman's own package cache dir as a BuildKit cache.

**`create_update_bundle.sh` used to extract package payloads directly into
the live checked-out `zohara-profile/airootfs`.** That's untracked output,
so `git reset --hard` never cleans it — it silently accumulates and causes
"exists in filesystem" conflicts on a reused clone. It now extracts into
the bundle's own scratch copy instead.

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
command completes.

## Git / line endings

**The signed update manifest broke because of CRLF vs LF, not a signing
bug.** It was signed on Windows (CRLF), then git normalized it to LF on
push — the signature never matched the served bytes, for every real
client, for as long as it existed. `manifest.json` and its `.minisig` are
now marked `-text` in `.gitattributes` so git never touches their line
endings, and CI verifies the signature against the committed file on
every push so this can't silently regress again.

## Terminal / shell

**zsh (the default live-USB shell) does not treat pasted lines starting
with `#` as comments the way bash does.** A script comment containing an
apostrophe (`"the installer's..."`) opened an unterminated quote and hung
the paste at a `quote>` prompt. Any script meant to be copy-pasted into a
terminal whose shell isn't guaranteed to be bash should wrap itself as
`bash <<'EOF' ... EOF` so the receiving shell just hands the whole block
to bash, comments and all. Reproduced by feeding the script to an
interactive `zsh -f -i` inside a real pseudo-terminal (`script -qfec`);
piping into non-interactive zsh doesn't reproduce it.
