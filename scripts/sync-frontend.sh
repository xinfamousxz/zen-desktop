#!/usr/bin/env bash
# Refresh the bundled Zen frontend snapshot in src/ from the artifact build.
#
# Run this on the dev machine BEFORE committing a desktop release, so the
# app ships the latest Zen UI. It only COPIES files — it never builds the
# artifact (the Zen builder owns the build; never run `bun run build` here).
#
# Usage:  bash scripts/sync-frontend.sh
# Requires: the artifact at ~/workspace/ts-spaces/real-platform
#           (its client/dist must be current — it is rebuilt by the Zen
#           builder on every accepted edit).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ARTIFACT_DIST="$HOME/workspace/ts-spaces/real-platform/client/dist"
DEST="$ROOT/src"

if [ ! -d "$ARTIFACT_DIST" ]; then
  echo "Artifact client/dist not found at $ARTIFACT_DIST" >&2
  echo "Nothing copied. The src/ snapshot already in the repo is used as-is." >&2
  exit 1
fi

# Copy the fresh bundle in. preload.html is NOT part of the artifact dist,
# so it is preserved (cp does not delete files that are already in src/).
cp -r "$ARTIFACT_DIST"/. "$DEST"/

echo "Synced Zen frontend -> $DEST"
du -sh "$DEST"
ls "$DEST"
test -f "$DEST/preload.html" && echo "preload.html present." || echo "WARNING: preload.html missing from src/ (it should live alongside index.html)."
