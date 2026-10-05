#!/usr/bin/env bash
# End-to-end check of the whole signing chain with Zohara's REAL signing key, in a throwaway Arch container:
#   1. a machine without the key refuses a signed repository
#   2. the zohara-keyring package installs the key
#   3. the same machine now accepts the signed repository and installs from it, and still refuses an unsigned one
# The secret key (~/.zohara-signing/zohara-packages-secret.asc) is mounted read-only into the container, which is
# deleted afterwards. Needs docker, and the sibling checkout ../zohara-packages.
#   bash zohara-keyring/test-e2e.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
secret="${ZOHARA_SIGNING_SECRET:-$HOME/.zohara-signing/zohara-packages-secret.asc}"
[ -r "$secret" ] || { echo "no secret key at $secret"; exit 1; }
exec docker run --rm -v "$here":/keyring:ro -v "$here/../../zohara-packages":/pkgs:ro -v "$secret":/secret.asc:ro \
  archlinux:latest bash /keyring/test-e2e-inner.sh
