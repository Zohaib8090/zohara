# Moving the owner's laptop from Zorin to Zohara (written 2026-10-08, before the switch)

The owner decided to remove Zorin and install Zohara on the laptop that is also the development machine. This page is for the
next session, which will run **on Zohara** with no memory of the earlier ones. Read it first, then `HANDOFF-2026-10-06.md`
(last sections), `GOTCHA.md` and `docs/READINESS-2026-10-08.md` (why a full switch was risky).

## The rescue stick (Ventoy, 115 GB, label `Ventoy`)

Created 2026-10-08 with Ventoy 1.1.17 (checksum matched the project's published one). It holds, on its data partition:

| Path | What |
|---|---|
| `zohara-os-2026.10.08-x86_64.iso` | the ISO built locally 2026-10-08 13:04 (Zohara OS 1.1), sha256 `44de7c35372ecb26f4121a6ed85f3a448fe84d478ef0f286f4c9e92545076d52` |
| `Zorin-OS-18.1-Core-64-bit.iso` | Zorin OS 18.1 Core from the official kernel.org mirror, to go back to Zorin if the switch goes wrong; official sha256 `44649b97bd307fc4c8529205d098ebbf98575a1e1ba2ee7d7005b697af1721d5` |
| `ZOHARA-BACKUP-2026-10-08/projects/` | every folder of `~/Documents` as `<name>.tar.zst` (with `.git`; without Cargo `target`, ISO `work`/`out`, `*.iso`) |
| `ZOHARA-BACKUP-2026-10-08/secrets/` | the update-system `.pem`, `~/.minisign`, `~/.zohara-signing`, `~/.oci`, `~/.ssh` (**unencrypted: keep the stick safe; this is the second copy of the keys the handoff asked for**) |
| `ZOHARA-BACKUP-2026-10-08/claude/dot-claude.tar.zst` | `~/.claude`: memory, session transcripts, skills |
| `ZOHARA-BACKUP-2026-10-08/docker/zs-img.tar.zst` | the `zs-img` developer image (`docker load`) |
| `ZOHARA-BACKUP-2026-10-08/START-HERE.txt` | this list in plain text |

Not backed up (rebuildable): the test VM disk `~/zohara-vm` (23 GB, a fresh install from the ISO recreates it), the
`zohara-builder` image (rebuilt by `scripts/build-iso-linux.sh`), Cargo `target` folders, the ISO build folders, Gmail/Chrome/GitHub
logins. All repos were clean and fully pushed to GitHub at backup time, so GitHub holds the same code.

## Restoring the work on Zohara

1. Install the tools (pacman): `git github-cli docker docker-buildx rustup qemu-full edk2-ovmf rsync jq minisign python
   openssh zstd xorriso`. Add yourself to the `docker` group, start docker.
   (`oci-cli` is not in the Arch repos: `pip install --user oci-cli` in a throwaway venv, dev machine only, never in the OS.)
2. Copy the stick's `projects/*.tar.zst` somewhere and extract each into `~/Documents/`:
   `tar -I zstd -xf <name>.tar.zst -C ~/Documents`. Or just `git clone` the repos from GitHub (`Zohaib8090/<name>`); use the
   archive for `kse` (not a git repo) and anything unpushed.
3. Secrets: extract `secrets/dot-*.tar.zst` into `$HOME` (`tar -I zstd -xf ... -C ~`), then `chmod 700 ~/.ssh ~/.minisign ~/.zohara-signing`
   and `chmod 600` the private keys. `gh auth login` again (tokens were not copied). The OCI login is a short session:
   `oci session authenticate --profile-name zohara --region ap-mumbai-1` (the owner does it in the browser).
4. Claude Code: install it, then `tar -I zstd -xf claude/dot-claude.tar.zst -C ~`. The memory index `MEMORY.md`, the project
   transcripts and the `zohara-os-dev` skill come back. The memory path is keyed by the working directory
   (`~/.claude/projects/-home-zohaib-Documents-zohara`), so keep the same user name and folder.
5. Docker images: `zstd -dc zs-img.tar.zst | docker load`. If it is missing, build from `scripts/dev/zs-img.Dockerfile`
   (reconstructed, unverified). The ISO builder rebuilds itself: `scripts/build-iso-linux.sh` (needs about 30 GB free).
6. Test VM: boot the ISO in QEMU (see `scripts/vm/README.md`, `run-vm.sh`) and install; make a user `tester` with the throwaway
   password `Zvm-test-2026`; helper scripts in `scripts/vm/guest/`.

## State at the time of the switch

* **Stable**: Settings `0.1.0.202610080705`, Store `0.1.0.202610060728`. **Alpha** is newer (Startup apps, cleanup fix,
  restore point delete, About rows). Details: `HANDOFF-2026-10-06.md` "2026-10-08".
* **ISOs 2026.10.08** (local and CI run `37740462144`) are built, **not boot-tested, not hosted**; the website still points
  at ISO 2026.10.07. The release number is 1.1.
* The first thing to do on Zohara is the owner's real-hardware pass: sleep/resume and lid, Wi-Fi/Bluetooth, hibernate (swap
  size), a big update plus "Undo the last update", Windows games, Flatpak Chrome as a web-app browser.
* Still open from earlier: the sleep logout bug, package signing rollout (owner pastes the secret) (the dad's-laptop Wi-Fi was fixed, see `HANDOFF-2026-10-06.md` "2026-10-08 (later)"),
  Secure Boot (deferred), secret cleanup (rotate the GitHub App client secret; the old `.pem` copies).

## If the install goes wrong

Boot the stick with Ventoy (BIOS boot menu key, Secure Boot must be off), choose the Zorin ISO, and reinstall Zorin. The
backup above is enough to rebuild the whole development setup on any Linux.
