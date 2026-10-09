# Captain's Call 2027 — Requirements (SRS)

Version: 0.1.0 (draft) | Date: 2026-10-09
Status: Pre-implementation. Phase 1 = Cricket Coach 2014 parity. Phase 2 = modernise.

## 0. Attribution

Inspired by **Cricket Coach 2014** (Oli Norwell / Rockingham Software, in dev since 1998). All design credit for the original management-sim concept goes to the original. This project copies **functionality/behaviour**, not assets, code, or proprietary data. No affiliation/endorsement.

## 1. Goals & Non-Goals

### Goals
1. Exact functional parity with Cricket Coach 2014, 2D only.
2. Better engine: deterministic, faster, testable, realistic results (including minnows weighting).
3. 2027 player database (ships current, not 2014).
4. Community roster upload + signed internet auto-update.
5. Best-in-class performance on low-end hardware.
6. AI-only buildable: small modules, typed interfaces, regression corpus.

### Non-Goals
- No 3D graphics, no motion capture, no live multiplayer match play (v1).
- No copying of original binaries, art, or paid database.
- No mobile native v1 (Tauri mobile later; desktop Win/Mac/Linux first).

## 2. Tech Stack (decided)

- **Tauri v2**: Rust backend + webview frontend. Auto-updater + bundler built in.
- **Core sim in Rust** (`src-tauri/core/`): match engine, scheduler, ratings, injuries, retirements, rankings, save/load. Seeded RNG (`StdRng`/`WyRand`-class), fixed-point where needed for determinism.
- **Frontend in TypeScript + React + Vite + Canvas 2D**: management UI + overhead 2D match view (variable ground sizes, day/night palette, outcome icons). Locked per reviewer 2026-10-09.
- **Storage**: SQLite (rusqlite/SQLx) for local saves + DB; JSON/CSV in `data/` as human-editable source of truth, compiled to SQLite at build/import.
- **No ORM bloat, no Electron, no game-engine dependency.**

Why: management sims are DB + rules + text UI. Tauri gives ~10MB binary, native speed sim, web-speed UI iteration — ideal for AI generation and low-end perf.

## 3. Functional Requirements

### 3.1 Game Modes
- FR-1 New Career: English County, Australian State/T20, Franchise League (IPL-style + BPL-style + auction), International-only.
- FR-2 Tournament modes: One-off World Cup, World T20, Champions Trophy (World Challenge), Custom series (1/3/5-match Tests, ODI, T20), Classic series replay, All-Time Greats.
- FR-3 Internet/database selection at new-game: choose bundled 2027 DB or imported community DB.
- FR-4 Trial/demo mode (2-day limited) optional.

### 3.2 Competitions & Scheduling
- FR-5 Domestic: County Championship (multi-day), One-Day league, T20 league; Minor Counties / 2nd XI / U19 fixtures per country (toggleable).
- FR-6 International: FTP 4-year round-robin, tour negotiator (add/arrange tours + warm-ups), real 2027 grounds per fixture.
- FR-7 Friendlies in any format for form recovery.
- FR-8 Day-by-day calendar view; live-games widget (all in-progress scores).

### 3.3 Match Engine (2D only)
- FR-9 Ball-by-ball sim with speeds: ball, over, session, innings, day, instant result. Skip = weaker outcome fidelity (documented, as original).
- FR-10 Overhead 2D canvas: variable boundary sizes, village→stadium art tiers, day/night, broadcaster-style score bug, wagon-wheel dots, outcome icons.
- FR-11 Rules per format: Tests (declarations, follow-on), ODI (overs, PP, DLS-lite), T20 (PP, SR pressure). Weather/pitch deterioration + light.
- FR-12 Minnow weighting so associates produce realistic upsets without breaking rankings.
- FR-13 Determinism: same seed + inputs = same result. Regression corpus of ~200 reference matches.

### 3.4 Tactics & Captaincy (in-match orders)
- FR-14 Batting: aggression slider per batter, keep-strike, hit-and-run, farm-strike, declare/close timing.
- FR-15 Bowling: line/length target, pace/spin rotation guard (no weak-bowler death overs unless forced), field presets + custom, enforce follow-on logic.
- FR-16 Captain per format + vice; leadership affects morale/composure.
- FR-17 Assistant auto-pilot: pick 2 spinners / extra bat / pace attack presets; delegate youth/2nd XI.

### 3.5 Squad & Selection
- FR-18 Player skills: batting, bowling (separate red-ball / ODI / T20 ratings), defence, fielding, leadership, each with current + potential. Form, fitness, morale, age, preferred + reserve batting slots, T20 preferred slot.
- FR-19 Selection screen: hints/biases per board, saved/loadable XIs, eligibility + fatigue greying, quick search + revamped search (players/teams/comps/series/matches).
- FR-20 Live career stats on match screen (innings-average style panel).

### 3.6 Development & Careers
- FR-21 U19 → 2nd XI → senior pipeline; yearly intake; contracts/promotions.
- FR-22 Training focus per player; skills grow toward potential; fielding/leadership grow over time.
- FR-23 Staged retirements (per-format), comebacks rare; regen intake balanced by nation strength.
- FR-24 Injuries, suspensions, workload (pace-bowler guards).

### 3.7 Economy & Transfers
- FR-25 Budgets per club; bidding (manual +/- with typeable value — fix original click-only pain), loans, releases.
- FR-26 Franchise auction: purse, RTM/marquee rules configurable, overseas cap. Covers IPL-style and BPL-style leagues as genericised competitions (configurable teams, purse, slots).

### 3.8 Stats, History, News
- FR-27 Career stats for 6000+ players; records (team/player) updated to 2027 start; international rankings engine (minnow-aware).
- FR-28 News feed with depth (transfers, injuries, milestones, board pressure).
- FR-29 Scorecards + 5000-page-style HTML almanac export; HTML output for sharing.

### 3.9 Editors & Customisation
- FR-30 Built-in editors: players, teams, competitions, grounds, tours. Enable/disable 2nd XI & U19 per country.
- FR-31 Validation on import: schema check, rating bounds, duplicate detection, nation caps.

### 3.10 Saves & Options
- FR-32 Save/load multiple slots, autosave per week, return-to-menu-after-match, holiday-until-date.
- FR-33 Options: sim speed defaults, sound on/off, text size/readability mode, DB channel (stable/beta/offline).

## 4. 2027 Database Requirement

Decided 2026-10-09: MIT for code; **fictionalised defaults** in the shipped DB; React for UI.

- DR-1 Ship DB = 2027-style season structure (squads, transfers, career stats, records, tour schedule, grounds) with **fictional player names and generic competition/team names** (e.g. "Indian T20 League", not "IPL"; city-based fictional franchises, not real clubs). No real league/team names, logos, or likenesses in the default ship.
- DR-2 Source format: `data/players/*.json`, `data/teams/*.json`, `data/competitions/*.json`, `data/grounds/*.json`, JSON Schema in `data/schema/`.
- DR-3 No 2014 defaults. Any legacy import must be explicitly labelled and migrated.
- DR-4 Real-names packs are **community mods, never bundled**: installed via the DB channel / local-file import (see §5). Pack publishers own their legal risk; app shows a "third-party content" notice on install.
- DR-5 Name/opt-out: player-name blocklist respected in all first-party content; takedown PR process for removals.
- DR-6 Why not "weird misspellings": deliberately misspelled real names are still identifiable (same publicity/trademark risk) while looking cheap and breaking search. Either fully fictional (default) or accurate + opt-out (mod). No middle ground.

Rationale (not legal advice): short names/facts aren't copyrightable, but **league/club names are trademarks** (IPL, BPL, team brands) and **real-person names carry right-of-publicity risk** varying by country. Fictional defaults keep the repo + binaries safe; the mod channel gives players realism without putting the project at risk.

## 5. Roster Upload & Auto-Update System

- UR-1 Offline-first: game fully playable with zero network.
- UR-2 Channels: `stable` (signed, default), `beta` (opt-in), `local file` (manual import).
- UR-3 Flow: app checks `https://<feed>/db/manifest.json` on launch (if enabled) → compare `dbVersion` → prompt → download delta/full SQLite + signature → verify (ed25519/minisign) → backup current → apply → changelog shown.
- UR-4 Community upload: PR to `data/` **or** in-app export → upload endpoint → CI validates (schema, bounds, sim smoke-test of N matches) → human review → sign + publish.
- UR-5 Rollback: one-click revert to previous DB version; saves pin the `dbVersion` they started with.
- UR-6 App updates via Tauri updater (separate from DB channel), silent-check + user-consent install.
- UR-7 Privacy: update check sends only app version + dbVersion + platform; no tracking; opt-out flag.

## 6. Non-Functional Requirements

- NFR-1 Perf: cold start < 2s on 5-yr-old laptop; screen nav < 100ms; full-season holiday sim < 5s domestic / < 15s full-world on reference hardware (to be benchmarked in Milestone M2).
- NFR-2 Size: installer < 25MB desktop; DB separate download < 50MB.
- NFR-3 2D only: Canvas 2D, no WebGL/three.js. 60fps dots animation on integrated GPU.
- NFR-4 Reliability: no save corruption (atomic writes + journal); crash never loses more than 1 week sim.
- NFR-5 Accessibility: scalable text, keyboard nav for all selection/match controls, colour-blind-safe palettes.
- NFR-6 Cross-platform: Win10+, macOS 12+, Linux (deb/AppImage). Same save format everywhere.
- NFR-7 Testability: engine unit tests >80% on rules; sim regression corpus must pass before merge.

## 7. Architecture (target)

```
src/                 # TS UI (screens, Canvas 2D view, stores)
src-tauri/core/      # Rust: engine, sim, scheduler, ratings, rankings
src-tauri/db/        # SQLite layer, migrations, DB version pinning
data/                # JSON source DB + schema + changelog
tools/               # validators, DB compiler, almanac exporter
tests/               # engine regression corpus, sim benchmarks
```

- UI ↔ Rust via Tauri `invoke` (commands) + events for live scores. No direct SQL from UI.
- Match tick never blocks UI: Rust async / web-worker bridge, batched events.

## 8. Acceptance Criteria (Phase 1 done = all true)

- [ ] All FR-1..FR-33 manually testable via `tasks.plan.md` checklist.
- [ ] Regression corpus green; benchmark targets met.
- [ ] 2027 DB loads with 6000+ players, zero schema errors.
- [ ] DB update check → install → rollback works offline-tested.
- [ ] HTML scorecard + almanac export opens standalone.
- [ ] Win + Mac + Linux builds install and run from clean machine.

## 9. Decisions (locked 2026-10-09)

1. Licence: **MIT** for code. `data/` fictional DB ships under the same repo licence; third-party real-name mods are publisher-responsible.
2. Names: **fictional defaults** (players + leagues + clubs). Real-name packs via mod channel only. No misspelled-parody names.
3. Scope v0.1: Minor Counties + full FTP **included, behind per-country toggles** (as FR-30). Franchise coverage: IPL-style + BPL-style genericised leagues with auctions.
4. Framework: **React** + Vite + Canvas 2D.
