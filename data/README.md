# `data/` — versioned databases (source of truth)

## Policy (locked 2026-10-09, see `specs/requirement.md` §4)

- The **default ship is fully fictional**: invented players, clubs, and generic
  league names ("Indian T20 Carnival", not "IPL"). No real names, trademarks,
  or logos in first-party content. This is what the game boots into.
- Real-world likeness packs are **opt-in test databases**, labelled as
  third-party content in the UI.
- Takedown/blocklist requests are honoured — see `CONTRIBUTING.md`.

## Layout

```
data/
  schema/                # JSON Schemas: world, player, team, competition, ground
  world.json             # the fictional default world (compiled into the binary)
  updates/               # signed DB manifests for the update channel
  packs/
    real-international/  # opt-in real-names pack (see below)
```

## Database packs

A **pack** is a complete alternative world the player can select at new-game.
Packs are compiled into the binary alongside the default, so every one works
fully offline and needs no download.

| Key | Contents | Names |
|-----|----------|-------|
| `default` | 12 invented nations, 37 clubs, 300 players | Fictional |
| `real-international` | The 10 ICC full members, 148 players, 60 grounds | **Real** |

### The real-names pack

`tools/generate-intl-db.mjs` builds it deterministically:

```bash
node tools/generate-intl-db.mjs          # writes data/packs/real-international/world.json
node tools/generate-intl-db.mjs --seed 12345 --out /tmp/alt.json
```

It carries its own `db_version` (`2027.10-real`), which saves pin, so a career
started against one database can never be silently mixed with another.

**This is a test database.** Real player names carry right-of-publicity risk
varying by country, and competition/team names are trademarks. The pack is
therefore:

- never the default the game boots into;
- labelled "Test database" with the caveat spelled out in the UI;
- not treated as first-party content by the name/trademark validator.

Squad ratings approximate current ability so the engine produces plausible
results. They are hand-written, not scraped from any official source.

The generator self-checks referential integrity, skill ranges, XI feasibility
and per-nation name uniqueness before writing, and fails loudly rather than
emitting a broken pack.

## Adding a pack

1. Add a `PACKS` entry in `src-tauri/src/world.rs` with an `include_str!` of the
   pack JSON, plus a `PackId` variant carrying its label and description.
2. Run the validator: `bun run validate:world` (default) and the generator's
   own self-check for packs.
3. `cargo test` includes `every_bundled_pack_is_a_playable_world`, which fails if
   any pack cannot field an XI for every side.