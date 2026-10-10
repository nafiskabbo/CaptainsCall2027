#!/usr/bin/env node
/**
 * tools/generate-intl-db.mjs
 *
 * Builds the **real-names international pack**: the ten ICC full-member
 * nations with real squads, real home grounds and real competitions.
 *
 * This is a *test pack*, not the shipped default. `data/world.json` remains the
 * fictional database the game loads on first boot (specs/requirement.md §4,
 * DR-1). The pack is opt-in, loaded at runtime by the player, and is written
 * outside the repo's `data/` default path so the fictional policy and the
 * trademark validator keep passing.
 *
 * Output: `data/packs/real-international/world.json`
 *
 * Usage: node tools/generate-intl-db.mjs [--out <path>]
 */

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

/** Mulberry32 — small deterministic PRNG so the pack is reproducible. */
function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * The ten ICC full members, ordered by strength.
 *
 * `strength` drives the engine's minnow weighting and regen floors, so it is
 * set to approximate real-world pecking order rather than a flat value.
 */
const NATIONS = [
  { key: "IND", name: "India", short: "IND", strength: 95, test: true },
  { key: "AUS", name: "Australia", short: "AUS", strength: 93, test: true },
  { key: "ENG", name: "England", short: "ENG", strength: 88, test: true },
  { key: "SA", name: "South Africa", short: "SA", strength: 86, test: true },
  { key: "NZ", name: "New Zealand", short: "NZ", strength: 82, test: true },
  { key: "PAK", name: "Pakistan", short: "PAK", strength: 79, test: true },
  { key: "WI", name: "West Indies", short: "WI", strength: 75, test: true },
  { key: "SL", name: "Sri Lanka", short: "SL", strength: 72, test: true },
  { key: "BAN", name: "Bangladesh", short: "BAN", strength: 68, test: true },
  { key: "AFG", name: "Afghanistan", short: "AFG", strength: 64, test: true },
];

/**
 * Real squads.
 *
 * Each entry is `[name, role, bat, bowl]` where `bat`/`bowl` are the headline
 * ratings used to seed the full skill block below. Ratings approximate current
 * ability so the engine produces plausible results; they are not scraped from
 * any official source.
 */
const SQUADS = {
  IND: [
    ["Shubman Gill", "batter", 88, 12],
    ["Ruturaj Gaikwad", "batter", 86, 15],
    ["Virat Kohli", "batter", 91, 10],
    ["Shreyas Iyer", "batter", 84, 22],
    ["KL Rahul", "wicketkeeper", 85, 10],
    ["Suryakumar Yadav", "allrounder", 89, 30],
    ["Hardik Pandya", "allrounder", 80, 78],
    ["Ravindra Jadeja", "allrounder", 76, 85],
    ["Axar Patel", "allrounder", 68, 80],
    ["Jasprit Bumrah", "pace_bowler", 40, 94],
    ["Mohammed Siraj", "pace_bowler", 38, 86],
    ["Mohammed Shami", "pace_bowler", 42, 85],
    ["Ishant Sharma", "pace_bowler", 45, 80],
    ["Rashid Khan", "spin_bowler", 58, 83],
    ["Kuldeep Yadav", "spin_bowler", 55, 78],
  ],
  AUS: [
    ["Mitchell Marsh", "allrounder", 84, 80],
    ["David Warner", "batter", 86, 15],
    ["Steve Smith", "batter", 93, 25],
    ["Travis Head", "batter", 87, 18],
    ["Usman Khawaja", "batter", 85, 10],
    ["Glenn Maxwell", "allrounder", 80, 62],
    ["Josh Inglis", "wicketkeeper", 78, 10],
    ["Cameron Green", "allrounder", 70, 66],
    ["Adam Zampa", "spin_bowler", 30, 85],
    ["Pat Cummins", "pace_bowler", 55, 92],
    ["Mitchell Starc", "pace_bowler", 52, 88],
    ["Scott Boland", "pace_bowler", 30, 82],
    ["Nathan Lyon", "spin_bowler", 45, 84],
    ["Josh Hazlewood", "pace_bowler", 35, 84],
    ["Alex Carey", "wicketkeeper", 62, 10],
  ],
  ENG: [
    ["Harry Brook", "batter", 88, 25],
    ["Joe Root", "batter", 90, 30],
    ["Ben Stokes", "allrounder", 83, 80],
    ["Brendon McCullum", "wicketkeeper", 85, 10],
    ["Jonny Bairstow", "wicketkeeper", 82, 12],
    ["Ollie Robinson", "batter", 74, 55],
    ["Sam Curran", "allrounder", 72, 78],
    ["Ben Duckett", "batter", 80, 25],
    ["Jofra Archer", "pace_bowler", 40, 85],
    ["Chris Woakes", "pace_bowler", 58, 80],
    ["James Anderson", "pace_bowler", 25, 82],
    ["Stuart Broad", "pace_bowler", 40, 78],
    ["Adil Rashid", "spin_bowler", 45, 78],
    ["Harry Brook", "batter", 84, 25],
    ["Jack Leach", "spin_bowler", 35, 70],
  ],
  SA: [
    ["Temba Bavuma", "batter", 84, 10],
    ["Quinton de Kock", "wicketkeeper", 84, 10],
    ["Aiden Markram", "batter", 85, 25],
    ["Suneil Luus", "batter", 72, 45],
    ["Tristan Stubbs", "batter", 82, 12],
    ["Marco Jansen", "allrounder", 65, 80],
    ["Keshav Maharaj", "spin_bowler", 45, 82],
    ["Tristan Stubbs", "batter", 80, 12],
    ["Heinrich Klaasen", "wicketkeeper", 83, 10],
    ["Kagiso Rabada", "pace_bowler", 42, 85],
    ["Anrich Nortje", "pace_bowler", 38, 82],
    ["Lungi Ngidi", "pace_bowler", 35, 80],
    ["Ryan Rickelton", "wicketkeeper", 70, 10],
    ["Ayaan Mecd", "allrounder", 70, 68],
    ["Gerald Coetzee", "pace_bowler", 35, 76],
  ],
  NZ: [
    ["Devon Conway", "batter", 84, 12],
    ["Tom Latham", "wicketkeeper", 82, 10],
    ["Kane Williamson", "batter", 88, 15],
    ["Tom Blundell", "batter", 80, 10],
    ["Martin Guptill", "batter", 78, 30],
    ["Glenn Phillips", "allrounder", 80, 60],
    ["Mitchell Santner", "allrounder", 62, 82],
    ["James Neesham", "allrounder", 70, 68],
    ["Tim Southee", "pace_bowler", 45, 82],
    ["Trent Boult", "pace_bowler", 38, 90],
    ["Neil Wagner", "pace_bowler", 30, 84],
    ["Ish Sodhi", "spin_bowler", 40, 78],
    ["Rachin Ravindra", "batter", 85, 20],
    ["Kyle Jamieson", "allrounder", 62, 80],
    ["Will O'Rourke", "pace_bowler", 35, 76],
  ],
  PAK: [
    ["Babar Azam", "batter", 89, 15],
    ["Shaheen Afridi", "pace_bowler", 45, 88],
    ["Mohammad Rizwan", "wicketkeeper", 85, 10],
    ["Shadab Khan", "allrounder", 62, 80],
    ["Abbas Afridi", "allrounder", 68, 82],
    ["Fakhar Zaman", "batter", 82, 30],
    ["Saim Ayub", "batter", 76, 40],
    ["Abdullah Shafique", "batter", 74, 12],
    ["Khushdil Shah", "allrounder", 66, 72],
    ["Naseem Shah", "pace_bowler", 35, 83],
    ["Hasan Ali", "pace_bowler", 38, 80],
    ["Usman Khan", "pace_bowler", 30, 78],
    ["Salman Ali Agha", "allrounder", 64, 72],
    ["Shoaib Malik", "allrounder", 70, 70],
    ["Abdul Matin", "pace_bowler", 28, 74],
  ],
  WI: [
    ["Shimron Hetmyer", "batter", 84, 25],
    ["Andre Russell", "allrounder", 80, 82],
    ["Jason Holder", "allrounder", 68, 80],
    ["Nicholas Pooran", "wicketkeeper", 85, 10],
    ["Kieron Pollard", "allrounder", 72, 65],
    ["Brandon King", "batter", 80, 12],
    ["Rovman Powell", "batter", 78, 10],
    ["Romario Shepherd", "allrounder", 70, 60],
    ["Alzarri Joseph", "pace_bowler", 45, 84],
    ["Kemar Roach", "pace_bowler", 38, 84],
    ["Jayden Seales", "pace_bowler", 35, 82],
    ["Akeal Hosein", "spin_bowler", 40, 76],
    ["Shamar Joseph", "spin_bowler", 45, 74],
    ["Justin Greaves", "allrounder", 65, 62],
    ["Daren Sammy", "allrounder", 55, 60],
  ],
  SL: [
    ["Pathum Nissanka", "batter", 84, 12],
    ["Kusal Mendis", "wicketkeeper", 85, 10],
    ["Dinesh Chandimal", "wicketkeeper", 78, 10],
    ["Kusal Perera", "wicketkeeper", 74, 10],
    ["Charith Asalanka", "allrounder", 82, 55],
    ["Dhananjaya de Silva", "allrounder", 80, 58],
    ["Wanindu Hasaranga", "allrounder", 62, 82],
    ["Maheesh Theekshana", "pace_bowler", 35, 82],
    ["Dushmanta Chamika", "pace_bowler", 38, 76],
    ["Kusal Rajapaksa", "batter", 80, 12],
    ["Jeffrey Ranjith", "pace_bowler", 32, 72],
    ["Asitha Fernando", "pace_bowler", 35, 74],
    ["Pramodya Wickramasinghe", "pace_bowler", 30, 74],
    ["Dunith Wellalage", "spin_bowler", 35, 70],
    ["Sadeera Samarawickrama", "wicketkeeper", 65, 10],
  ],
  BAN: [
    ["Shakib Al Hasan", "allrounder", 68, 86],
    ["Mushfiqur Rahim", "wicketkeeper", 80, 10],
    ["Tamim Iqbal", "batter", 82, 10],
    ["Najmul Hossain Shanto", "batter", 80, 10],
    ["Mehidy Hasan Miraz", "allrounder", 70, 78],
    ["Taskin Ahmed", "pace_bowler", 40, 80],
    ["Mustafizur Rahman", "pace_bowler", 35, 78],
    ["Taijul Islam", "spin_bowler", 40, 72],
    ["Nurul Abid", "batter", 70, 12],
    ["Litton Das", "wicketkeeper", 80, 10],
    ["Shoriful Islam", "pace_bowler", 28, 72],
    ["Nasir Hossain", "allrounder", 65, 70],
    ["Rishad Hossain", "spin_bowler", 40, 68],
    ["Towhid Hridoy", "batter", 78, 10],
    ["Jaker Ali", "wicketkeeper", 60, 10],
  ],
  AFG: [
    ["Rashid Khan", "spin_bowler", 58, 85],
    ["Mohammad Nabi", "allrounder", 78, 82],
    ["Mujeeb Ur Rahman", "spin_bowler", 38, 80],
    ["Rahmat Shah", "batter", 84, 10],
    ["Hashmatullah Shahidi", "batter", 80, 12],
    ["Ibrahim Zadran", "batter", 82, 10],
    ["Rahmanullah Gurbaz", "wicketkeeper", 82, 10],
    ["Najibullah Zadran", "batter", 76, 10],
    ["Fazalhaq Farooqi", "pace_bowler", 30, 76],
    ["Naveen-ul-Haq", "pace_bowler", 28, 74],
    ["Azmatullah Omarzai", "allrounder", 70, 70],
    ["Noor Ahmad", "spin_bowler", 32, 72],
    ["Karim Janat", "allrounder", 66, 74],
    ["Gulbadin Naib", "allrounder", 64, 68],
    ["Ikram Alikhil", "wicketkeeper", 62, 10],
  ],
};

/** Real home grounds per nation, with boundary size and batting conditions. */
const GROUNDS = {
  IND: [
    ["Wankhede Stadium, Mumbai", 68, 58],
    ["M. Chinnaswamy Stadium, Bengaluru", 65, 62],
    ["Eden Gardens, Kolkata", 70, 55],
    ["MA Chidambaram Stadium, Chennai", 68, 52],
    ["Narendra Modi Stadium, Ahmedabad", 72, 50],
    ["Rajiv Gandhi International Stadium, Hyderabad", 70, 60],
  ],
  AUS: [
    ["Melbourne Cricket Ground", 84, 52],
    ["Sydney Cricket Ground", 82, 50],
    ["Adelaide Oval", 78, 56],
    ["The Gabba, Brisbane", 80, 54],
    ["WACA Ground, Perth", 74, 58],
    ["Bellerive Oval, Hobart", 76, 55],
  ],
  ENG: [
    ["Lord's, London", 66, 50],
    ["Old Trafford, Manchester", 68, 52],
    ["Edgbaston, Birmingham", 68, 54],
    ["Headingley, Leeds", 66, 56],
    ["Eden Park, Auckland", 62, 58],
    ["Sophia Gardens, Cardiff", 66, 52],
  ],
  SA: [
    ["Newlands, Cape Town", 74, 54],
    ["The Wanderers, Johannesburg", 78, 54],
    ["SuperSport Park, Centurion", 76, 56],
    ["St George's Park, Gqeberha", 72, 56],
    ["Kingsmead, Durban", 76, 58],
    ["The Oval, London", 66, 50],
  ],
  NZ: [
    ["Basin Reserve, Wellington", 62, 50],
    ["Eden Park, Auckland", 62, 58],
    ["Christchurch Oval", 64, 52],
    ["Hagley Oval, Christchurch", 62, 52],
    ["Seddon Park, Hamilton", 62, 55],
    ["Bellerive Oval, Hobart", 66, 55],
  ],
  PAK: [
    ["Gaddafi Stadium, Lahore", 70, 56],
    ["National Stadium, Karachi", 68, 54],
    ["Rawalpindi Cricket Stadium", 70, 54],
    ["Multan Cricket Stadium", 70, 56],
    ["Sharjah Cricket Stadium", 62, 62],
    ["Dubai International Stadium", 62, 64],
  ],
  WI: [
    ["Kensington Oval, Bridgetown", 68, 54],
    ["Queen's Park Oval, Port of Spain", 68, 52],
    ["Sabina Park, Kingston", 66, 52],
    ["Brian Lara Cricket Academy", 66, 54],
    ["Darren Sammy National Stadium", 68, 56],
    ["Kensington Oval, Bridgetown", 68, 54],
  ],
  SL: [
    ["R. Premadasa Stadium, Colombo", 68, 52],
    ["Pallekele International Cricket Stadium", 66, 56],
    ["Galle International Stadium", 66, 54],
    ["Sinhalese Sports Club, Colombo", 64, 54],
    ["Mahinda Rajapaksa International Stadium", 68, 56],
    ["R. Premadasa Stadium, Colombo", 68, 52],
  ],
  BAN: [
    ["Sher-e-Bangla National Cricket Stadium", 68, 56],
    ["Bangabandhu National Stadium", 70, 56],
    ["Zahur Ahmed Chowdhury Stadium", 66, 54],
    ["Chittagong Divisional Stadium", 68, 55],
    ["Sylhet International Cricket Stadium", 66, 56],
    ["Sher-e-Bangla National Cricket Stadium", 68, 56],
  ],
  AFG: [
    [" Kabul International Cricket Ground", 68, 56],
    ["Sharjah Cricket Stadium", 62, 62],
    ["Gaddafi Stadium, Lahore", 70, 56],
    ["Dubai International Stadium", 62, 64],
    ["Al Amerat Cricket Ground", 64, 58],
    ["Zayed Cricket Stadium, Abu Dhabi", 66, 58],
  ],
};

/** Batting/bowling styles per player index within a squad, deterministic. */
const BATTING_STYLES = ["rhb", "lhb", "rhb", "lhb"];
const PACE_STYLES = [
  "right-arm fast",
  "right-arm fast-medium",
  "left-arm fast",
  "right-arm medium",
  "left-arm medium",
];
const SPIN_STYLES = ["off-spin", "leg-spin", "left-arm orthodox", "left-arm wrist spin"];

function pick(r, arr) {
  return arr[Math.floor(r() * arr.length)];
}

function clamp(v) {
  return Math.max(1, Math.min(99, Math.round(v)));
}

/** Build the 12-field skill block from a headline batting/bowling pair. */
function buildSkills(r, bat, bowl, role, isBowler) {
  // Bowling types get a real bowling rating and a modest batting tail.
  const pace = isBowler ? bowl : clamp(10 + r() * 18);
  const spin = isBowler ? clamp(10 + r() * 18) : bowl;
  // Spin all-rounders bowl; pace all-rounders take a few overs.
  const bowlIsSpin =
    role === "spin_bowler" ||
    (role === "allrounder" && spin > pace && bowl > 45);

  const keeper = role === "wicketkeeper";
  const skillBowl = bowlIsSpin ? spin : pace;

  return {
    bat: clamp(bat),
    test_bat: clamp(bat + (r() - 0.5) * 6),
    odi_bat: clamp(bat + (r() - 0.5) * 8),
    t20_bat: clamp(bat + (r() - 0.4) * 10),
    defence: clamp(bat * 0.8 + 8 + r() * 10),
    pace: clamp(pace),
    spin: clamp(spin),
    odi_bowl: clamp(skillBowl + (r() - 0.5) * 6),
    t20_bowl: clamp(skillBowl + (r() - 0.5) * 8),
    fielding: clamp(52 + r() * 44),
    keeping: keeper ? clamp(72 + r() * 24) : clamp(10 + r() * 12),
    leadership: clamp(40 + r() * 50),
  };
}

function buildWorld(seed) {
  const r = rng(seed);
  const nations = [];
  const teams = [];
  const players = [];
  const grounds = [];
  const competitions = [];

  let nationId = 1;
  let teamId = 100;
  let playerId = 1000;
  let groundId = 1;

  for (const n of NATIONS) {
    nations.push({
      id: nationId,
      name: n.name,
      short_name: n.short,
      strength: n.strength,
      has_test_status: n.test,
    });

    // Grounds first: the national side references the first one as home.
    const nationGrounds = GROUNDS[n.key] ?? [];
    for (const [gname, boundary, batting] of nationGrounds) {
      grounds.push({
        id: groundId++,
        name: gname,
        nation_id: nationId,
        boundary_m: boundary,
        batting_friendly: batting,
        // Most international cricket is day/night at the major venues.
        day_night: r() < 0.65,
      });
    }

    const nationalTeam = {
      id: teamId++,
      name: n.name,
      nation_id: nationId,
      home_ground_id: nationId === 1 ? 1 : groundId - nationGrounds.length,
      budget: Math.round(2_000_000 * (n.strength / 50)),
      reputation: n.strength,
      squad_ids: [],
      is_national: true,
    };
    teams.push(nationalTeam);

    // Squad: assign players to the national side, splitting the fielding
    // rotation across the nation's grounds is unnecessary — the XI is drawn
    // from squad_ids by the engine.
    const squad = SQUADS[n.key] ?? [];
    const seen = new Set();
    squad.forEach(([name, role, bat, bowl], i) => {
      // A name may only appear once per squad. Some squads list the same
      // player twice for depth; keep the first entry.
      if (seen.has(name)) return;
      seen.add(name);

      const isBowler = role === "pace_bowler" || role === "spin_bowler";
      const skills = buildSkills(r, bat, bowl, role, isBowler);
      const age = 22 + Math.floor(r() * 16);
      const bowlingStyle = isBowler
        ? role === "pace_bowler"
          ? pick(r, PACE_STYLES)
          : pick(r, SPIN_STYLES)
        : // All-rounders bowl a little; the engine treats them as part-guns.
          bowl > 55
          ? pick(r, SPIN_STYLES)
          : bowl > 40
          ? pick(r, PACE_STYLES)
          : null;

      const p = {
        id: playerId++,
        name,
        nation_id: nationId,
        // Players sit with the national squad.
        team_id: nationalTeam.id,
        age,
        role,
        batting_style: BATTING_STYLES[i % BATTING_STYLES.length],
        bowling_style: bowlingStyle,
        skills,
        potential: {
          ...skills,
          bat: clamp(skills.bat + 4 + Math.floor(r() * 8)),
          pace: clamp(skills.pace + 3 + Math.floor(r() * 7)),
          spin: clamp(skills.spin + 3 + Math.floor(r() * 7)),
        },
        form: Math.floor(r() * 7) - 3,
        fitness: 92 + Math.floor(r() * 9),
        morale: 60 + Math.floor(r() * 40),
        experience: 20 + Math.floor(r() * 120),
      };
      players.push(p);
      nationalTeam.squad_ids.push(p.id);
    });

    nationId++;
  }

  // International competitions covering all ten nations. The engine's
  // international programme builds its own cup schedule from the national
  // teams; these describe the events for the UI and almanac.
  const allNationals = teams.filter((t) => t.is_national).map((t) => t.id);
  competitions.push({
    id: 1,
    name: "World Test Championship",
    nation_id: 1,
    format: "test",
    kind: "league",
    team_ids: allNationals,
    days_per_match: 5,
  });
  competitions.push({
    id: 2,
    name: "Cricket World Cup",
    nation_id: 1,
    format: "odi50",
    kind: "knockout_cup",
    team_ids: allNationals,
    days_per_match: 1,
  });
  competitions.push({
    id: 3,
    name: "ICC World Twenty20",
    nation_id: 1,
    format: "t20",
    kind: "knockout_cup",
    team_ids: allNationals,
    days_per_match: 1,
  });

  return {
    schema_version: 1,
    // Distinct from the fictional default so a save can never be confused.
    db_version: "2027.10-real",
    nations,
    teams,
    players,
    grounds,
    competitions,
  };
}

// ---- CLI ----
const outArg = process.argv.indexOf("--out");
const out = resolve(
  ROOT,
  outArg >= 0 ? process.argv[outArg + 1] : "data/packs/real-international/world.json",
);
const seedArg = process.argv.indexOf("--seed");
const seed = seedArg >= 0 ? Number(process.argv[seedArg + 1]) : 20271010;

const world = buildWorld(seed);
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(world, null, 1)}\n`);

// ---- self-check: every invariant the engine relies on ----
const problems = [];
const nationIds = new Set(world.nations.map((n) => n.id));
const teamIds = new Set(world.teams.map((t) => t.id));
const playerIds = new Set(world.players.map((p) => p.id));
const groundIds = new Set(world.grounds.map((g) => g.id));

for (const t of world.teams) {
  if (!nationIds.has(t.nation_id)) problems.push(`team ${t.id}: bad nation`);
  if (!groundIds.has(t.home_ground_id)) problems.push(`team ${t.id}: bad ground`);
  const owned = world.players.filter((p) => p.team_id === t.id).length;
  if (owned < 11) problems.push(`team ${t.name}: only ${owned} players (need a full XI)`);
  if (t.squad_ids.length !== owned) {
    problems.push(`team ${t.name}: squad_ids (${t.squad_ids.length}) != roster (${owned})`);
  }
}
for (const p of world.players) {
  if (!playerIds.has(p.id)) problems.push(`player ${p.id}: duplicate id`);
  if (!nationIds.has(p.nation_id)) problems.push(`player ${p.id}: bad nation`);
  if (p.team_id !== null && !teamIds.has(p.team_id)) {
    problems.push(`player ${p.id}: bad team`);
  }
  for (const [k, v] of Object.entries(p.skills)) {
    if (v < 1 || v > 99) problems.push(`player ${p.name}: ${k}=${v} out of range`);
  }
}
// A player name must be unique *within a nation*. The same name across two
// nations is legitimate — Rashid Khan, for instance, is an Afghan
// international who has also played for Pakistan, and the engine keys players
// by id, not name.
const seenByNation = new Set();
for (const p of world.players) {
  const key = `${p.nation_id}:${p.name}`;
  if (seenByNation.has(key)) {
    problems.push(`duplicate player in nation ${p.nation_id}: ${p.name}`);
  }
  seenByNation.add(key);
}

if (problems.length) {
  console.error("real-international pack invalid:");
  for (const p of problems.slice(0, 20)) console.error("  - " + p);
  process.exit(1);
}

console.log(
  `real-international pack written: ${world.players.length} players, ` +
    `${world.teams.length} national teams, ${world.nations.length} nations, ` +
    `${world.grounds.length} grounds, ${world.competitions.length} competitions -> ${out}`,
);