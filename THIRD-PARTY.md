# Third-party material

## Cricket Coach 2014 assets

`src/assets/cc2014/` contains artwork and audio copied from a retail copy of
**Cricket Coach 2014** (Trade Derby Works / Big Ant Studios, 2014), installed
at `old-game/` (which is git-ignored and never committed).

**These files are copyrighted and are not licensed for redistribution.** They
are present because this is a private, personal-use rebuild of a game the owner
holds. Consequences, stated plainly so they are not a surprise later:

- Do not push this directory to a public remote.
- Do not publish a build that includes it.
- Do not relicense the project in a way that covers them.

The `NOTICE` file inside `src/assets/cc2014/` repeats this at the point of use.

### What is there

| Path | Contents | Used for |
|---|---|---|
| `outcomes/` | Outcome icons (`O_1`, `O_4`, `O_W`, …) | The over grid and shot picker |
| `shots/` | Stroke sprites (`SHOT_STRDRV`, `SHOT_PULL`, …) | The shot picker |
| `audio/` | Crowd and bat sounds (`sn_*.wav`) | Match ambience |

Sprites come in normal and `HD/` sizes; the `HD` set is preferred at runtime.

### What was deliberately not taken

- **Commentary prose.** `bbbc_*.csv` lines are the author's text. The extractor
  reads their tags and counts them; it never copies the words. See
  `tools/extract-cc2014-model.mjs`.
- **The database.** `db/*.dbj` are the commercial player and ground databases.
  This project ships its own generated world.

### If this ever needs to be public

Delete `src/assets/cc2014/` and the loader in `src/lib/cc2014-assets.ts`. The
game renders without it — the previous hand-drawn and synthesised assets were
replaced by these, so expect the match screen to look worse, not to break.

## Other

- `misc/font.ttf` and the `misc/names/` CSVs in `old-game/` are also the
  original's and are not used.
- Everything else in this repository is original work for this project.