#!/usr/bin/env node
/**
 * tools/generate-world.mjs
 *
 * Deterministically generates the fictional 2027 world database and writes
 * `data/world.json`. Fictional names only — see specs/requirement.md §4.
 *
 * Usage: node tools/generate-world.mjs [--seed 2027]
 */

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");

// Mulberry32: small deterministic PRNG.
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

const NATIONS = [
  { key: "Kear", name: "Kearland", short: "KRX", strength: 92, test: true },
  { key: "Ter", name: "Terrance", short: "TRC", strength: 84, test: true },
  { key: "Viv", name: "Vivania", short: "VVN", strength: 78, test: true },
  { key: "Nor", name: "Norwich Isles", short: "NWI", strength: 74, test: true },
  { key: "Aur", name: "Aurelia", short: "ARL", strength: 68, test: true },
  { key: "Sab", name: "Sabara", short: "SBR", strength: 63, test: true },
  { key: "Mer", name: "Mercia", short: "MRX", strength: 60, test: false },
  { key: "Cal", name: "Calderon", short: "CLD", strength: 57, test: false },
  { key: "Zam", name: "Zambora", short: "ZMB", strength: 55, test: false },
  { key: "Hel", name: "Helvetia", short: "HLV", strength: 50, test: false },
  { key: "Ind", name: "Indrang", short: "IDG", strength: 48, test: false },
  { key: "Oce", name: "Oceania", short: "OAN", strength: 44, test: false },
];

// Fictional first/last name pools, deliberately heterogeneous.
const FIRST = {
  Kear: ["Ares", "Brant", "Colm", "Doran", "Egan", "Fintan", "Garran", "Hale", "Ivor", "Jarek", "Kellan", "Lorcan", "Maddox", "Nolan", "Orson", "Padraig", "Quinn", "Rory", "Seamus", "Tobin"],
  Ter: ["Alaric", "Bastian", "Corvin", "Darius", "Emrys", "Florian", "Gideon", "Hugo", "Ignatius", "Jonas", "Kasper", "Leander", "Maxim", "Nikolai", "Oskar", "Piotr", "Rasmus", "Stefan", "Tomas", "Viktor"],
  Viv: ["Ambrose", "Benedict", "Cyril", "Desmond", "Erasmus", "Fabian", "Giles", "Horace", "Isadore", "Julius", "Klaus", "Lucius", "Marius", "Norbert", "Ozwald", "Pliny", "Quintus", "Rufus", "Sylvan", "Theodore"],
  Nor: ["Ainsley", "Bodhi", "Caspian", "Dane", "Elowen", "Fjord", "Gunnar", "Haldor", "Ingrid", "Joren", "Kolbrun", "Lars", "Mikkel", "Njal", "Osvald", "Petra", "Ragna", "Sigrid", "Tove", "Ulrik"],
  Aur: ["Aurel", "Bruno", "Cesar", "Domenico", "Enzo", "Fabrizio", "Gianni", "Hector", "Iker", "Joaquin", "Kiko", "Lorenzo", "Mateo", "Nico", "Osvaldo", "Pablo", "Rafa", "Sergio", "Toro", "Vito"],
  Sab: ["Adnan", "Bilal", "Cyrus", "Dario", "Emir", "Farid", "Ghassan", "Hamid", "Ishaan", "Jabar", "Kamran", "Latif", "Mahesh", "Nadir", "Omar", "Piruz", "Rashid", "Sahil", "Tariq", "Yusuf"],
  Mer: ["Anton", "Blaise", "Cedric", "Damien", "Etienne", "Fabrice", "Gaston", "Hugues", "Isidore", "Jerome", "Karim", "Lionel", "Manuel", "Noel", "Olivier", "Pascal", "Quentin", "Remi", "Serge", "Thierry"],
  Cal: ["Alejandro", "Bruno", "Camilo", "Diego", "Esteban", "Felipe", "Gabriel", "Hernan", "Ignacio", "Javier", "Kevin", "Lautaro", "Matias", "Nicolas", "Oscar", "Patricio", "Rodrigo", "Santino", "Tomas", "Valentin"],
  Zam: ["Andile", "Bongani", "Chuma", "Dumisani", "Emeka", "Femi", "Gugu", "Hlengiwe", "Ikenna", "Jabulani", "Kagiso", "Lwazi", "Mpho", "Nkosi", "Obi", "Phumzile", "Sipho", "Thabo", "Unathi", "Vusi"],
  Hel: ["Adrian", "Bernhard", "Caspar", "Dominik", "Emil", "Fabian", "Gregor", "Hannes", "Jonas", "Kilian", "Lukas", "Marco", "Nils", "Othmar", "Peter", "Ralph", "Simon", "Timon", "Urs", "Vincent"],
  Ind: ["Aarav", "Bhavesh", "Chetan", "Devansh", "Eshan", "Farhan", "Girish", "Harsh", "Ishan", "Jayanth", "Kiran", "Lokesh", "Manav", "Nakul", "Omkar", "Pranav", "Rajesh", "Sameer", "Tanmay", "Vikram"],
  Oce: ["Aidan", "Bodhi", "Connell", "Dylan", "Ethan", "Flynn", "Grady", "Hemi", "Isaac", "Jack", "Koa", "Lachlan", "Manu", "Nikau", "Ollie", "Pita", "Rangi", "Sam", "Tane", "Wiremu"],
};
const LAST = {
  Kear: ["Ashcombe", "Ballinderry", "Carmichael", "Dunmore", "Eastwood", "Fairbrother", "Glenfield", "Hartleigh", "Inglewood", "Jarrowdale", "Kilbride", "Larkmere", "Mowbray", "Northgate", "Oakhurst", "Peverley", "Quarryhill", "Redmayne", "Stonewell", "Thornbury"],
  Ter: ["Aldenberg", "Brandt", "Corvath", "Drechsler", "Eberhardt", "Falkner", "Gruber", "Hoffstetter", "Iversen", "Jahnke", "Kronberg", "Lindqvist", "Meissner", "Nordhaus", "Osterman", "Prantl", "Rosenkranz", "Sorensen", "Timmermann", "Voss"],
  Viv: ["Alborne", "Bisset", "Calder", "Duval", "Everard", "Florin", "Grevill", "Halloran", "Ismer", "Jorrand", "Kestrel", "Lumier", "Marchetti", "Navarre", "Ozan", "Perrault", "Quiron", "Ravanel", "Saintclair", "Thibaudeau"],
  Nor: ["Asklund", "Bjerke", "Coldfield", "Dahlburg", "Eiriksen", "Fjeldstad", "Gulliksen", "Havn", "Isberg", "Jansson", "Kirsten", "Lindgren", "Molvik", "Norheim", "Ovesen", "Petersen", "Quisling", "Rasmussen", "Strandvik", "Thorvald"],
  Aur: ["Altamura", "Bianchi", "Carrada", "Delgado", "Escudero", "Ferrante", "Galante", "Herrera", "Ibarra", "Jimeno", "Kazares", "Liendo", "Mancini", "Nogales", "Olmedo", "Paredes", "Quintela", "Roldan", "Salvatore", "Trevino"],
  Sab: ["Afshar", "Bahari", "Chehreh", "Dabiri", "Entezam", "Farrokh", "Ghazali", "Hashemi", "Iravani", "Jamshidi", "Karimi", "Lotfi", "Mesbah", "Nazari", "Oskooei", "Pahlavi", "Qaderi", "Rahmani", "Soleimani", "Tehrani"],
  Mer: ["Auberon", "Belmont", "Chastain", "Dumoulin", "Escoffier", "Fontaine", "Girardin", "Hennequin", "Ivoire", "Jauffret", "Kermeur", "Lavalle", "Montagne", "Noiret", "Osselin", "Pellerin", "Quercy", "Renaudin", "Sauvage", "Tournier"],
  Cal: ["Aguilar", "Barrantes", "Cardoso", "Delacruz", "Esquivel", "Fuentes", "Granados", "Herreros", "Ibarrola", "Juarez", "Kawai", "Lamas", "Montoya", "Negrete", "Ortiz", "Pineda", "Quintero", "Reynoso", "Salcedo", "Valdez"],
  Zam: ["Achieng", "Baloyi", "Chikafu", "Dhlomo", "Ekwueme", "Falana", "Gumede", "Hlongwane", "Idowu", "Juma", "Kabwe", "Lubega", "Mbeki", "Ndlovu", "Okonkwo", "Phiri", "Qacha", "Radebe", "Sibanda", "Tshabalala"],
  Hel: ["Affolter", "Burckhardt", "Cadonau", "Dolder", "Ebneter", "Frutiger", "Gutknecht", "Hochuli", "Imboden", "Jost", "Kobel", "Liechti", "Mettler", "Nothiger", "Omlin", "Peyer", "Quadri", "Reusser", "Steiner", "Twerenbold"],
  Ind: ["Acharya", "Bhattacharya", "Chauhan", "Deshpande", "Engineer", "Fernandes", "Gokhale", "Harikumar", "Iyengar", "Jadhav", "Kulkarni", "Lakshman", "Menon", "Nadkarni", "Oberoi", "Prabhakar", "Qureshi", "Ranganathan", "Sundaram", "Thakur"],
  Oce: ["Ariki", "Broughton", "Coleman", "Devereux", "Ellison", "Fitzgerald", "Greenwood", "Hollis", "Ihaka", "Jenkins", "Kaipara", "Lomax", "Manihera", "Ngata", "O'Sullivan", "Pearce", "Quilty", "Rutherford", "Sorenson", "Tamihana"],
};

const CITY = {
  Kear: ["Kearstown", "Port Vale", "Redhaven", "Highgate"],
  Ter: ["Terrence City", "Novaholm", "Braunfell", "Ludwigsberg"],
  Viv: ["Vivian Bay", "Port Aubert", "Saintclair", "Marlbrook"],
  Nor: ["Nordvik", "Fjordheim", "Bregstadt", "Kalvnes"],
  Aur: ["Aurelia", "Port Sable", "Monte Cal", "Viento"],
  Sab: ["Sabara Central", "Port Farid", "Zireh", "Baharon"],
  Mer: ["Mercia Park", "Vieuxport", "Châteauvert", "Bordeaux"],
  Cal: ["Calderon", "Puerto Fuego", "Lomas", "Valcor"],
  Zam: ["Zambora Town", "Kabwe Bay", "Nkosi Park", "Mbeki Field"],
  Hel: ["Helvetia", "Alpenstadt", "Lakeside", "Voralpen"],
  Ind: ["Indrang", "Kaveri", "New Bombay", "Ghatkot"],
  Oce: ["Oceania Park", "Kaipara", "Wiremu Bay", "Taneville"],
};

const BATTING_STYLES = ["rhb", "lhb"];
const BOWL_STYLES = {
  pace: ["right-arm fast", "right-arm fast-medium", "left-arm fast", "left-arm medium"],
  spin: ["off-spin", "leg-spin", "left-arm orthodox", "left-arm wrist spin"],
};

function pick(r, arr) {
  return arr[Math.floor(r() * arr.length)];
}

function weightedSkill(r, base, spread) {
  // Normal-ish distribution around `base`, clamped 15..99.
  const u = (r() + r() + r()) / 3; // central limit-ish
  const v = Math.round(base + (u - 0.5) * spread * 2);
  return Math.max(15, Math.min(99, v));
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

    const cityPool = CITY[n.key];
    const nationalTeam = {
      id: teamId++,
      name: n.name,
      nation_id: nationId,
      home_ground_id: groundId,
      budget: Math.round(2_000_000 * (n.strength / 50)),
      reputation: weightedSkill(r, n.strength, 8),
      squad_ids: [],
      is_national: true,
    };
    teams.push(nationalTeam);

    // Domestic teams: top-6 nations get 3 sides, others get 2.
    const domesticCount = n.strength >= 74 ? 3 : n.strength >= 55 ? 2 : 1;
    const domesticTeams = [];
    for (let d = 0; d < domesticCount; d++) {
      const t = {
        id: teamId++,
        name: `${cityPool[d % cityPool.length]} ${d === 0 ? "First XI" : d === 1 ? "Royals" : "United"}`,
        nation_id: nationId,
        home_ground_id: groundId + d,
        budget: Math.round(900_000 * (n.strength / 50)),
        reputation: weightedSkill(r, n.strength - 6 + d * 3, 8),
        squad_ids: [],
        is_national: false,
      };
      teams.push(t);
      domesticTeams.push(t);
    }

    for (let g = 0; g < domesticCount + 1; g++) {
      grounds.push({
        id: groundId++,
        name: `${cityPool[g % cityPool.length]} ${g === 0 ? "Stadium" : "Oval"}`,
        nation_id: nationId,
        boundary_m: 62 + Math.floor(r() * 18),
        batting_friendly: weightedSkill(r, 50, 14),
        day_night: r() < 0.4,
      });
    }

    // Squad: enough for full domestic XIs (12 per domestic team); the
    // national side selects from the whole nation pool. Skills scale to strength.
    const baseBat = Math.round(n.strength * 0.92);
    const baseBowl = Math.round(n.strength * 0.88);
    // snake_case to match Rust Role serde rename_all.
    const roles = ["batter", "wicketkeeper", "allrounder", "allrounder", "pace_bowler", "spin_bowler"];
    const squadSize = domesticCount * 12;
    for (let i = 0; i < squadSize; i++) {
      const first = pick(r, FIRST[n.key]);
      const last = pick(r, LAST[n.key]);
      const role = roles[Math.min(i, roles.length - 1)];
      const isBowler = role === "pace_bowler" || role === "spin_bowler";
      const bat = weightedSkill(r, isBowler ? baseBat - 22 : baseBat - 3, 14);
      const bowlSkill = weightedSkill(r, isBowler ? baseBowl : baseBowl - 26, 12);
      const spin = role === "spin_bowler" || (role === "allrounder" && r() < 0.4);
      const age = 19 + Math.floor(r() * 18);

      const skills = {
        bat,
        test_bat: weightedSkill(r, bat, 10),
        odi_bat: weightedSkill(r, bat, 10),
        t20_bat: weightedSkill(r, bat, 10),
        defence: weightedSkill(r, bat, 12),
        pace: spin ? 15 : bowlSkill,
        spin: spin ? bowlSkill : 15,
        odi_bowl: weightedSkill(r, bowlSkill, 10),
        t20_bowl: weightedSkill(r, bowlSkill, 12),
        fielding: weightedSkill(r, 55, 20),
        keeping: role === "wicketkeeper" ? weightedSkill(r, 72, 10) : 20,
        leadership: weightedSkill(r, 45, 20),
      };
      const team = domesticTeams[i % domesticTeams.length];
      const p = {
        id: playerId++,
        name: `${first} ${last}`,
        nation_id: nationId,
        team_id: team.id,
        age,
        role,
        batting_style: pick(r, BATTING_STYLES),
        bowling_style: spin ? pick(r, BOWL_STYLES.spin) : bowlSkill > 45 ? pick(r, BOWL_STYLES.pace) : null,
        skills,
        potential: {
          ...skills,
          bat: Math.min(99, bat + 6 + Math.floor(r() * 8)),
          pace: Math.min(99, skills.pace + 5 + Math.floor(r() * 8)),
          spin: Math.min(99, skills.spin + 5 + Math.floor(r() * 8)),
        },
        form: Math.floor(r() * 7) - 3,
        fitness: 92 + Math.floor(r() * 9),
        morale: 55 + Math.floor(r() * 45),
        experience: Math.floor(r() * 90),
      };
      players.push(p);
      team.squad_ids.push(p.id);
      nationalTeam.squad_ids.push(p.id);
    }
    nationId++;
  }

  // Domestic competitions: one T20 cup (top domestic sides) + one 50-over league
  // + a championship for the top-6 nations.
  for (const n of NATIONS.slice(0, 6)) {
    const nat = nations.find((x) => x.name === n.name);
    const domTeams = teams.filter((t) => t.nation_id === nat.id && !t.is_national);
    if (domTeams.length >= 2) {
      competitions.push({
        id: competitions.length + 1,
        name: `${n.name} T20 Carnival`,
        nation_id: nat.id,
        format: "t20",
        kind: "league",
        team_ids: domTeams.map((t) => t.id),
        days_per_match: 1,
      });
      competitions.push({
        id: competitions.length + 1,
        name: `${n.name} Fifty League`,
        nation_id: nat.id,
        format: "odi50",
        kind: "league",
        team_ids: domTeams.map((t) => t.id),
        days_per_match: 1,
      });
    }
  }

  return {
    schema_version: 1,
    db_version: "2027.01",
    nations,
    teams,
    players,
    grounds,
    competitions,
  };
}

const seedArg = process.argv.indexOf("--seed");
const seed = seedArg >= 0 ? Number(process.argv[seedArg + 1]) : 20270101;
const world = buildWorld(seed);

const out = resolve(ROOT, "data/world.json");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify(world, null, 1)}\n`);

const count = (k) => world[k].length;
console.log(
  `world written: ${count("players")} players, ${count("teams")} teams, ` +
    `${count("nations")} nations, ${count("grounds")} grounds, ${count("competitions")} competitions`,
);
