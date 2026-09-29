#!/usr/bin/env bash
# Clones every Zohara repo next to each other, so ../zohara-settings etc. resolve
# the way the build scripts expect.  Usage: run once from an empty folder, e.g.
#   mkdir ~/zohara-dev && cd ~/zohara-dev && bash <(curl -fsSL https://raw.githubusercontent.com/Zohaib8090/zohara/master/scripts/clone-all.sh)
set -e
for r in zohara zohara-settings zohara-packages zohara-pipeline zohara-link zohara-apps; do
    [ -d "$r" ] || git clone "https://github.com/Zohaib8090/$r.git"
done
# The hub lives in a repo whose name differs from its folder:
[ -d zohara-hub ] || git clone https://github.com/Zohaib8090/zohara-updates-system.git zohara-hub
echo "Done. Next: read zohara/docs/HANDOFF-2026-09-29.md"
