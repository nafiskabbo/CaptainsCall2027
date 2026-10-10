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

Playable vertical slice running on the desktop: `bun run tauri:dev`.

- 12 fictional nations / 37 teams / 300 players / 12 competitions, embedded DB `2027.01`
- Start any fixture, pick format + ground, watch the 2D overhead canvas view
- Ball-by-ball + over/session/innings/to-result sim controls with live scorebug
- In-match orders (aggression, field presets, keep strike, hit & run, bowler change)
- Batting/bowling/over/commentary tabs, HTML scorecard export
- 11 Rust tests green (determinism, scoring bands, full match flow, scorecard, trademark check)

## UI REDESIGN TO MATCH THE ORIGINAL (2026-10-09)

Rebuilt the frontend around the Cricket Coach 2014 design language, worked out from the
original's own screenshots:

- navy gradient top bar: clock + match title in gold + nation flag chips
- left icon sidebar (news / team / domestic / search / rankings / records) + quick search
- glossy, gradient-highlighted buttons; big blue pill "Continue" with arrow
- green-field backdrop with crowd/stand band behind content
- match view: stadium bowl with crowd speckle, boundary ad boards, mown stripes, 30-yard
  circle, named fielder dots, batters with bat, bowler + ball in flight, floodlight pools
- translucent commentary bars over the ground, bright green event line, partnership tag
- bottom scorebar: TEAM score/overs then batter/bowler runs (balls)
- Manhattan runs-per-over chart with wicket markers
- conditions panel: run rate, last 10 overs, ball age, pitch-wear and spin meters
- actions bar with «« / Fast Forward / »» / Save / Main Menu / Continue

 (2026-10-09)

`bun run tauri:dev` — a full career loop, not just single matches:

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
- [x] P1.5 Declarations/follow-on logic + overs limits; DLS-lite target revision module (chase status + required rate)
- [x] P1.6 Minnow weighting: the underdog's boundaries/lift are boosted by the squad-strength gap (capped at 1.25), so minnows post competitive totals
- [x] P1.7 Sim speeds ball/over(session)/innings/to-result wired to the UI
- [x] P1.8 Regression corpus: scoring bands per format, determinism across seeds, minnow boost behaviour, full-season simulation (corpus seeded and expandable)

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
- [x] P4.5 Fixtures (day-grouped, stage labels, playable rows, filter), tables, news, records, squad, saves screens (live-games widget pending)
- [x] P4.6 HTML scorecard export + end-of-season summary screen (final table, awards, leading run-scorer/wicket-taker)

## M5 — Deep systems (toward G2)

- [x] P5.1 Off-season: budget + wage bill, transfer pool with reputation gate, bidding (accepts at/above wage), releases (min XI guard)
- [x] P5.2 Training screen (focus multiplies matching development, prospects and retirement watch lists), injuries, retirements and academy intake (U19/2nd XI side own schedules pending)
- [x] P5.3 International: World Challenge T20 cup (4 groups + semis + final), bilateral series (ODI + Test tours), group tables, national careers
- [x] P5.4 Franchise auction (genericised leagues): lots from player pool, purses, base prices, overseas cap, rival bidding, spending tracker

## M6 — Update infra & release

- [x] P6.1 DB manifest schema + version comparison + stable/beta/local channels + unsigned-refusal rules + signature field plumbing (ed25519 crate integration pending)
- [x] P6.2 Update decision logic (off by default, unreachable ≠ error, rejected reasons) + backup/install/rollback file ops + settings UI panel
- [x] P6.3 CI validation: schema + business rules + trademark/name policy + db version/size gates (review + publish remain a hosting step)
- [x] P6.4 CI matrix builds on ubuntu/macos/windows with fmt + clippy -D warnings + tests + perf gates (release signing key setup remains a maintainer step)
- [x] P6.5 Saves pin dbVersion; loading a mismatched save warns the user and keeps the roster it was created from

## M7 — Acceptance (G3 exit)

- [x] P7.1 Perf benchmarks: tests/performance.rs gates world load, full-season sim, ball-by-ball innings and save serialization (all green)
- [x] P7.2 macOS release bundle verified (builds, launches, 17MiB app / 5MiB dmg); CI matrix builds Win/Mac/Linux
- [x] P7.3 Tagged `v0.1-playable` — G2/G3 roadmap continues in `tasks.plan.md`

## PLAYABILITY AUDIT — end-to-end UI + engine verification (2026-10-10)

The career screen rendered fixtures but no match could be started, and most
action-bar buttons did nothing. Both were real bugs, not engine gaps, and both
are now covered by automated regression tests that drive the **real** UI against
the **real** engine.

**Blocking: the game could not be played at all.** `CareerView` serializes with
`rename_all = "camelCase"` while `src/types.ts` declared `next_fixture_id` /
`season_complete` in snake_case. Both were `undefined` at runtime, so
`undefined !== null` let the "Your next match" panel render, but `NextMatchCard`
found no fixture and bailed out — no Play button, no Select XI. The same mismatch
killed the season-complete panel and therefore "Go to off-season".

**Dead buttons** (each had no `onClick` or the wrong handler): `««`, `Fast
Forward` and `»»` all called `advanceDay`; `Continue` had no handler at all;
match `Main Screen` and `Options` did nothing; `Autoplay` could not be stopped;
the career-sidebar `Fixtures` row and the match/off-season sidebar rows did
nothing; `Save` gave no feedback anywhere. The field-setting select wrote
straight into the React state object, so the field preset never re-rendered. The
training screen always displayed "Balanced", ignoring the saved focus.

**Engine additions:** `advance_one_day` (steps one day, simulates it),
`advance_to_my_next_match` (skips to the player's fixture),
`start_with_options` (first season + form reset), and directory-based
`saves_in` / `load_settings_in` helpers so harnesses and tests can point the
store at a temp dir instead of an `AppHandle`.

**How it is verified now** (this is the part that keeps it working):

- `src-tauri/examples/e2e.rs` — the real command surface over HTTP, mirroring
  `lib.rs`'s handler list 1:1. Lives under `examples/`, never ships in the bundle.
- `e2e.html` + `src/mock-e2e.ts` — the browser transport shim (same contract).
- `bun run test:ui` (`tools/ui-e2e.mjs`) — drives the **compiled React app** in
  jsdom with every `invoke` routed to the real engine. 49 checks across the full
  player journey: setup → career controls → every tab → squad selection → live
  match (ball / over / to result / export / autoplay) → main screen → continue →
  a complete season rollover. Fails on React errors and unhandled rejections.
- `bun run test:contract` (`tools/contract-check.mjs`) — diffs every
  `src/types.ts` interface against live engine JSON. Caught two further latent
  mismatches the eye missed: `training_focus` and the missing
  `CareerState.records` shape.

Status: 104 Rust tests green, `cargo fmt`/`clippy -D warnings` clean, contract
check green, UI e2e green, release bundle rebuilt and launched.

## STANDING CHECKLIST — "a button that does nothing is a bug"

Before calling any screen done, every control must either do something or not
exist. Concretely:

1. No `<button>` without an `onClick` / `disabled` reason.
2. No control whose state is never read back (a select that writes into a React
   state object, or a screen that hardcodes a default instead of reading the
   saved value).
3. Every field the UI reads must exist in the engine's JSON — enforced by
   `bun run test:contract`.
4. Every screen reachable, and reachable from a working control — enforced by
   `bun run test:ui`.
5. Career/mutating commands must surface failure to the user, not swallow it.

## PLAYABILITY + PARITY AUDIT — international cricket, teams, sound (2026-10-10)

Follow-up to the audit above, driven by "the match is not working at all" and
"there are no sound effects". The engine looked healthy in tests; playing it
found four real bugs, and two of them made the headline features unusable.

**The World Cup was a dead end.** Semi-finals and the final were scheduled with
`home_id = 0` and never filled in, so they could be neither played nor
simulated. An international season therefore *could never complete* — the
calendar ran to day 122 with 3 fixtures permanently unplayable. Knockout slots
are now seeded from the completed group tables (`KnockoutPairings`), the final
is filled from the semi-final winners, and the two semi-finals are genuinely
different ties.

**Fast-forward skipped the player's own matches.** `advance_day` jumped to the
next day with *any* unplayed fixture, sailing past the player's own group games
— which only the player can play. Their groups never resolved, so the bracket
never seeded. It now stops at the player's next fixture.

**Chases ran on past the target.** Innings only ended on overs/all-out/
declaration, never on reaching the target: a 75 target finished on 158 in 20
overs. A chase now closes on the ball that passes the target.

**White-ball matches could be "tied".** Level scores returned a null winner.
That is not a possible T20/ODI result; it is now decided by a simulated Super
Over (`WinMargin::SuperOver`). Test draws are untouched — those are real.

**`quit_to_menu` was missing.** "Main Menu" only cleared React state, leaving
the engine's career slot populated, which blocked changing the database and
starting a new career. The app was wedged after any match.

Also fixed: the database picker reported "0 nations · 0 teams · 0 players" for
the default pack because the counts were hardcoded placeholders.

### Real-names international database

`tools/generate-intl-db.mjs` builds the ten ICC full members (India, Australia,
England, South Africa, New Zealand, Pakistan, West Indies, Sri Lanka,
Bangladesh, Afghanistan) — 148 players, 60 real grounds, World Test
Championship / World Cup / World Twenty20.

Packaged as a **selectable pack**, not the shipped default, so the fictional DB
stays what the game boots into and §4 DR-1 plus the trademark validator keep
passing. Real player names carry right-of-publicity risk, so the pack is
labelled a test database in the UI and is never first-party content. Each pack
pins its own `db_version`, so saves from different databases cannot mix.

The generator's self-check caught three genuine data defects while writing the
squads: duplicated entries, and a real cross-nation name collision (Rashid
Khan is an Afghan international who has also played for Pakistan). Name
uniqueness is therefore per nation, not global.

### Sound

No audio existed. Every cue is now **synthesised** with the Web Audio API —
filtered noise bursts and enveloped tones for the bat, the glove, timber and
the crowd — so there are no binary assets, nothing to license, and it works
offline. Twelve cues; the cue per delivery is derived from the engine's
`BallOutcome`, not the commentary text, so a wicket always sounds like a
wicket (and correctly wins over a boundary when both coincide).

Every host interaction is guarded. That guard was not theoretical: the UI e2e
suite caught a **boot-crashing** bug where `addEventListener` on a jsdom window
under Bun throws. Audio failing must never be able to take the game down.

### Verification

Every fix above is covered by a regression test **confirmed to fail against the
previous behaviour** (e.g. reverting the chase fix makes the test report
`75 -> 158 in 120 balls`). 112 Rust tests, 61 UI e2e checks, clippy and fmt
clean. A full international season — pick India, play the group stage, reach
the semi-final, finish the season, read career records — was played end to end
against the real engine.

## DESIGN PASS — rebuilt from the original's actual screenshots (2026-10-10)

The previous design was written from memory of Cricket Coach 2014 and was
wrong in kind, not just in detail. Fetching the official screenshots from
cricketcoachgame.com (`C14_SS5`, `C14_SS29`, `C14_SS52`) showed the real
language is a **late-2000s Windows utility on a cricket ground**, not glossy
game chrome:

- solid royal-blue title bar and a **blue left rail** with white labels and
  small triangular expanders — not individually beveled blue buttons
- a **deep saturated green** content field with the stadium showing faintly
  through, drawn in CSS rather than shipped as artwork
- **brushed-silver tabs** and form controls, with the selected tab dropping the
  metal fill for near-white
- **yellow column headers** with a thin white rule, on the green, with no
  filled header band — the single most recognisable table detail
- **white text fields with black text**; dropdowns and buttons in metal
- **blue slider bars** with a pale handle
- **no boxed panels at all**: content sits on the green, so panels here are
  only a soft wash and a hairline

`color-scheme` also moved to `light`. It was `dark`, which made the webview
render native controls — dropdowns, scrollbars — dark, producing dark-on-dark
unreadable dropdown text and fighting the whole design.

### Bugs this pass found

Design review meant actually looking at the screens, which surfaced four more
real defects that tests had not:

- **The setup screen had no left rail at all.** It rendered the bare field
  through the empty grid column. It now carries the blue rail the original
  shows on every screen.
- **The match screen rendered its own source comment on the page.** A `//`
  comment placed as a direct child of a JSX element is a text node, not a
  comment. Guarded by a new rendered-text check.
- **The international Table tab was permanently empty.** A national career's
  `competition_id` is a synthetic cup id matching no entry in
  `world.competitions`, so the league lookup returned no teams. A national side
  is now shown its World Cup **group standings**, which is what actually
  decides who reaches the semi-finals.
- **The table's NRR column printed `NaN`.** `TableRow.net_run_rate_x100` is a
  Rust *method*, not a serialized field, so the UI read `undefined`. Worse,
  `diffShape` in the contract check skipped the "declared but absent" direction
  entirely for array payloads, so the phantom field passed every check. Both
  are fixed: the NRR is derived in the UI from the fields that are sent, and
  `diffShape` now compares arrays against the union of their keys so a field
  absent from every element is reported.
- Also: unresolved knockouts showed a raw **"Team 0"** (now "To be decided",
  dimmed and unplayable), the sidebar never highlighted the active tab past the
  first row, and the partnership readout was the only violet element in a
  green-and-blue game.

### Verification

Reviewed by rendering each screen through the real bundle with the real
stylesheets and screenshotting in headless Chrome — the reference is a picture,
so the comparison has to be one. 114 Rust tests, 69 UI e2e checks, the contract
check, clippy, fmt and tsc all green. Each new guard was confirmed to fail
against the behaviour it was written for.

## AI workflow rules

1. One plan box = one branch/commit (`m1-1-ball-model` etc.).
2. Engine PRs must include tests; no test = no merge.
3. Contract changes: bump `specs/requirement.md` + this file first, code second.
4. No 3D assets, no real names, no trademarked league names in shipped DB.

## REVERSE ENGINEERING — Cricket Coach 2014, from an installed copy (2026-10-10)

An installed copy of the original was added at `old-game/` (git-ignored). The
batting mechanic shipped in the previous commit was a guess; the original's data
files show it was wrong in kind, and this pass rebuilds it against what is
actually there.

### What the original does

`db/db2014/bbbc_shot.csv` and `bbbc_ball.csv` hold the whole model in tagged CSV.
The player commits to a **stroke** — one of twelve, `SHOT_LEAVE` (20100) through
`SHOT_OFFDRIVE` (20112) — not to an outcome number. The runs follow from how the
stroke meets the delivery, which is classified first on two axes: length
(4 codes) and movement (11 codes), crossed with 7 bowler types.

So the previous outcome-number picker was a slot machine: nothing about choosing
"4" said it was a cut shot to a leg-cutter, because it wasn't a shot at all.

### Done

- [x] `tools/extract-cc2014-model.mjs` parses the original's data files into
      `data/reference/cc2014-model.json`. Self-checks: it fails loudly if the
      model stops parsing as expected. It copies no assets and no commentary
      prose — identifiers, tags and structure only.
- [x] `docs/cc2014-model.md` — the recovered model, and what is still unknown.
- [x] `Stroke` replaces `ShotChoice` in the engine, keeping the original's codes.
- [x] `BallLength` and `BallMovement` added; `stroke_vs_delivery` decides which
      stroke suits the ball, so the delivery changes which shot is a good idea.
- [x] Defensive strokes block and leave; attacking strokes span the outcome
      range including dismissal. The split follows the commentary data, where
      every LEAVE and FRONT_DEFENSIVE line is a dot ball.
- [x] Picker shows twelve strokes grouped block and attack, priced as expected
      runs for that delivery against the bowler in, with the wicket's chance
      shown alongside.
- [x] Assets copied from `old-game/` into `src/assets/cc2014/`, git-ignored, with
      `THIRD-PARTY.md` and `tools/sync-cc2014-assets.sh` to restore them.

### Fixed en route

- **`bun run test` was checking nothing.** `tsc --noEmit` against a
  solution-style tsconfig with `files: []` exits 0 without reading a single file,
  so the script reported green through several rounds of broken code. Now
  `tsc -b --force`, which immediately surfaced four real errors it had been
  hiding — including a sound checkbox bound to `audio.on` when the field is
  `audio.enabled`, so it never reflected the stored setting.
- The attacking branch of the stroke model scaled the format's weights and left
  the distribution nearly unchanged: every attacking stroke priced the same, so
  the picker asked a question the engine had no opinion about. Now assigned
  directly.
- `expectedRuns` read the engine's outcome *names* as numbers, returning 0 for
  everything. It looked like a rendering bug.

### Guards

New `src-tauri/tests/strokes.rs`. Each was confirmed to fail against the
behaviour it targets first:

- A committed stroke must change the innings — fails if `ctx.input.stroke` is
  ignored.
- Defensive blocks, attacking risks — fails if the split is removed.
- Stroke codes match the original's, since commentary tables key on them.
- The delivery decides which stroke is safe — checked directly against
  `stroke_vs_delivery`. An earlier version asserted this through simulated
  innings and passed with the delivery factored out, because driving and
  pulling differ in ambition anyway.

### Still unknown

- The probability constants, which are in the packed binary.
- What aggression levels `19000`-`19004` mean in plain terms.
- `bowlplans.csv`: 39 scripted delivery sequences are parsed but unused. A whole
  tactical layer the project does not have.
- The original's commentary table is incomplete — `COVERDRIVE` has no lines at
  all and six strokes have one or two. Not a parsing bug; the shipped game is
  like that.
