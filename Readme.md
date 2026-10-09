# Captain's Call 2027

> A 2D-only cricket management simulation, inspired by **Cricket Coach 2014** by Oli Norwell / Rockingham Software. Built with AI, for performance.

**Credit:** Original concept, match-engine depth, and management design by [Cricket Coach](https://www.cricketcoachgame.com/). Captain's Call 2027 is an unofficial, open-source tribute — not affiliated with or endorsed by the original authors. If you love this genre, buy Cricket Coach 2014 to support the original developer.

## Objective

1. **Phase 1 — Parity:** Recreate every core feature of Cricket Coach 2014, 2D only.
2. **Phase 2 — Modernise:** Better engine, better UX, 2027 rosters, community DB + auto-update.

No 3D. No bloat. Fast sim, fast UI, playable on low-end laptops.

## Why Tauri? (opinion)

**Yes, use Tauri v2.** Short version:

| Option | Verdict |
|--------|---------|
| **Tauri v2 (Rust core + Web UI + Canvas 2D) — Recommended** | Tiny binary (~10MB), native perf, built-in auto-updater, Win/Mac/Linux from one codebase. Perfect for a UI-heavy management sim. |
| Electron | Same UI, 5–10x bigger, more RAM. No benefit here. |
| Unity / Godot | Overkill for 2D text+dots sim. Slower iteration for AI-driven UI work, heavier builds. |
| Native (C#/Qt) | Fast but 3x UI work for cross-platform. |

Proposed split:

- **Rust core (`src-core/`):** match engine, sim, ratings, scheduling, save format, SQLite access. Deterministic + seeded RNG so results are reproducible and testable.
- **Frontend (`src/`):** React + TypeScript + Vite + Canvas 2D overhead view (village green → stadium, day/night). No heavy extras.
- **DB (`data/`):** SQLite + versioned JSON exports. 2027 rosters.
- **Updater:** Tauri built-in updater for app + separate DB channel for roster updates (see `specs/requirement.md` §7).

This stack is ideal for AI-only development: Rust core has strict types/tests, frontend is simple components + canvas, everything is spec-testable without manual 3D work.

## Key Features (Phase 1 parity)

- Careers: England County, Australia State, IPL-style Franchise + Auction, International-only.
- Competitions: Tests/Championship, ODIs, T20s, World Cup / World T20 / Champions Trophy, tours + FTP, friendlies.
- 2D overhead match view, ball-by-ball + session/day sim speed controls.
- Tactics: batting aggression, keep-strike, hit-and-run, bowling line/length, fields, bowler rotation, declarations.
- Squad: selection hints, saved selections, preferred/reserve batting positions, separate captains per format.
- Development: U19 → 2nd XI → First team, training, form/fitness/morale, staged retirements, skills with potential (bat/bowl/field/leadership/defence, split LO skills).
- Management: assistant delegation, transfers/auctions, budgets, news, rankings, records, almanac HTML export.
- Editors: players, teams, competitions, DB enable/disable 2nd XI & U19 per country.

## What Modernises in Phase 2

- 2027 database (not 2014). Community roster uploads + signed auto-update feed.
- Faster sim (multi-threaded Rust, instant holiday-season sim).
- Cleaner readable GUI, search everywhere, live-games widget, calendar view.
- Deterministic engine + regression test corpus so AI changes don't break realism.

## Docs

- `specs/requirement.md` — full requirements / SRS.
- `tasks.plan.md` — feature-by-feature build tracker.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` — open-source basics.

## 2027 Rosters + Updates

- Ship DB is a **2027-style structure with fictional names** (players, clubs, leagues) to avoid trademark/publicity issues — e.g. generic "Indian T20 League", not "IPL". Real-name packs are community mods, installed via the update channel, clearly labelled third-party.
- Roster format is plain versioned JSON/CSV in `data/` so anyone can PR an update.
- App checks a signed feed on launch (opt-out, offline-first) and offers one-click DB update. Community uploads go through validation + review before signing.

## Performance Rules

- 2D Canvas only, no WebGL/3D deps.
- Season sim must stay interactive; full holiday sim < seconds on modest hardware (target defined in specs).
- SQLite indexed; no N+1 queries; engine has zero UI blocking (worker / Rust async).

## Run it

```bash
npm install
npm run tauri:dev     # desktop app (Rust core + React UI)
npm run tauri:build   # release bundle: .app + .dmg on macOS
```

Release artifacts land in `src-tauri/target/release/bundle/` (≈17 MiB app,
≈5 MiB dmg — inside the <25MB target in specs NFR-2).

Useful scripts:

```bash
npm run generate:world   # regenerate the fictional 2027 database
npm run validate:world   # validate data/world.json against data/schema
npm test                # TypeScript check
cargo test --manifest-path src-tauri/Cargo.toml   # engine + gameplay tests
```

## Architecture

- `src-tauri/src/core/` — world types + deterministic RNG.
- `src-tauri/src/match_/` — ball resolution, innings/match state, Test follow-on logic.
- `src-tauri/src/commands/` — the UI command surface (`get_world`, `start_match`, `sim_match`, `sim_innings`, `export_scorecard`).
- `src/screens/` — React screens (setup, 2D match view, controls, scorecards).

## Status

**G0 and G1 playable** — `npm run tauri:dev` gives you the full loop: pick a fictional
domestic side, generate the season, play your fixtures ball-by-ball with the 2D overhead
view and in-match orders, auto-sim the rest of the competition, watch the table and news
update, save/load careers.

Next up (G2 career management): squad selection UI, budgets/bidding, U19/2nd XI,
training/retirements, international FTP + World Challenge, franchise auction.
See `tasks.plan.md`.

## License

MIT — see `LICENSE`. Original game assets/names/stats belong to their owners; our DB uses fictional player names only (see `data/README.md`).
