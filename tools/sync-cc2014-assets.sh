#!/usr/bin/env bash
# Copy the Cricket Coach 2014 assets into src/assets/cc2014/ and regenerate the
# loader manifest.
#
# The assets are the original's and are not licensed for redistribution, so
# they are git-ignored (see THIRD-PARTY.md). Run this after a fresh clone.
#
#   ./tools/sync-cc2014-assets.sh [path/to/Cricket Coach 2014]

set -euo pipefail

SRC="${1:-old-game/Cricket Coach 2014}"
DEST="src/assets/cc2014"

if [ ! -d "$SRC" ]; then
  echo "No game at '$SRC'." >&2
  echo "Pass the installed game directory as the first argument." >&2
  exit 1
fi

mkdir -p "$DEST/outcomes" "$DEST/shots" "$DEST/audio"

# Outcomes: the HD set where it exists, then the 32px set for the rest.
cp -n "$SRC/graphics/tiles/HD/Outcomes/"*.png "$DEST/outcomes/" 2>/dev/null || true
cp -n "$SRC/graphics/Outcomes32/"*.png "$DEST/outcomes/" 2>/dev/null || true

# Strokes: the complete .bmp set, which carries the mirrored _L variants the
# game uses for left-handers. The HD .png set is partial, so it is a fallback.
cp -n "$SRC/graphics/tiles/shots/"*.bmp "$DEST/shots/" 2>/dev/null || true
cp -n "$SRC/graphics/tiles/HD/Shots/"*.png "$DEST/shots/" 2>/dev/null || true

cp -n "$SRC/audio/"*.wav "$DEST/audio/" 2>/dev/null || true

echo "Copied:"
echo "  outcomes: $(ls "$DEST/outcomes" | wc -l | tr -d ' ')"
echo "  shots:    $(ls "$DEST/shots" | wc -l | tr -d ' ')"
echo "  audio:    $(ls "$DEST/audio" | wc -l | tr -d ' ')"

bun tools/gen-cc2014-assets.mjs

echo
echo "These files are git-ignored and must not be pushed or published."