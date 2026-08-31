#!/usr/bin/env bash
# download-and-flash.sh -- download the latest Zohara ISO from CI and flash
# it to the Kingston USB drive.
#
# USAGE:  bash download-and-flash.sh
#
# This script:
#   1. Finds the latest successful CI run that has the ISO artifact
#   2. Downloads zohara-os-x86_64.zip
#   3. Extracts the .iso
#   4. Flashes it to the Kingston USB drive (Disk 1)

set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST="$REPO/out"
mkdir -p "$DEST"

echo "[1/4] Finding the latest CI run with the ISO artifact..."
# Find the most recent successful run
RUN_ID=$(gh run list --workflow=build-iso.yml --limit 20 --json databaseId,conclusion --jq '.[] | select(.conclusion == "success") | .databaseId' | head -1)
if [ -z "$RUN_ID" ]; then
    echo "ERROR: no successful CI run found yet."
    echo "Check https://github.com/Zohaib8090/zohara/actions/workflows/build-iso.yml"
    exit 1
fi
echo "Using run ID: $RUN_ID"

echo "[2/4] Downloading artifact..."
cd /tmp
rm -rf zohara-artifact
mkdir -p zohara-artifact
cd zohara-artifact
gh run download "$RUN_ID" --name zohara-os-x86_64 --dir . 2>&1 | tail -3
ls -lh

echo "[3/4] Finding the ISO file..."
ISO=$(ls *.iso 2>/dev/null | head -1)
if [ -z "$ISO" ]; then
    # Maybe it's zipped
    ZIP=$(ls *.zip 2>/dev/null | head -1)
    if [ -n "$ZIP" ]; then
        echo "Extracting $ZIP..."
        powershell -Command "Expand-Archive -Path '$PWD/$ZIP' -DestinationPath '$PWD' -Force" 2>&1 | tail -3
        ISO=$(ls *.iso 2>/dev/null | head -1)
    fi
fi
if [ -z "$ISO" ]; then
    echo "ERROR: no .iso file in the downloaded artifact"
    ls -la
    exit 1
fi
echo "ISO: $ISO ($(du -h "$ISO" | cut -f1))"

echo "[4/4] Flashing to USB drive (Disk 1)..."
# Use PowerShell to do the raw write. This is safer than dd on Windows.
powershell -Command "
    \$iso = '$PWD/$ISO'
    \$disk = Get-Disk -Number 1
    Write-Host \"Target disk: \$(\$disk.FriendlyName), Size: \$([math]::Round(\$disk.Size/1GB,1)) GB\"
    Write-Host \"Source ISO:  \$iso, Size: \$([math]::Round((Get-Item \$iso).Length/1GB,2)) GB\"
    Write-Host 'Offline the disk and clear partitions...'
    Set-Disk -Number 1 -IsOffline \$true
    Initialize-Disk -Number 1 -PartitionStyle GPT
    Set-Disk -Number 1 -IsReadOnly \$false
    Write-Host 'Writing ISO via raw disk access (this takes 5-10 min)...'
    \$stream = [System.IO.File]::OpenRead(\$iso)
    \$diskStream = [System.IO.File]::OpenWrite('\\\\.\\PhysicalDrive1')
    \$diskStream.SetLength(\$stream.Length)
    \$buffer = New-Object byte[] 1MB
    \$totalRead = 0
    while ((\$read = \$stream.Read(\$buffer, 0, \$buffer.Length)) -gt 0) {
        \$diskStream.Write(\$buffer, 0, \$read)
        \$totalRead += \$read
        if ((\$totalRead % 100MB) -lt \$buffer.Length) {
            Write-Host \"  Written \$([math]::Round(\$totalRead/1MB,0)) MB...\"
        }
    }
    \$stream.Close()
    \$diskStream.Close()
    Write-Host 'Flash complete!'
" 2>&1 | tail -20
echo ""
echo "DONE. USB drive E: now has the Zohara OS ISO. Boot from it to test."
