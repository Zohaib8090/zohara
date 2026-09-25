#!/usr/bin/env bash
# Publish a built package to the zohara-packages release and tell that repo to
# rebuild its package database. Used by the build workflows in this repo.
#
#   publish-package.sh <package-file> <package-name> <version> <artifact-name>
#
# The workflow must already have uploaded the package as an Actions artifact
# named <artifact-name> (zohara-packages downloads it from this run).
# Environment: GH_TOKEN (PACKAGES_DISPATCH_TOKEN), GITHUB_RUN_ID, CHANNEL (default stable).
set -euo pipefail

FILE="$1"; NAME="$2"; VER="$3"; ARTIFACT="$4"
REPO="${PACKAGES_REPO:-Zohaib8090/zohara-packages}"
CHANNEL="${CHANNEL:-stable}"

if [ -z "${GH_TOKEN:-}" ]; then
    echo "::warning::PACKAGES_DISPATCH_TOKEN is not set; $NAME is only in the build artifact."
    exit 0
fi
if [ "$CHANNEL" = "stable" ]; then TAG="stable"; else TAG="channel-$CHANNEL"; fi

if ! gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
    PRE=""; [ "$CHANNEL" != "stable" ] && PRE="--prerelease"
    gh release create "$TAG" --repo "$REPO" --title "Zohara Packages — $CHANNEL" \
        --notes "Auto-managed Arch package repository for the **$CHANNEL** channel." $PRE || true
fi
gh release upload "$TAG" "$FILE" --repo "$REPO" --clobber

BASE="$(basename "$FILE")"
T0="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
gh api "repos/$REPO/dispatches" --method POST --input - <<JSON
{
  "event_type": "package-published",
  "client_payload": {
    "package_name": "$NAME",
    "version": "$VER",
    "artifact_url": "https://github.com/$REPO/releases/download/$TAG/$BASE",
    "run_id": "$GITHUB_RUN_ID",
    "source_repo": "Zohaib8090/zohara",
    "artifact_name": "$ARTIFACT",
    "pkg_filename": "$BASE",
    "channel": "$CHANNEL"
  }
}
JSON
echo "Published $NAME $VER to $TAG"

# WAIT=1: hold until zohara-packages has finished rebuilding its database, so
# a second package published right after doesn't overwrite this one's entry.
if [ "${WAIT:-0}" = "1" ]; then
    echo "Waiting for zohara-packages to finish publishing..."
    for _ in $(seq 1 90); do
        sleep 10
        NEW=$(gh run list --repo "$REPO" --workflow publish.yml --limit 10 --json status,createdAt \
              -q "[.[] | select(.createdAt >= \"$T0\")] | length" 2>/dev/null || echo 0)
        OPEN=$(gh run list --repo "$REPO" --workflow publish.yml --limit 10 --json status,createdAt \
              -q "[.[] | select(.createdAt >= \"$T0\" and .status != \"completed\")] | length" 2>/dev/null || echo 1)
        if [ "$NEW" -ge 1 ] && [ "$OPEN" -eq 0 ]; then
            echo "zohara-packages finished."
            break
        fi
    done
fi
