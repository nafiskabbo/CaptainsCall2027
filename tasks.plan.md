# Captain's Call 2027 — Build Plan (feature-by-feature tracker)

> Check off each box as you + AI complete it. Order matters: engine → data → UI → polish.
> Spec: `specs/requirement.md`. Update this file per milestone.

## M0 — Foundations (week 1)

- [x] P0.1 Repo hygiene: `README.md`, `LICENSE`, `CONTRIBUTING.md`, `.gitignore` (Rust+Node), Tauri scaffold (React+TS+Vite frontend, Rust core stub, updater plugin wired inactive, placeholder icon set generated)
- [x] P0.2 Framework locked: React + Vite + Canvas 2D (decided 2026-10-09)
- [ ] P0.3 `data/schema/` JSON Schemas (player, team, competition, ground)
- [ ] P0.4 Deterministic RNG + seed plumbing in Rust core
- [ ] P0.5 Save format v1 (atomic write, dbVersion pin) + save/load round-trip test

## M1 — Match Engine Core (parity heart)

- [ ] P1.1 Ball outcome model (red-ball base): bat/bowl/defence/pitch/weather inputs
- [ ] P1.2 White-ball overlays: ODI + T20 distinct bowling skills, PP, pressure curves
- [ ] P1.3 Dismissals, extras, strike rotation, keep-strike / hit-and-run orders
- [ ] P1.4 Bowling AI: rotation guard, no weak-bowler death overs, field presets
- [ ] P1.5 Declarations, follow-on, DLS-lite, rain/light, day/night factor
- [ ] P1.6 Minnow weighting + rankings engine rewrite
- [ ] P1.7 Regression corpus: 200 reference matches green + benchmark harness

## M2 — Competitions & Scheduling

- [ ] P2.1 Domestic: Championship / One-Day / T20 + points tables
- [ ] P2.2 2nd XI + U19 + Minor Counties (toggleable per country)
- [ ] P2.3 International FTP 4-yr + tour negotiator + friendlies
- [ ] P2.4 Tournaments: World Cup / World T20 / Champions Trophy / Custom / Classic / All-Time Greats
- [ ] P2.5 Calendar view + live-games widget

## M3 — Squad, Growth & Management

- [ ] P3.1 Selection screen: hints/biases, saved XIs, preferred/reserve slots, captains per format
- [ ] P3.2 Search everywhere (players/teams/comps/series/matches) + quick search
- [ ] P3.3 Training, form/fitness/morale, injuries, workload guards
- [ ] P3.4 Youth intake → contracts, staged retirements, regens
- [ ] P3.5 Budgets + bidding (typeable values) + loans/releases
- [ ] P3.6 Franchise auction (purse, overseas cap, RTM)
- [ ] P3.7 Assistant delegation presets (2 spinners / extra bat / pace attack, youth auto)

## M4 — 2D Match View + UX

- [ ] P4.1 Canvas overhead view: variable grounds, village→stadium tiers, outcome icons
- [ ] P4.2 Broadcaster score bug + live career-stats panel
- [ ] P4.3 Sim-speed controls (ball → instant) + return-to-menu-after-match
- [ ] P4.4 Readability pass: text scaling, keyboard nav, colour-blind palettes
- [ ] P4.5 News feed depth + records/stats screens

## M5 — 2027 Data + Editors

- [ ] P5.1 Compile 2027 DB: 6000+ players, squads, stats, records, grounds, tours
- [ ] P5.2 Built-in editors (players/teams/comps/grounds/tours) + validation
- [ ] P5.3 Player-name blocklist / opt-out flow
- [ ] P5.4 HTML scorecard + almanac export

## M6 — Update & Release Infra

- [ ] P6.1 DB manifest + ed25519 signing + stable/beta channels
- [ ] P6.2 In-app check → backup → apply → rollback flow (offline-first, opt-out)
- [ ] P6.3 Community upload pipeline: export → CI validate → review → publish
- [ ] P6.4 Tauri updater + cross-platform builds (Win/Mac/Linux) + installer size check
- [ ] P6.5 Saves pin dbVersion; migration test old-save → new-DB

## M7 — Phase 1 Acceptance

- [ ] P7.1 Full FR-1..FR-33 pass, perf targets (specs §6) measured + recorded
- [ ] P7.2 Clean-machine install test on all 3 OSes
- [ ] P7.3 Tag `v0.1-parity`, then open Phase 2 (modern UX/AI coach) roadmap

## How to use with AI

1. Pick one box, say "implement P1.1 per specs/requirement.md".
2. AI implements + adds tests; you tick the box + note commit hash.
3. Never start M(n+1) core items if M(n) regression corpus is red.
