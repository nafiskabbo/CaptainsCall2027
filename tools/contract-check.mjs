//! Contract check: `src/types.ts` must describe what the Rust core actually
//! serializes.
//!
//! A field-name mismatch is invisible to TypeScript (both sides are just
//! objects) and to `cargo test` (Rust never sees the TS type). It silently
//! yields `undefined` — which is exactly how the career screen lost its Play
//! button once. This script boots the real engine and diffs the live JSON
//! against the declared interfaces.
//!
//! ```text
//! node tools/contract-check.mjs
//! ```

import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const HARNESS = process.env.CC_HARNESS ?? "http://127.0.0.1:8787";

let failures = 0;
const ok = (message) => console.log(`  PASS  ${message}`);
const bad = (message) => {
  failures += 1;
  console.log(`  FAIL  ${message}`);
};

/** Parses `export interface X { ... }` blocks into flat key lists. */
function parseInterfaces(source) {
  const out = new Map();
  const lines = source.split("\n");
  let current = null;
  for (const line of lines) {
    const open = line.match(/^export interface (\w+)\s*\{/);
    if (open) {
      current = { name: open[1], keys: [] };
      continue;
    }
    if (current && /^\s*\}\s*$/.test(line)) {
      out.set(current.name, current.keys);
      current = null;
      continue;
    }
    if (!current) continue;
    // skip comments and nested/inline object literals
    if (/^\s*(\/\/|\/\*|\*)/.test(line)) continue;
    if (/\{|\[/.test(line) && !/^\s*\w+\??:/.test(line.trim())) continue;
    const field = line.match(/^\s{2}(\w+)(\?)?:\s*(.+?)[,;]?\s*$/);
    if (field) {
      current.keys.push({ name: field[1], optional: Boolean(field[2]), type: field[3] });
    }
  }
  return out;
}

/**
 * Every key of `shape` that also exists in the declared interface, plus any
 * declared key missing from the payload.
 */
function diffShape(declared, value, prefix = "") {
  const missing = [];
  const extra = [];
  if (value === null || typeof value !== "object") return { missing, extra };
  const isArray = Array.isArray(value);
  /*
    For an array payload, compare against the UNION of the keys seen across all
    elements, not just the first row.

    Skipping the "declared but absent" direction for arrays — as this did — left
    the most important check blind to every list type. `TableRow` declared
    `net_run_rate_x100`, which is a Rust *method* and never serialized; the UI
    read `undefined` and rendered a NaN column, and every contract check passed
    because arrays were never checked in that direction.

    A field that is absent from every element is a phantom declaration and is
    reported. A field merely absent from one sparse row is not, which is the
    tolerance the original `continue` was reaching for.
  */
  const samples = isArray ? value.filter((v) => v && typeof v === "object") : [value];
  const sample = samples[0];
  if (sample && typeof sample === "object") {
    const declaredKeys = declared ?? [];
    const seen = new Set();
    for (const row of samples) {
      for (const key of Object.keys(row)) seen.add(key);
    }
    for (const key of seen) {
      const match = declaredKeys.find((d) => d.name === key);
      if (!match) {
        extra.push(`${prefix}${key}`);
        continue;
      }
      missing.push(...diffShape(null, sample[key], `${prefix}${key}.`).missing);
    }
    for (const d of declaredKeys) {
      if (seen.has(d.name)) continue;
      if (!d.optional) missing.push(`${prefix}${d.name}`);
    }
  }
  return { missing, extra };
}

const TYPES = parseInterfaces(fs.readFileSync(path.join(ROOT, "src/types.ts"), "utf8"));

/** Which payload to compare against which declared interface. */
const CHECKS = [
  { cmd: "get_career", type: "CareerView", label: "CareerView (get_career)" },
  { cmd: "get_career", type: "CareerState", at: "state", label: "CareerState (get_career.state)" },
  { cmd: "get_transfer_pool", type: "TransferListing", label: "TransferListing (transfer pool)" },
  { cmd: "list_saves", type: "SaveSummary", label: "SaveSummary (list_saves)" },
  { cmd: "get_finances", type: "Finances", label: "Finances (get_finances)" },
  { cmd: "get_update_state", type: "UpdateState", label: "UpdateState (get_update_state)" },
];

const MATCH_CHECKS = [
  { cmd: "get_match", type: "MatchSnapshot", label: "MatchSnapshot (get_match)" },
  { cmd: "get_match", type: "Innings", at: "innings", label: "Innings (innings[])" },
];

/*
 * Shapes reachable only after a match has been played. These are the ones most
 * likely to drift, because nothing inspects them until a result exists — a
 * field that Rust computes as a method rather than serializing is invisible to
 * every other check here.
 */
const POST_MATCH_CHECKS = [
  { cmd: "get_career_after", type: "TableRow", at: "table", label: "TableRow (career.table[])" },
];

async function main() {
  const proc = spawn(
    "cargo",
    ["run", "--example", "e2e", "--", "127.0.0.1:8781", "target/e2e-contract"],
    { cwd: path.join(ROOT, "src-tauri"), stdio: "ignore" },
  );
  const deadline = Date.now() + 180_000;
  for (;;) {
    try {
      if ((await fetch(`${HARNESS_SHIM()}/health`)).ok) break;
    } catch {
      /* compiling */
    }
    if (Date.now() > deadline) {
      console.error("harness did not start");
      process.exit(3);
    }
    await delay(500);
  }
  const base = "http://127.0.0.1:8781";

  const invoke = async (cmd, args = {}) => {
    const res = await fetch(`${base}/invoke`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ cmd, args }),
    });
    const payload = await res.json();
    if (!payload.ok) throw new Error(payload.error);
    return payload.data;
  };

  try {
    console.log("\nCaptain's Call 2027 — API contract\n");
    const career = await invoke("start_career", { name: "Contract", teamId: 101, seed: 20270101 });
    const pool = await invoke("get_transfer_pool");
    const finances = await invoke("get_finances");
    const updates = await invoke("get_update_state");
    const next = career.state.fixtures.find((f) => f.id === career.nextFixtureId);
    await invoke("start_fixture", { fixtureId: next.id });
    const match = await invoke("sim_match", { count: 1 });
    // Play the fixture out so the table has a result in it. The table is
    // compared below, and an empty array cannot be checked — which is exactly
    // how a phantom field like `net_run_rate_x100` survived here for so long:
    // a Rust *method* was declared as a serialized field, and the UI rendered
    // `NaN` from the resulting `undefined`.
    await invoke("sim_match", { count: 100000 });
    const careerAfter = await invoke("get_career");
    // Save first so list_saves has a row to compare against.
    await invoke("save_career");
    const saves = await invoke("list_saves");
    const payloads = {
      get_career: career,
      get_career_after: careerAfter,
      get_transfer_pool: pool,
      get_finances: finances,
      get_update_state: updates,
      get_match: match,
      list_saves: saves,
    };
    const all = [...CHECKS, ...MATCH_CHECKS, ...POST_MATCH_CHECKS];
    for (const check of all) {
      const declared = TYPES.get(check.type);
      if (!declared) {
        bad(`src/types.ts has no '${check.type}' interface`);
        continue;
      }
      let value = payloads[check.cmd];
      if (value === undefined) {
        bad(`${check.label}: no payload captured`);
        continue;
      }
      if (check.at) value = value[check.at];
      if (value === undefined || value === null || (Array.isArray(value) && value.length === 0)) {
        bad(`${check.label}: engine returned nothing to compare`);
        continue;
      }
      const { missing, extra } = diffShape(declared, value);
      if (missing.length === 0 && extra.length === 0) {
        ok(`${check.label} — ${declared.length} fields match`);
      } else {
        if (missing.length) bad(`${check.label}: missing from JSON: ${missing.join(", ")}`);
        if (extra.length) bad(`${check.label}: in JSON but not in types.ts: ${extra.join(", ")}`);
      }
    }

    // And the reverse direction: the engine must not accept a selection whose
    // shape the TS type promises.
    const selection = await invoke("get_selection", { fixtureId: next.id });
    const declaredSelection = TYPES.get("TeamSelection");
    const selDiff = diffShape(declaredSelection, selection);
    if (selDiff.missing.length === 0 && selDiff.extra.length === 0) {
      ok("TeamSelection (get_selection) — matches");
    } else {
      bad(`TeamSelection: missing ${selDiff.missing.join(", ")} ; extra ${selDiff.extra.join(", ")}`);
    }

    console.log(
      failures === 0
        ? "\nAll declared shapes match the engine.\n"
        : `\n${failures} contract mismatch(es). src/types.ts and the Rust serde output disagree.\n`,
    );
    proc.kill("SIGTERM");
    process.exit(failures === 0 ? 0 : 1);
  } catch (error) {
    console.error(`\nerror: ${error.message}`);
    proc.kill("SIGTERM");
    process.exit(2);
  }
}

function HARNESS_SHIM() {
  return "http://127.0.0.1:8781";
}

main();
