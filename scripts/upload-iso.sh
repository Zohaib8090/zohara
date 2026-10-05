#!/usr/bin/env bash
# Uploads a Zohara ISO to SourceForge (project "zohara-os") with rsync over ssh, together with a SHA256SUMS file
# and a README.md for the release folder (SourceForge shows the README on the folder page).
#
#   scripts/upload-iso.sh ISO_FILE            show the plan and ask before uploading
#   scripts/upload-iso.sh ISO_FILE --dry-run  write the README/SHA256SUMS and print the rsync command; connect to nothing
#   scripts/upload-iso.sh ISO_FILE --yes      upload without asking
#
# Settings (environment):
#   SF_USER     SourceForge username        (default: zohaib-baig, the lowercase name SourceForge shows)
#   SF_PROJECT  SourceForge project name    (default: zohara-os)
#   SF_KEY      ssh private key to use      (default: ~/.ssh/id_ed25519_sourceforge)
#
# The public half of the key must be added to your SourceForge account first. No credentials live in this repo.
# The ISO name must look like zohara-os-YYYY.MM.DD-x86_64.iso; the date becomes the folder name on SourceForge.
# SourceForge limits: files over 10 GB do not reach the mirrors, and file names may not contain & : % ? / * $ | { ; ^ } < > " '
# (docs: https://sourceforge.net/p/forge/documentation/Release%20Files%20for%20Download/). Keep about 5 GB or less
# per project in total: delete the oldest ISO from the SourceForge file manager when adding a new one.
set -euo pipefail

SF_USER="${SF_USER:-zohaib-baig}"
SF_PROJECT="${SF_PROJECT:-zohara-os}"
SF_KEY="${SF_KEY:-$HOME/.ssh/id_ed25519_sourceforge}"
HOST="frs.sourceforge.net"

log() { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
die() { printf '\n\033[1;31m!! %s\033[0m\n' "$*" >&2; exit 1; }

ISO="${1:-}"; MODE=ask
[ -n "$ISO" ] || die "usage: $0 ISO_FILE [--dry-run | --yes]"
case "${2:-}" in
    "")        ;;
    --dry-run) MODE=dry ;;
    --yes)     MODE=yes ;;
    *)         die "unknown option: $2" ;;
esac

[ -f "$ISO" ] || die "no such file: $ISO"
NAME="$(basename "$ISO")"
[[ "$NAME" =~ ^zohara-os-([0-9]{4}\.[0-9]{2}\.[0-9]{2})-x86_64\.iso$ ]] \
    || die "the file name must look like zohara-os-YYYY.MM.DD-x86_64.iso (got: $NAME)"
VERSION="${BASH_REMATCH[1]}"
DEST="/home/frs/project/$SF_PROJECT/$VERSION/"

command -v rsync >/dev/null || die "rsync is not installed"
if [ "$MODE" != dry ]; then
    command -v ssh >/dev/null || die "ssh is not installed"
    [ -f "$SF_KEY" ] || die "ssh key not found: $SF_KEY (create it and add its .pub file to your SourceForge account, or set SF_KEY)"
fi

WORK="$(mktemp -d)"; trap 'rm -rf "$WORK"' EXIT
log "Checksumming $NAME (a minute or so)"
SUM="$(sha256sum "$ISO" | cut -d' ' -f1)"
SIZE="$(stat -c %s "$ISO")"
printf '%s  %s\n' "$SUM" "$NAME" > "$WORK/SHA256SUMS"

GIB="$(awk -v s="$SIZE" 'BEGIN{printf "%.2f", s/1073741824}')"
cat > "$WORK/README.md" <<EOF
# Zohara OS $VERSION

A Linux that feels like home: an Arch-based system with KDE Plasma, a Windows-familiar desktop, its own Settings
app and software store, and voice typing that runs on your own computer.

**File:** \`$NAME\` ($GIB GiB, $SIZE bytes)
**SHA-256:** \`$SUM\`

## Install

1. Check the download (optional): put \`SHA256SUMS\` next to the ISO and run \`sha256sum -c SHA256SUMS\`.
2. Write the ISO to a USB stick with Ventoy, balenaEtcher or \`dd\`.
3. Boot from the stick and choose **Install Zohara** (or **Try Zohara** to look around first).

Before you boot:
- **Turn off Secure Boot.** The bootloaders are not signed yet.
- If the installer shows no disks, set the storage mode to **AHCI** instead of RAID / Intel RST.
- On a PC with Windows, choose **Install alongside** or **Manual partitioning**. The installer never picks
  "Erase disk" for you.

## Requirements

64-bit x86 PC, 4 GB RAM, 20 GB free disk, UEFI or legacy BIOS, Intel / AMD / modern NVIDIA graphics.

## Status

Zohara OS is a young project. Try it in a virtual machine or on a spare computer first.

## More

- Website: https://zohara-website.onrender.com/
- Source and issues: https://github.com/Zohaib8090/zohara
- Licence: MIT and GPL-3.0-or-later; third-party software keeps its own licences.
EOF

# IPQoS=throughput: with ssh's default traffic marking the upload ran at about 17 KB/s from Pakistan (raw uplink
# 6 MB/s); marked as bulk traffic it runs at about 5.6 MB/s. Round trip to SourceForge is ~400 ms.
RSYNC=(rsync -e "ssh -i $SF_KEY -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new -o IPQoS=throughput" --partial --progress "$ISO" "$WORK/SHA256SUMS" "$WORK/README.md" "$SF_USER@$HOST:$DEST")

log "Plan"
echo "  file:        $NAME  ($GIB GiB)"
echo "  SHA-256:     $SUM"
echo "  destination: $SF_USER@$HOST:$DEST"
echo "  also sends:  SHA256SUMS, README.md"
echo "  the file will appear at https://sourceforge.net/projects/$SF_PROJECT/files/$VERSION/$NAME/download"

if [ "$MODE" = dry ]; then
    log "Dry run: nothing was uploaded. The generated README.md:"
    sed 's/^/    /' "$WORK/README.md"
    echo; echo "  command that would run: ${RSYNC[*]}"
    exit 0
fi
if [ "$MODE" = ask ]; then
    read -r -p "Upload now? [y/N] " a
    [[ "$a" =~ ^[Yy]$ ]] || die "cancelled"
fi

log "Uploading (safe to interrupt and re-run: --partial resumes)"
"${RSYNC[@]}"
log "Done. Check https://sourceforge.net/projects/$SF_PROJECT/files/$VERSION/"
