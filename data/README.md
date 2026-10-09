# `data/` — versioned fictional database (source of truth)

Policy (locked 2026-10-09, see `specs/requirement.md` §4):

- Default ship is **fully fictional**: invented players, clubs, and generic
  league names ("Indian T20 League", not "IPL"). No real names, trademarks,
  or logos in first-party content.
- Real-world likeness packs are **community mods only**, installed via the DB
  channel / local-file import and labelled third-party in-app.
- Takedown/blocklist requests are honoured — see `CONTRIBUTING.md`.

Layout (arrives in P0.3 / M5):

```
data/
  schema/        # JSON Schemas: player, team, competition, ground
  players/       # per-nation JSON
  teams/
  competitions/  # incl. genericised IPL-style + BPL-style T20 leagues
  grounds/
  CHANGELOG.md   # per-dbVersion notes shown by the updater
```

Nothing here yet — this file reserves the contract.
