# Contributing to Captain's Call 2027

Thanks for helping build an open tribute to Cricket Coach 2014 (credit: Oli Norwell / Rockingham Software).

## AI-first workflow

- Work item = one box in `tasks.plan.md`. Reference it in branch + PR (`p1-1-ball-model`).
- Keep modules small and typed so AI + reviewers can verify. Rust core needs unit tests; UI needs component tests where logic lives.
- Run validators before pushing: schema check (`data/`), engine regression corpus, lint.

## Roster / data PRs (`data/`)

1. Edit JSON per `data/schema/` (coming in M0). One nation or team per PR.
2. Sources required in PR description (e.g. official squad lists). No scraped proprietary DBs.
3. Player-name opt-out: if someone asks for removal, we blocklist — no debate.
4. CI runs bounds + duplicate + smoke-sim checks. Human review signs off.

## Pull requests

- Small, one feature. Link the plan box. Include before/after sim output if engine changed.
- Do not commit binaries, saves, or copied assets from the original game.

## Conduct

Be kind. See `CODE_OF_CONDUCT.md`. Cricket debates welcome, abuse not.
