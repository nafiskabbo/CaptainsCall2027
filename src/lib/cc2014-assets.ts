//! Resolves the copied Cricket Coach 2014 artwork.
//!
//! The sprites in `src/assets/cc2014/` are the original's and are not licensed
//! for redistribution — see THIRD-PARTY.md. They are here because this is a
//! private rebuild of a game the owner holds. Everything degrades gracefully:
//! a missing asset returns "" and the caller falls back, so deleting the
//! directory breaks nothing structurally.
//!
//! Two sprite sets shipped in the original. The `.bmp` set is complete and
//! carries the mirrored `_L` variants the game uses for left-handers; the HD
//! `.png` set covers only some strokes. So `.bmp` is the source of truth and
//! the PNG is a fallback, not the other way round.
//!
//! The asset map is generated (see `tools/gen-cc2014-assets.mjs`) rather than
//! globbed, because the UI e2e harness bundles to IIFE where `import.meta` is
//! empty.

import { STROKE_SPRITE, type Stroke } from "../types";
import { CC2014_ASSETS } from "./cc2014-assets.generated";

/**
 * The sprite for a stroke.
 *
 * `leftHanded` matters: the original ships `_L` sprites for the strokes whose
 * shape is not symmetric, and showing a right-hander's cover drive to a
 * left-hander would be visibly wrong.
 */
export function strokeSprite(stroke: Stroke, leftHanded = false): string {
  const base = STROKE_SPRITE[stroke];
  const candidates = leftHanded
    ? [`${base}_L.bmp`, `${base}_L.png`, `${base}.bmp`, `${base}.png`]
    : [`${base}.bmp`, `${base}.png`];
  for (const name of candidates) {
    const hit = CC2014_ASSETS[name];
    if (hit) return hit;
  }
  return "";
}

/**
 * An outcome icon. The original has no generic "W" — it shows *how* a batter
 * was dismissed (bowled, lbw, caught, run out, stumped), which is a detail
 * worth keeping rather than flattening into one red chip.
 */
export function outcomeIcon(key: string): string {
  return CC2014_ASSETS[`${key}.png`] ?? "";
}