# Captain's Call 2027 — GOAL SYSTEM + Build Plan

> Working agreement: work **top-to-bottom**. Do not start a milestone whose
> dependencies in the previous milestone are red. Every engine change must keep
> the regression corpus + determinism tests green.
> Spec: `specs/requirement.md`. Decisions locked: React, MIT, fictional DB, mods allowed.

## GOAL LEVELS (finish each fully before the next)

| Level | Name | Done means |
|-------|------|-----------|
| **G0** | First Boot | App runs. Loads fictional 2027 world (12 nations, ~180 players). You can: start career, pick team, see squad, play **one** match, export scorecard HTML. |
| **G1** | Playable Slice | Full domestic T20 + 50-over league seasons (round-robin), squad selection with saved XIs, ball-by-ball match play with canvas + orders, live tables, news, simple calendar, infinite save/load. **This is "one proper playable version".** |
| **G2** | Career Management | Budgets, transfers/bidding, U19/2nd XI, training, retirement/regen, forms + fatigue, international FTP + World Cup/World T20/Champs Trophy (World Challenge), franchise auction (IPL/BPL-style), rankings. |
| **G3** | Phase 1 Parity + Phase 2 start | Editors, almanac, minor counties, assistant delegation, records, all FRs in `specs/requirement.md` §8, then modern UX/AI-coach work. |

## G0 ACHIEVED (2026-10-09)

Playable vertical slice running on the desktop: `npm run tauri:dev`.

- 12 fictional nations / 37 teams / 300 players / 12 competitions, embedded DB `2027.01`
- Start any fixture, pick format + ground, watch the 2D overhead canvas view
- Ball-by-ball + over/session/innings/to-result sim controls with live scorebug
- In-match orders (aggression, field presets, keep strike, hit & run, bowler change)
- Batting/bowling/over/commentary tabs, HTML scorecard export
- 11 Rust tests green (determinism, scoring bands, full match flow, scorecard, trademark check)

## G1 PLAYABLE SLICE ACHIEVED (2026-10-09)

`npm run tauri:dev` — a full career loop, not just single matches:

- **New career** → pick a fictional domestic side → season fixture list generated (circle-method round robin)
- **Career home** → next-match card, fixtures, live table, news feed, squad, save/load/delete
- **Play your fixtures** ball-by-ball on the 2D canvas with orders (aggression, field presets, keep strike, hit & run, bowler change)
- **Other fixtures auto-simulate** on day advance; results, table and news update
- **Season completes** when every fixture is played; final table shown
- **Saves** in SQLite under the app data dir, pinned to dbVersion
- 25 Rust tests green (engine, scheduler, saves, full season simulation, determinism, scorecard, trademark policy)

## ENGINE ↔ UI CONTRACT (frozen for G1; changes need spec bump)

Rust commands (`src-tauri/src/commands/*`):

```rust
get_world() -> World;                              // nations, teams, players (fictional)
start_career(teamId: u32, dbVersion: String) -> CareerState;
get_career() -> CareerState;                      // loaded active career
list_fixtures(careerId) -> Fixture[];             // next 14 days
get_squad(teamId) -> Player[];                    // full squad w/ skills
auto_select(squadId, Format) -> TeamSelection;    // CPU selection
set_team_selection(squadId, Format, selection);
get_match(matchId) -> MatchSnapshot;
submit_match_input(matchId, MatchInput) -> MatchSnapshot;  // bat/bowl + orders
sim_match_to(matchId, phase) -> MatchSnapshot;    // ball | over | innings | end
export_scorecard(matchId) -> String;              // HTML path
save_career() / load_career(id) -> CareerState;
list_saves() / delete_save(id);
```

Types: `World`, `Nation`, `Team`, `Player`, `Ground`, `Competition`,
`CareerState`, `Fixture`, `MatchSnapshot`, `Innings`, `BattingCard`,
`BowlingCard`, `TeamSelection`, `MatchInput`, `Format` {Test, Odi50, T20}.

## M0 — Foundations

- [x] P0.1 Repo hygiene + Tauri scaffold (React+TS+Vite, Rust core, updater wired, icons)
- [x] P0.2 Framework locked: React + Vite + Canvas 2D
- [x] P0.3 `data/schema/world.schema.json` + `tools/validate-world.mjs` validator
- [x] P0.4 Seeded RNG (xoshiro256**) + stats + determinism tests
- [x] P0.5 Save format v1: SQLite (WAL, dbVersion pin) + round-trip/update tests

## M1 — Match Engine (G1 core)

- [x] P1.1 Ball outcome model with format anchors; distribution + bounds tests (`distribution_is_sane`)
- [x] P1.2 White-ball overlays + powerplay + chase-pressure curves (calibrated in tests)
- [x] P1.3 Dismissals (7 kinds), extras, strike rotation, keep-strike / hit-and-run orders
- [x] P1.4 Bowling rotation guard, 5 field presets, target-bowler override
- [ ] P1.5 Declarations / follow-on / overs limits / DLS-lite / light
- [ ] P1.6 Minnow weighting for uneven teams
- [x] P1.7 Sim speeds ball/over(session)/innings/to-result wired to the UI
- [ ] P1.8 Regression corpus (200 reference matches, green) + determinism guard

## M2 — World & DB (G0 data)

- [x] P2.1 `tools/generate-world.mjs` → embedded versioned `data/world.json` (dbVersion 2027.01)
- [x] P2.2 Fictional content: 12 nations, 25 domestic sides, 300 players, grounds, 12 competitions
- [x] P2.3 Deterministic per-nation name pools + business-rule validator

## M3 — Saves & Career Loop (G1 shell)

- [x] P3.1 SQLite save store (WAL, indexed) + save/load/delete commands
- [x] P3.2 Round-robin scheduler (circle method, odd-count byes) + knockout brackets (knockout not yet wired to UI)
- [x] P3.3 CareerState, fixtures, table, news feed (rankings snapshot pending)
- [x] P3.4 Save/load/delete commands (autosave on day advance pending)

## M4 — UI (G1 playable)

- [x] P4.1 React shell: setup → career → match screens (Squad/Tables/News/Saves tabs)
- [x] P4.2 Selection screen: auto-pick + saved XIs, keyboard-accessible reordering/drop, composition hints (keepers/bowling)
- [x] P4.3 Canvas 2D ground view (variable pitch, fielders, day/night) + scorebug
- [x] P4.4 Orders panel + ball/over/session/end-innings/to-result controls
- [ ] P4.5 Fixtures/tables screens + live widget
- [ ] P4.6 HTML scorecard export done; end-of-season summary screen pending

## M5 — Deep systems (toward G2)

- [ ] P5.1 Budgets, bidding (typeable value), loans, releases
- [ ] P5.2 U19/2nd XI pipeline + training + form/fitness/injuries + retirements/regen
- [ ] P5.3 International FTP + World Challenge tournaments + rankings
- [ ] P5.4 Franchise auction (IPL/BPL-style genericised) + assistant delegation

## M6 — Update infra & release

- [ ] P6.1 DB manifest + ed25519 signing + stable/beta channels
- [ ] P6.2 In-app check → backup → apply → rollback (offline-first)
- [ ] P6.3 Community upload pipeline (CI validate → review → publish)
- [ ] P6.4 Tauri updater enabled + Win/Mac/Linux bundles
- [ ] P6.5 Save pins dbVersion + old-save migration

## M7 — Acceptance (G3 exit)

- [ ] P7.1 Perf benchmarks (specs §6 NFR-1/2/3) recorded
- [ ] P7.2 Clean-machine install test (Win/Mac/Linux)
- [ ] P7.3 Tag `v0.1-playable` → G2/G3 roadmap continues in `tasks.plan.md` only

## AI workflow rules

1. One plan box = one branch/commit (`m1-1-ball-model` etc.).
2. Engine PRs must include tests; no test = no merge.
3. Contract changes: bump `specs/requirement.md` + this file first, code second.
4. No 3D assets, no real names, no trademarked league names in shipped DB.
