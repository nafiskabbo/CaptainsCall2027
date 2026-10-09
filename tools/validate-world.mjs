#!/usr/bin/env node
/**
 * tools/validate-world.mjs
 *
 * Validates data/world.json against data/schema/world.schema.json using a
 * tiny built-in checker (no dependencies). Exits non-zero on the first problem.
 *
 * Usage: node tools/validate-world.mjs
 */

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const schema = JSON.parse(readFileSync(resolve(ROOT, "data/schema/world.schema.json"), "utf8"));
const world = JSON.parse(readFileSync(resolve(ROOT, "data/world.json"), "utf8"));

const problems = [];
const root = schema.definitions ? { definitions: schema.definitions, ...schema } : schema;

function check(node, s, path) {
  if (s.$ref) {
    const ref = s.$ref.replace("#/definitions/", "");
    const target = schema.definitions[ref];
    if (!target) {
      problems.push(`${path}: unknown $ref ${s.$ref}`);
      return;
    }
    check(node, target, path);
    return;
  }
  if (s.const !== undefined && node !== s.const) {
    problems.push(`${path}: expected const ${s.const}, got ${node}`);
  }
  if (s.enum && !s.enum.includes(node)) {
    problems.push(`${path}: ${node} not in enum [${s.enum.join(", ")}]`);
  }
  if (s.type) {
    const types = Array.isArray(s.type) ? s.type : [s.type];
    const jsType = node === null ? "null" : Array.isArray(node) ? "array" : typeof node;
    const ok = types.some((t) => t === jsType || (t === "integer" && typeof node === "number" && Number.isInteger(node)));
    if (!ok) {
      problems.push(`${path}: expected ${types.join("|")}, got ${jsType} (${JSON.stringify(node)})`);
      return;
    }
  }
  if (typeof node === "number") {
    if (s.minimum !== undefined && node < s.minimum) problems.push(`${path}: ${node} < min ${s.minimum}`);
    if (s.maximum !== undefined && node > s.maximum) problems.push(`${path}: ${node} > max ${s.maximum}`);
  }
  if (typeof node === "string") {
    if (s.minLength && node.length < s.minLength) problems.push(`${path}: too short`);
    if (s.maxLength && node.length > s.maxLength) problems.push(`${path}: too long`);
    if (s.pattern && !new RegExp(s.pattern).test(node)) problems.push(`${path}: fails pattern ${s.pattern}`);
  }
  if (Array.isArray(node)) {
    node.forEach((item, i) => check(item, s.items ?? {}, `${path}[${i}]`));
    if (s.minItems && node.length < s.minItems) problems.push(`${path}: fewer than ${s.minItems} items`);
  }
  if (node && typeof node === "object" && !Array.isArray(node)) {
    const props = s.properties ?? {};
    for (const [key, value] of Object.entries(node)) {
      if (s.additionalProperties === false && !props[key]) {
        problems.push(`${path}: unexpected property "${key}"`);
        continue;
      }
      if (props[key]) check(value, props[key], `${path}.${key}`);
    }
    for (const req of s.required ?? []) {
      if (!(req in node)) problems.push(`${path}: missing required property "${req}"`);
    }
  }
}

check(world, root, "world");

// Cross-references the schema can't express.
const nationIds = new Set(world.nations.map((n) => n.id));
const teamIds = new Set(world.teams.map((t) => t.id));
const playerIds = new Set(world.players.map((p) => p.id));
const groundIds = new Set(world.grounds.map((g) => g.id));

for (const t of world.teams) {
  if (!nationIds.has(t.nation_id)) problems.push(`team ${t.id}: unknown nation ${t.nation_id}`);
  if (!groundIds.has(t.home_ground_id)) problems.push(`team ${t.id}: unknown ground ${t.home_ground_id}`);
}
for (const p of world.players) {
  if (!nationIds.has(p.nation_id)) problems.push(`player ${p.id}: unknown nation`);
  if (p.team_id !== null && !teamIds.has(p.team_id)) problems.push(`player ${p.id}: unknown team ${p.team_id}`);
}
for (const c of world.competitions) {
  if (!nationIds.has(c.nation_id)) problems.push(`competition ${c.id}: unknown nation`);
  for (const id of c.team_ids) if (!teamIds.has(id)) problems.push(`competition ${c.id}: unknown team ${id}`);
}

// Business rules from the spec.
const banned = ["ipl", "bpl", "psl", "cpl"];
for (const c of world.competitions) {
  const lower = c.name.toLowerCase();
  for (const b of banned) {
    if (lower.includes(b)) problems.push(`competition "${c.name}": trademarked league name`);
  }
}
for (const t of world.teams) {
  const owned = world.players.filter((p) => p.team_id === t.id).length;
  if (!t.is_national && owned < 11) problems.push(`team "${t.name}": only ${owned} players, need a full XI`);
}

if (problems.length > 0) {
  console.error(`world.json invalid (${problems.length} problem(s)):`);
  for (const p of problems.slice(0, 20)) console.error(`  - ${p}`);
  process.exit(1);
}

console.log(
  `world.json OK — ${world.players.length} players, ${world.teams.length} teams, ` +
    `${world.nations.length} nations, ${world.grounds.length} grounds, ${world.competitions.length} competitions`,
);
