# Cricket Coach 2014 — the ball-by-ball model

Extracted from an installed copy of the game. This is the reference the engine
is built against, and it exists because the first attempt at the batting
mechanic was a guess that turned out to be wrong in an instructive way.

`tools/extract-cc2014-model.mjs` parses `db/db2014/*.csv` into
`data/reference/cc2014-model.json`. Re-run it after changing the extractor; it
fails loudly if the model stops parsing the way it is supposed to.

## Why this document exists

The original's mechanics are not readable in its binary — `CC2014.exe` is a
UPX-packed Delphi executable, 23MB, no symbols. But the game's *vocabulary* is
entirely in its data files, in a plain tagged-CSV form. Reconstructing the model
from those is what the extractor does.

## The shot model: strokes, not outcomes

This is the correction. The first implementation of the batting mechanic let the
player pick an **outcome number** — `0`, `1`, `2`, `3`, `4`, `6` — and the
engine resolved that attempt. It felt right and was wrong. `bbbc_shot.csv`
declares the real vocabulary:

| Code | Stroke | Sprite |
|---|---|---|
| 20100 | `LEAVE` | `SHOT_LEAVE` |
| 20101 | `FRONT_DEFENSIVE` | `SHOT_FRDEF` |
| 20102 | `BACK_DEFENSIVE` | `SHOT_BDEF` |
| 20103 | `STRAIGHTDRIVE` | `SHOT_STRDRV` |
| 20104 | `COVERDRIVE` | `SHOT_CDRIVE` |
| 20105 | `LEGGLANCE` | `SHOT_LEGGLANCE` |
| 20106 | `CUT` | `SHOT_CUT` |
| 20107 | `PULL` | `SHOT_PULL` |
| 20108 | `HOOK` | `SHOT_HOOK` |
| 20109 | `SWEEP` | `SHOT_SWEEP` |
| 20111 | `ONDRIVE` | `SHOT_ONDRIVE` |
| 20112 | `OFFDRIVE` | `SHOT_DRIVE` |

The player picks a **stroke**. The runs are a consequence of that stroke meeting
a particular delivery, not the thing that was chosen. This is the difference
between a menu and a game: "cover drive" against a good-length inswinging ball
is a decision that can go several ways, and the player is the one making it.

Sprites ship as `SHOT_X` plus `SHOT_X_L` for left-handers — a mirrored variant.
Defensive strokes and the straight drive have no `_L`, being already symmetric.

## The delivery: two axes plus a bowler type

`bbbc_ball.csv` classifies every ball *before* the batter plays a shot, on two
independent axes.

**Length** (8xxx) — `GOODLENGTH` 8001, `SHORT_OFF` 8002, `YORKER` 8003,
`BOUNCER` 8004.

**Movement** (9xxx) — `STRAIGHT` 9001, `INSWING` 9002, `OUTSWING` 9003,
`OFFCUTTER` 9004, `LEGCUTTER` 9005, `SLOWER` 9006, `LEGBREAK` 9007,
`OFFBREAK` 9008, `STRAIGHTONE` 9009, `WRONGUN` 9010, `SPECIALITY` 9011.

**Bowler type** — `CANTBOWL` 0, `FASTBOWL` 1, `FASTMED` 2, `MEDFAST` 3,
`MEDIUM` 4, `OFFSPIN` 5, `LEGSPIN` 6.

## Outcomes fall out of stroke, not the reverse

Every commentary line is tagged with the situation it describes:
`text#stroke,aggression,fielded,in_air,runs`, where `-1` means "any" and `0`
means "specifically not". Counting the lines by stroke shows the mechanic
directly:

```
LEAVE              25 lines   all 0 runs
FRONT_DEFENSIVE    25 lines   all 0 runs
BACK_DEFENSIVE     25 lines   0 runs, occasionally 1
STRAIGHTDRIVE      49 lines   spans 0,1,2,3,4,6 and caught
```

Defensive strokes are risk-free and worth nothing. Attacking strokes span the
whole outcome range *including dismissal*. That is the game: block and survive,
or attack and find out.

There are five aggression levels, `19000`–`19004`, used as a secondary tag on
strokes that can be played at different force. The `in_air` column is `0` for
every line shipped — declared but unused in this build.

## Commentary is looked up, not generated

The engine classifies an outcome first, then selects a commentary line matching
the tags. This project had it backwards — generating prose from the outcome
class. The lookup table is the original's and its text is the author's; only the
tag structure was extracted.

The shipped table is also **incomplete**, which is worth knowing rather than
hiding: `COVERDRIVE` has no commentary lines at all, and `LEGGLANCE`, `CUT`,
`PULL`, `HOOK`, `ONDRIVE` and `OFFDRIVE` have one or two each. The game falls
back to the nearest match, which is why `STRAIGHTDRIVE` has 49 lines to the
others' handful — it is acting as the generic attacking shot. (An earlier build
directory, `db/AAA_2014_610`, ships these two files at zero bytes, so the table
was filled in later and never completely.)

## Bowling plans

`bowlplans.csv` holds 39 named delivery sequences, each a header, a delivery
count, and rows of `seq,?,length,movement,?,weight,description`. A bowler's plan
is a scripted over, not a per-ball decision. This is a whole tactical layer the
project does not have.

## What was taken, and what was not

This project reuses the original's **assets** under the owner's private-use
decision, recorded in `THIRD-PARTY.md`. The **model** above is reimplemented from
the data files. No commentary prose was copied: the extractor reads the tags and
counts the lines, and deliberately drops the text.

## Still unknown

- The probability constants. These live in the packed binary; recovering them
  needs a Delphi decompiler and is a larger job than it looks.
- What `19000`–`19004` mean in plain terms. The codes exist in the tags but are
  not declared in any data file, and the binary that names them is packed.