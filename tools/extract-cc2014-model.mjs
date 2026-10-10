//! Extract the Cricket Coach 2014 match model from an installed copy of the game.
//!
//! The game's rules are not in its binary in any readable form — it is a
//! UPX-packed Delphi executable. But its *vocabulary* is all in the data files,
//! in a form this script can parse:
//!
//!   db/db2014/bbbc_ball.csv   deliveries, tagged by length + movement + bowler type
//!   db/db2014/bbbc_shot.csv    strokes, tagged by shot + aggression + fielded + runs
//!   db/db2014/bowlplans.csv   named delivery sequences a bowler can bowl
//!
//! Together those describe the whole ball-by-ball model, and they contradict the
//! shot system this project originally guessed at (see docs/cc2014-model.md).
//!
//! This writes data/reference/cc2014-model.json. It copies no assets and no
//! commentary text — only the identifiers, tags and structure, which is the
//! functional design. The prose in those files is the author's and stays here.
//!
//!   bun tools/extract-cc2014-model.mjs [path/to/Cricket Coach 2014]

import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const root =
  process.argv[2] ??
  join(process.cwd(), "old-game", "Cricket Coach 2014");
const dbDir = join(root, "db", "db2014");

if (!existsSync(dbDir)) {
  console.error(`No database at ${dbDir}`);
  console.error("Pass the installed game directory as the first argument.");
  process.exit(1);
}

const read = (f) => readFileSync(join(dbDir, f), "utf8");

/**
 * Split a `bbbc_*.csv` into its header comments and its data rows.
 *
 * Lines beginning `%` are documentation; the notable ones declare enum
 * name/code pairs as `NAME<whitespace>CODE`, and prose lines of the form
 * `% - Forward defensive shot` name a section.
 */
function parseBBC(text) {
  const enums = new Map();
  const sections = [];
  const rows = [];

  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith("%")) {
      // Enum declarations end in a stray `#`: `% SHOT_LEAVE\t\t\t\t20100 #`.
      const decl = line.replace(/#+\s*$/, "").match(/^%\s*([A-Z][A-Z0-9_]+)\s+(\d+)\s*$/);
      if (decl) enums.set(Number(decl[2]), decl[1]);
      const sec = line.match(/^%\s*-\s*(.+?)\s*$/);
      if (sec) sections.push(sec[1]);
      continue;
    }
    if (!line) continue;

    // Rows are `commentary#constraint`, where the constraint is one
    // comma-separated tag string rather than several `#`-separated fields.
    const hash = line.indexOf("#");
    if (hash < 0) continue;
    rows.push({
      text: line.slice(0, hash).trim(),
      tags: line
        .slice(hash + 1)
        .split(",")
        .map((t) => t.trim()),
    });
  }
  return { enums, sections, rows };
}

/** The ball taxonomy: lengths, movements, and bowler types. */
const ball = parseBBC(read("bbbc_ball.csv"));
/** The shot taxonomy, plus every commentary line tagged by outcome. */
const shot = parseBBC(read("bbbc_shot.csv"));

// Group the enums by their numeric band, which is how the original namespaces
// them: 8xxx = lengths, 9xxx = movements, 1xxxx = shots, 19xxx = aggression.
const byPrefix = (m, prefix) =>
  [...m.entries()]
    .filter(([code]) => code >= prefix && code < prefix + 1000)
    .map(([code, name]) => ({ code, name }))
    .sort((a, b) => a.code - b.code);

const lengths = byPrefix(ball.enums, 8000);
const movements = byPrefix(ball.enums, 9000);
const strokes = byPrefix(shot.enums, 20000);
const bowlerTypes = [...ball.enums.entries()]
  .filter(([code]) => code < 10)
  .map(([code, name]) => ({ code, name }))
  .sort((a, b) => a.code - b.code);

/**
 * Turn a tagged commentary row into the constraint it represents.
 *
 * `bbbc_shot.csv` rows are `text#shot,aggression,fielded,in_air,runs`, where
 * `-1` means "any" and `0` means "specifically not". The last column is the
 * runs scored, not a boundary flag as its own header comment suggests — the
 * data has values 1, 2, 3, 4 and 6 in it.
 */
function shotConstraints(tags) {
  const [type, aggression, fielded, inAir, runs] = tags;
  return {
    stroke: type === "-1" || type === "" ? null : Number(type),
    aggression: aggression === "-1" || aggression === "" ? null : Number(aggression),
    fielded: fielded === "-1" || fielded === "" ? null : Number(fielded),
    inAir: inAir === "-1" || inAir === "" ? null : Number(inAir),
    runs: runs === "-1" || runs === "" ? null : Number(runs),
  };
}

function ballConstraints(tags) {
  const [length, movement, bowler] = tags;
  return {
    length: length === "-1" || length === "" ? null : Number(length),
    movement: movement === "-1" || movement === "" ? null : Number(movement),
    bowlerType: bowler === "-1" || bowler === "" ? null : Number(bowler),
  };
}

/**
 * Bowling plans: a named sequence of deliveries.
 *
 * Header is `?,?,plan name,plan description` followed by a line count, then one
 * row per delivery: `seq,?,length,movement,?,weight,description`. Fields are
 * unquoted, so the plan name and description are taken as the last two columns
 * and a description containing a comma simply truncates.
 */
function parseBowlPlans(text) {
  const lines = text.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  const plans = [];
  let i = 0;
  while (i < lines.length) {
    const head = lines[i].split(",");
    const count = Number(lines[i + 1]);
    if (head.length < 4 || !Number.isInteger(Number(head[0])) || !Number.isInteger(count)) {
      i += 1;
      continue;
    }
    const name = head[2].trim();
    const description = head.slice(3).join(",").trim();
    const deliveries = [];
    let j = i + 2;
    while (deliveries.length < count && j < lines.length) {
      const d = lines[j].split(",");
      if (d.length >= 7 && /^-?\d+$/.test(d[3])) {
        deliveries.push({
          seq: Number(d[0]),
          length: d[2] === "-1" ? null : Number(d[2]),
          movement: d[3] === "-1" ? null : Number(d[3]),
          weight: Number(d[5]),
          description: d.slice(6).join(","),
        });
      }
      j += 1;
    }
    plans.push({ name, description, deliveries });
    i = j;
  }
  return plans;
}

const plans = parseBowlPlans(read("bowlplans.csv"));

/** Which outcomes the original can actually describe, from the commentary tags. */
const observedRuns = [
  ...new Set(
    shot.rows
      .map((r) => shotConstraints(r.tags).runs)
      .filter((v) => v !== null),
  ),
].sort((a, b) => a - b);

const outDir = join(process.cwd(), "data", "reference");
mkdirSync(outDir, { recursive: true });

const model = {
  _about:
    "Cricket Coach 2014 ball-by-ball model, extracted from the installed data " +
    "files by tools/extract-cc2014-model.mjs. Identifiers, tags and structure " +
    "only — no commentary prose, no assets.",
  source: "Cricket Coach 2014 (db/db2014/*.csv)",
  taxonomy: {
    lengths,
    movements,
    bowlerTypes,
    strokes,
  },
  outcomes: {
    // Runs the shot commentary is tagged with. Note 5 is absent, as it must be:
    // five cannot be scored off the bat.
    runs: observedRuns,
    wicketCodes: null,
  },
  commentary: {
    // Counts only. The lines themselves are the author's text.
    deliveries: ball.rows.length,
    shots: shot.rows.length,
    byStroke: Object.fromEntries(
      strokes.map((s) => [
        s.code,
        shot.rows.filter((r) => shotConstraints(r.tags).stroke === s.code).length,
      ]),
    ),
  },
  bowlingPlans: plans.map((p) => ({
    name: p.name,
    description: p.description,
    deliveries: p.deliveries.map((d) => ({
      seq: d.seq,
      length: d.length,
      movement: d.movement,
      weight: d.weight,
    })),
  })),
};

const outPath = join(outDir, "cc2014-model.json");
writeFileSync(outPath, JSON.stringify(model, null, 2) + "\n");

/*
  Self-checks. An extractor that silently extracts the wrong thing is worse than
  no extractor, because everything downstream inherits the error.
*/
const problems = [];
if (strokes.length !== 12) problems.push(`expected 12 strokes, got ${strokes.length}`);
if (lengths.length !== 4) problems.push(`expected 4 lengths, got ${lengths.length}`);
if (movements.length !== 11) problems.push(`expected 11 movements, got ${movements.length}`);
if (bowlerTypes.length !== 7) problems.push(`expected 7 bowler types, got ${bowlerTypes.length}`);
if (observedRuns.includes(5)) problems.push("runs include an impossible 5");
if (plans.length === 0) problems.push("no bowling plans parsed");

console.log(`Wrote ${outPath}\n`);
console.log(`  strokes        ${strokes.length}`);
console.log(`  lengths        ${lengths.length}   ${lengths.map((l) => l.name).join(", ")}`);
console.log(`  movements      ${movements.length}`);
console.log(`  bowler types   ${bowlerTypes.length}`);
console.log(`  run outcomes   ${observedRuns.join(", ")}`);
console.log(`  bowling plans  ${plans.length}`);
console.log(`  commentary     ${shot.rows.length} shot lines, ${ball.rows.length} delivery lines`);

if (problems.length) {
  console.error(`\nFAILED — the model did not parse as expected:`);
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
}