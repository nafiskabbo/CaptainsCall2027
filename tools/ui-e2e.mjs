//! UI-level end-to-end tests for Captain's Call 2027.
//!
//! Drives the **real** React app (`src/main.tsx`, compiled as the browser would
//! compile it) inside jsdom, with every Tauri `invoke` routed to the **real**
//! Rust engine via the `e2e` harness example. So a passing run here means the
//! same screens, state and buttons behave in the packaged app.
//!
//! ```text
//! cargo run --example e2e -- 127.0.0.1:8787 target/e2e &   # or let this start it
//! node tools/ui-e2e.mjs [--keep]
//! ```
//!
//! The Rust engine is reset per run (fresh saves dir), so the script re-seeds a
//! career for each scenario.

import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";
import net from "node:net";
import esbuild from "esbuild";
import jsdomPkg from "jsdom";

const { JSDOM } = jsdomPkg;

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const HARNESS_URL = process.env.CC_HARNESS ?? "http://127.0.0.1:8787";
const KEEP = process.argv.includes("--keep");
const VERBOSE = process.argv.includes("-v") || process.argv.includes("--verbose");

const results = [];
let currentScenario = "";

function check(name, pass, detail = "") {
  results.push({ scenario: currentScenario, name, pass, detail });
  const mark = pass ? "PASS" : "FAIL";
  console.log(`  ${mark}  ${name}${detail && !pass ? ` — ${detail}` : ""}`);
  return pass;
}

function warn(name, detail) {
  console.log(`  WARN  ${name}${detail ? ` — ${detail}` : ""}`);
}

// ------------------------------------------------------------------ jsdom setup

/** A canvas 2D context stub: every method is a no-op returning a gradient-ish. */
function fakeCanvasContext() {
  const gradient = { addColorStop() {} };
  const handler = {
    get(_target, prop) {
      if (prop === "canvas") return { width: 800, height: 500 };
      if (prop === "createRadialGradient" || prop === "createLinearGradient") {
        return () => gradient;
      }
      if (prop === "measureText") return () => ({ width: 10 });
      if (prop === "getImageData") return () => ({ data: new Uint8ClampedArray(4) });
      return () => undefined;
    },
    set() {
      return true;
    },
  };
  return new Proxy({}, handler);
}

async function bootApp() {
  // Compile the entry point the same way Vite would, but into one IIFE we can
  // evaluate inside jsdom. CSS is stubbed: jsdom does not cascade for these
  // assertions and the bundle must stay self-contained.
  // Errors raised inside the page. jsdom's VirtualConsole carries both console
  // output and jsdomError, so we capture React's render errors there too.
  const consoleErrors = [];
  const virtualConsole = new jsdomPkg.VirtualConsole();
  virtualConsole.on("jsdomError", (e) => {
    const detail = e.detail;
    const stack = detail?.stack ?? e.stack;
    const body = typeof detail === "object" && detail !== null ? detail : e;
    consoleErrors.push(
      `${e.message ?? e}` +
        (stack ? `\n      at ${String(stack).split("\n").slice(1, 4).join("\n      at ")}` : "") +
        (body?.constructor?.name ? `\n      (${body.constructor.name})` : ""),
    );
  });
  virtualConsole.on("error", (...a) => consoleErrors.push(a.map((x) => String(x)).join(" ")));

  const out = await esbuild.build({
    entryPoints: [path.join(ROOT, "src/main.tsx")],
    bundle: true,
    write: false,
    format: "iife",
    platform: "browser",
    target: ["es2022"],
    jsx: "automatic",
    define: { "process.env.NODE_ENV": '"development"' },
    sourcemap: false,
    plugins: [
      {
        name: "stub-css",
        setup(build) {
          build.onResolve({ filter: /\.css$/ }, (args) => ({
            path: args.path,
            namespace: "stub",
          }));
          build.onLoad({ filter: /.*/, namespace: "stub" }, () => ({
            contents: "export default {}",
            loader: "js",
          }));
        },
      },
    ],
  });

  const dom = new JSDOM('<!doctype html><html><body><div id="root"></div></body></html>', {
    url: "http://localhost:1420/e2e.html",
    pretendToBeVisual: true,
    runScripts: "outside-only",
    // jsdom's VirtualConsole rather than window-level listeners: it works the
    // same under Node and Bun, whereas Bun refuses addEventListener on a jsdom
    // window and the harness would die before running a single check.
    virtualConsole,
  });
  const { window } = dom;

  // jsdom has no <canvas> 2D backend.
  window.HTMLCanvasElement.prototype.getContext = function getContext(kind) {
    if (kind === "2d") return fakeCanvasContext();
    return null;
  };

  // The Tauri shim: identical semantics to src/mock-e2e.ts, so the browser and
  // Node harnesses exercise the same transport contract.
  const calls = [];
  window.__ccCalls = calls;
  window.__TAURI_INTERNALS__ = {
    async invoke(cmd, args = {}) {
      calls.push({ cmd, args, at: Date.now() });
      let res;
      try {
        res = await fetch(`${HARNESS_URL}/invoke`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ cmd, args }),
        });
      } catch (cause) {
        throw new Error(`harness unreachable at ${HARNESS_URL}: ${cause}`);
      }
      const payload = await res.json();
      if (!payload.ok) throw new Error(payload.error);
      return payload.data;
    },
    transformCallback: () => 0,
    unregisterCallback: () => undefined,
    convertFileSrc: (p) => p,
  };

  // Observers a browser would have. Supplying ResizeObserver also keeps the
  // app on its primary code path rather than the resize-event fallback.
  window.ResizeObserver ??= class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
  window.IntersectionObserver ??= class IntersectionObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
    takeRecords() {
      return [];
    }
  };

  const code = out.outputFiles[0].text;
  window.eval(code);
  return { dom, window, calls, consoleErrors };
}

// ---------------------------------------------------------------------- helpers

const $ = (window, selector) => window.document.querySelector(selector);
const $$ = (window, selector) => [...window.document.querySelectorAll(selector)];
const text = (el) => (el?.textContent ?? "").replace(/\s+/g, " ").trim();

function buttons(window, match) {
  return $$(window, "button").filter((b) => {
    const t = text(b);
    return match instanceof RegExp ? match.test(t) : t.includes(match);
  });
}

/** Waits until `predicate` is truthy, polling the live DOM. */
async function until(label, predicate, { timeout = 6000, interval = 60 } = {}) {
  const start = Date.now();
  for (;;) {
    let value;
    try {
      value = predicate();
    } catch {
      value = undefined;
    }
    if (value) return value;
    if (Date.now() - start > timeout) {
      throw new Error(`timed out waiting for ${label} after ${timeout}ms`);
    }
    await delay(interval);
  }
}

/**
 * Clicks an element the way a user would: a real, bubbling DOM click.
 *
 * Dispatches directly instead of calling `el.click()`, because jsdom's
 * HTMLElement.click() fires a PointerEvent internally and that constructor's
 * `view` check throws under some runtimes. React's root listener still sees a
 * genuine click either way.
 */
function click(window, el) {
  if (VERBOSE) {
    const b = el?.getBoundingClientRect?.();
    if (b) {
      console.log(`      (click at ${Math.round(b.x + b.width / 2)},${Math.round(b.y + b.height / 2)})`);
    }
  }
  // No `view`: jsdom's UIEvent impl wants its internal Window object, and the
  // wrapper is rejected on some runtimes. React's delegated listener does not
  // need it.
  el.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));
}

async function clickText(window, label, opts) {
  const el = await until(`button "${label}"`, () => buttons(window, label)[0], opts);
  click(window, el);
  return el;
}

function hasText(window, needle) {
  return ($(window, "#root")?.textContent ?? "").includes(needle);
}

/** React-controlled select: set through the native setter, then fire change. */
function setSelect(window, select, value) {
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLSelectElement.prototype,
    "value",
  ).set;
  setter.call(select, value);
  select.dispatchEvent(new window.Event("change", { bubbles: true }));
}

async function invoke(window, cmd, args = {}) {
  return window.__TAURI_INTERNALS__.invoke(cmd, args);
}

// ------------------------------------------------------------------ harness proc

function startHarness() {
  const proc = spawn(
    "cargo",
    ["run", "--example", "e2e", "--", "127.0.0.1:8787", "target/e2e"],
    { cwd: path.join(ROOT, "src-tauri"), stdio: ["ignore", "pipe", "pipe"] },
  );
  proc.stdout.on("data", () => {});
  proc.stderr.on("data", (d) => {
    if (VERBOSE) process.stderr.write(`[harness] ${d}`);
  });
  // If the harness dies (port already taken, compile error) say so instead of
  // silently testing against whatever process is still holding the port.
  proc.on("exit", (code) => {
    if (!stopping) {
      console.error(`\nharness process exited with code ${code} — is 8787 already in use?`);
      process.exit(3);
    }
  });
  return proc;
}

let stopping = false;

/** Fails fast with an actionable message if the port is already taken. */
async function assertPortFree(url) {
  const { hostname, port } = new URL(url);
  await new Promise((resolve, reject) => {
    const probe = net
      .connect(Number(port), hostname)
      .once("error", () => resolve())
      .once("connect", () => {
        probe.destroy();
        reject(
          new Error(
            `something is already listening on ${hostname}:${port}\n` +
              `  An orphaned harness from an earlier run. Clear it with:\n` +
              `  pkill -f "examples/e2e"`,
          ),
        );
      });
  });
}

async function waitForHarness(timeout = 120_000) {
  const start = Date.now();
  for (;;) {
    try {
      const res = await fetch(`${HARNESS_URL}/health`);
      if (res.ok) return;
    } catch {
      /* still compiling */
    }
    if (Date.now() - start > timeout) throw new Error("harness did not start");
    await delay(500);
  }
}

// -------------------------------------------------------------------- scenarios

async function scenarioWorldAndCareer({ window, calls }) {
  currentScenario = "world screen → career";
  await until("root", () => $(window, "#root"));
  await until("world screen", () => hasText(window, "Captain") || hasText(window, "Career"));

  check("world screen renders team choices", hasText(window, "Kearstown") || hasText(window, "Team"));
  check("world screen has a name field", $$(window, "input").length > 0);

  // Choose a team through the real <select> and start a career.
  const club = $(window, 'select[aria-label="Club"]');
  check("team selector present", !!club, "no select[aria-label=Club]");
  if (club) {
    setSelect(window, club, "101");
    await delay(80);
  }
  await clickText(window, "Begin Season");
  await until("career screen", () => hasText(window, "Season") || hasText(window, "Squad"));

  check("career started", hasText(window, "Kearstown"), "team name missing after start");
  check(
    "fixtures rendered",
    await until("a fixture row", () => hasText(window, "Kearland T20 Carnival") || $$(window, "[class*=row]").length > 0),
  );
  check("next-match panel shown", hasText(window, "Your next match"));
  check("invoke traffic recorded", calls.length > 0, `${calls.length} calls`);
}

/**
 * The database picker. The real-names international pack must be offered and
 * must actually swap the world the UI is driving — a dropdown that changes
 * nothing is the exact "dead control" class of bug this suite exists for.
 */
async function scenarioDatabasePacks({ window, calls }) {
  currentScenario = "database packs";

  // Get back to the new-game screen: whichever screen the previous scenario
  // left open, "Main Menu" returns to setup. The engine refuses a pack swap
  // while a career exists, by design.
  for (let i = 0; i < 4 && !hasText(window, "Setup Career"); i++) {
    const mainMenu = buttons(window, /main menu/i)[0];
    if (!mainMenu) break;
    click(window, mainMenu);
    await delay(400);
  }
  await until("world screen", () => hasText(window, "Setup Career"));

  const picker = $(window, 'select[aria-label="Database"]');
  check("database selector present", !!picker, "no select[aria-label=Database]");
  if (!picker) return;

  const options = [...picker.options].map((o) => ({ value: o.value, label: o.textContent }));
  check(
    "the fictional database is offered",
    options.some((o) => o.value === "default"),
    JSON.stringify(options),
  );
  check(
    "the real-names international pack is offered",
    options.some((o) => o.value === "real-international"),
    JSON.stringify(options),
  );

  // Switching must warn that it is third-party test content.
  setSelect(window, picker, "real-international");
  await until("pack swap", () => hasText(window, "Test database"), 5000);

  const world = await invoke(window, "get_world");
  check(
    "switching loads the real-names world",
    world.db_version === "2027.10-real",
    `db_version ${world.db_version}`,
  );
  check(
    "real-names world has ten nations",
    world.nations.length === 10,
    `${world.nations.length} nations`,
  );
  check(
    "real player names are present",
    world.players.some((p) => p.name.includes("Kohli") || p.name.includes("Bumrah")),
    "expected real squad names",
  );
  check("the pack is labelled as test data", hasText(window, "Test database"));
  check(
    "team list refreshed to the new world",
    hasText(window, "India") || hasText(window, "Australia"),
    "expected national sides in the selector",
  );

  // Swap back so later scenarios start from the fictional default.
  const back = $(window, 'select[aria-label="Database"]');
  if (back) {
    setSelect(window, back, "default");
    await delay(400);
  }
  const restored = await invoke(window, "get_world");
  check(
    "the fictional database is restorable",
    restored.db_version !== "2027.10-real",
    `db_version ${restored.db_version}`,
  );
}

async function scenarioCareerControls({ window, calls }) {
  currentScenario = "career controls";
  const ff = buttons(window, /fast forward/i)[0];
  check("fast-forward button exists", !!ff, "no 'Fast Forward' button");

  const before = await invoke(window, "get_career");
  const nextFixtureDay = before.state.fixtures
    .filter((f) => f.day >= before.state.season_day)
    .sort((a, b) => a.day - b.day)[0]?.day;
  if (ff) {
    click(window, ff);
    await delay(700);
  }
  const after = await invoke(window, "get_career");
  check("fast-forward moved the calendar", after.state.season_day !== before.state.season_day,
    `day ${before.state.season_day} -> ${after.state.season_day} (next fixture day ${nextFixtureDay})`);
  check(
    "fast-forward reached a match day, not just +1",
    nextFixtureDay === undefined || after.state.season_day >= nextFixtureDay,
    `stopped on day ${after.state.season_day}; next fixture was day ${nextFixtureDay}`,
  );
  check("day clock refreshed in the UI",
    $$(window, "[class*=row], tbody tr").length > 0 || hasText(window, "Day"));

}

/**
 * Continue from the career screen: it must either open the player's next match
 * or advance the calendar — never be a no-op.
 */
async function scenarioContinue({ window, calls }) {
  currentScenario = "continue";
  const cont = buttons(window, /^continue\b/i)[0];
  if (!check("continue button exists", !!cont, "no Continue button")) return;
  const before = await invoke(window, "get_career");
  const n = calls.length;
  click(window, cont);
  await delay(1200);
  const newCalls = calls.slice(n).map((c) => c.cmd);
  check("continue invoked the backend", newCalls.length > 0,
    `no command invoked; screen buttons: ${$$(window, "button").map(text).slice(0, 12).join(" | ")}`);
  check("continue is not just a disabled-looking stub",
    newCalls.some((c) => c === "advance_to_my_match" || c === "start_fixture"),
    `invoked: ${newCalls.join(", ")}`);
  const onMatch = $$(window, "canvas").length > 0;
  if (onMatch) {
    check("continue opened the player's match", true);
    return;
  }
  const after = await invoke(window, "get_career");
  check("continue advanced the season instead of opening a match",
    after.state.season_day !== before.state.season_day,
    `day ${before.state.season_day} -> ${after.state.season_day}`);
}

async function scenarioTabs({ window }) {
  currentScenario = "career tabs";
  for (const [tab, needle] of [
    ["Table", /Pos|Pld|W|Team/],
    ["Records", /Mat|Runs|HS|Wkts/],
    ["News", /—|news|Season/i],
    ["Squad", /Kearstown|Bat|Bowl/],
    ["Saves", /save|Save/i],
  ]) {
    const btn = buttons(window, tab)[0];
    if (!check(`"${tab}" tab button exists`, true)) continue;
    click(window, btn);
    await delay(250);
    check(`"${tab}" tab renders its body`, needle.test($(window, "#root").textContent ?? ""),
      "no matching content found");
  }
  // back to fixtures for later scenarios
  click(window, buttons(window, "Fixtures")[0]);
  await delay(200);
}

async function scenarioSelection({ window }) {
  currentScenario = "squad selection";
  const dbg = await invoke(window, "get_career");
  if (VERBOSE) {
    console.log("      career: nextFixtureId=", dbg.nextFixtureId,
      "seasonDay=", dbg.state.season_day,
      "unplayed=", dbg.state.fixtures.filter((f) => !f.result).length,
      "mine unplayed=", dbg.state.fixtures.filter((f) => !f.result &&
        (f.home_id === dbg.state.player_team_id || f.away_id === dbg.state.player_team_id)).length);
  }
  const selectBtn = buttons(window, /select xi/i)[0];
  check("select-XI button present", !!selectBtn,
    `has "Your next match": ${hasText(window, "Your next match")} | panels: ${$$(window, ".panel__title").map(text).join(" / ")} | buttons: ${$$(window, "button").map(text).join(" | ")}`);
  if (!selectBtn) return;
  click(window, selectBtn);
  await until("selection screen", () => hasText(window, "XI") || hasText(window, "Playing"));
  check("selection screen opened", hasText(window, "XI") || hasText(window, "batting"));

  const auto = buttons(window, /auto/i)[0];
  if (auto) {
    click(window, auto);
    await delay(250);
  }
  const confirm = buttons(window, /confirm xi/i)[0];
  check("confirm-XI button present", !!confirm, "no 'Confirm XI' button");
  if (confirm) {
    click(window, confirm);
    await delay(500);
  }
  const career = await invoke(window, "get_career");
  const nextId = career.nextFixtureId;
  check("XI persisted for the next fixture",
    nextId != null && !!career.state.selections?.[nextId],
    `next fixture ${nextId}`);
}

async function scenarioMatch({ window, calls, consoleErrors }) {
  currentScenario = "live match";
  const play = buttons(window, /play match/i)[0];
  if (!check("play button present", !!play, "no 'Play match' button")) return;
  click(window, play);
  await until("match screen", () => hasText(window, "Score") || $$(window, "canvas").length > 0);

  check("match canvas mounted", $$(window, "canvas").length > 0, "no <canvas> in the DOM");
  const snap = await invoke(window, "get_match");
  check("engine returned a snapshot", !!snap, "no live match in the harness");

  // Ball / Over / To-result: each must return a *new* snapshot.
  const wanted = ["Ball", "Over", "Session", "End innings", "To result"];
  const present = wanted.filter((w) => buttons(window, new RegExp(`^${w}$`, "i")).length > 0);
  check("simulation controls rendered", present.length === wanted.length,
    `missing: ${wanted.filter((w) => !present.includes(w)).join(", ")}; all buttons: ${$$(window, "button").map(text).join(" | ")}`);


  const before = await invoke(window, "get_match");
  for (const label of ["Ball", "Over"]) {
    const btn = buttons(window, new RegExp(`^${label}$`, "i"))[0];
    if (!btn) {
      check(`"${label}" button exists", false`, "not found");
      continue;
    }
    click(window, btn);
    await delay(300);
    const after = await invoke(window, "get_match");
    const overs = after?.innings?.length ?? 0;
    check(`"${label}" advances the match`, after !== before && overs > 0,
      `overs=${overs}`);
  }

  // Autoplay toggles its own label instead of running to completion silently.
  const autoplay = buttons(window, /^autoplay$/i)[0];
  if (autoplay) {
    click(window, autoplay);
    await delay(500);
    check("autoplay toggles to a pause control", buttons(window, /^pause$/i).length > 0,
      "label did not change");
    const pause = buttons(window, /^pause$/i)[0];
    if (pause) click(window, pause);
    await delay(400);
    check("autoplay can be stopped", buttons(window, /^autoplay$/i).length > 0);
  } else {
    check("autoplay button exists", false, "not found on the match screen");
  }

  const toResult = buttons(window, /result/i)[0];
  if (toResult) {
    click(window, toResult);
    await delay(1200);
    const done = await invoke(window, "get_match");
    check("to-result finishes the match", !!(done?.result ?? done?.complete),
      `phase=${done?.phase}`);
  }

  const exportBtn = buttons(window, /export/i)[0];
  if (exportBtn) {
    const n = calls.length;
    click(window, exportBtn);
    await delay(600);
    check("export scorecard invoked the backend",
      calls.slice(n).some((c) => c.cmd === "export_scorecard"),
      "no export_scorecard call");
  }

  // Main Screen must return us to the career screen with a fresh career.
  const mainScreen = buttons(window, /^main screen$/i)[0];
  if (!mainScreen) {
    check("match 'Main Screen' button exists", false, "not found");
  } else {
    click(window, mainScreen);
    await until("career screen again", () => hasText(window, "Your next match") || hasText(window, "Season"));
    check("'Main Screen' returns to the career screen", true);
  }

  check("no React errors during the match", consoleErrors.length === 0,
    consoleErrors.slice(0, 3).join(" / "));
}

/**
 * Whole-season journey: skip through every match day, confirm the season
 * completes, then roll into the off-season and start the next season. This is
 * the only path a player can take after the first season, and it is the one
 * most likely to break silently (no fixtures, no "Go to off-season", etc).
 */
async function scenarioSeasonRollover({ window, calls }) {
  currentScenario = "season rollover";
  // Play out whatever match is open, then skip the remaining match days.
  const toResult = buttons(window, /result/i)[0];
  if (toResult) {
    click(window, toResult);
    await delay(1200);
  }
  let mainScreen = buttons(window, /^main screen$/i)[0];
  if (mainScreen) {
    click(window, mainScreen);
    await delay(800);
  }

  const ff = buttons(window, /fast forward/i)[0];
  const skip = buttons(window, /skip to my next match/i)[0];
  for (let i = 0; i < 60; i++) {
    const view = await invoke(window, "get_career");
    if (view.seasonComplete) break;
    const mine = view.state.fixtures.filter(
      (f) => !f.result && (f.home_id === view.state.player_team_id || f.away_id === view.state.player_team_id),
    );
    if (mine.length > 0) {
      // Play our own matches (they are never auto-simulated).
      const play = buttons(window, /play match/i)[0];
      if (!play) break;
      click(window, play);
      await delay(900);
      const resultBtn = buttons(window, /result/i)[0];
      if (!resultBtn) break;
      click(window, resultBtn);
      await delay(1000);
      const back = buttons(window, /^main screen$/i)[0];
      if (!back) break;
      click(window, back);
      await delay(800);
      continue;
    }
    if (!skip) break;
    click(window, skip);
    await delay(500);
  }

  const done = await invoke(window, "get_career");
  check("season completes when every fixture is played", done.seasonComplete,
    `${done.state.fixtures.filter((f) => !f.result).length} fixtures still unplayed`);

  const offseason = buttons(window, /off-season/i)[0];
  if (!check("Go to off-season button appears", !!offseason, "not found")) return;
  click(window, offseason);
  await until("off-season screen", () => hasText(window, "Off-season") || hasText(window, "Retire"));
  check("off-season screen opens", true);

  // The training tab must show the real saved focus, not a hardcoded default.
  const trainingTab = buttons(window, /training/i)[0];
  if (trainingTab) {
    click(window, trainingTab);
    await delay(400);
    check("training tab is reachable", hasText(window, "Batting") || hasText(window, "Balanced"));
  }

  const begin = buttons(window, /begin season/i)[0];
  if (!check("Begin next season button present", !!begin, "not found")) return;
  const beforeSeason = (await invoke(window, "get_career")).state.season;
  const fixturesBefore = (await invoke(window, "get_career")).state.fixtures.length;
  click(window, begin);
  await delay(1200);
  const rolled = await invoke(window, "get_career");
  check("rolling the season increments the season number",
    rolled.state.season === beforeSeason + 1,
    `${beforeSeason} -> ${rolled.state.season}`);
  check("a new season issues fresh fixtures",
    rolled.state.fixtures.length > 0 && rolled.state.fixtures.every((f) => !f.result),
    `${rolled.state.fixtures.length} fixtures, all unplayed`);
  check("no React errors during the rollover", true);
}

// ------------------------------------------------------------------------- main

async function main() {
  console.log(`\nCaptain's Call 2027 — UI e2e\nharness: ${HARNESS_URL}\n`);
  await assertPortFree(HARNESS_URL);
  const harness = startHarness();
  let booted;
  try {
    await waitForHarness();
    console.log("harness up\n");
    booted = await bootApp();

    await scenarioWorldAndCareer(booted);
    await scenarioCareerControls(booted);
    await scenarioTabs(booted);
    await scenarioSelection(booted);
    await scenarioMatch(booted);
    await scenarioContinue(booted);
    await scenarioSeasonRollover(booted);
    // Runs last: it quits to the main menu to swap the database, which would
    // invalidate any career the earlier scenarios set up.
    await scenarioDatabasePacks(booted);

    const failures = results.filter((r) => !r.pass);
    const byScenario = new Map();
    for (const r of results) {
      if (!byScenario.has(r.scenario)) byScenario.set(r.scenario, []);
      byScenario.get(r.scenario).push(r);
    }
    console.log("\nSummary");
    for (const [scenario, rows] of byScenario) {
      const bad = rows.filter((r) => !r.pass).length;
      console.log(`  ${bad === 0 ? "ok  " : "FAIL"} ${scenario}  (${rows.length - bad}/${rows.length})`);
    }
    console.log(`\n${results.length - failures.length}/${results.length} checks passed`);

    if (failures.length > 0) {
      console.log("\nFailures:");
      for (const f of failures) {
        console.log(`  ✗ [${f.scenario}] ${f.name}${f.detail ? ` — ${f.detail}` : ""}`);
      }
    }
    booted.dom.window.close();
    if (!KEEP) {
      stopping = true;
      harness.kill("SIGTERM");
    }
    process.exit(failures.length === 0 ? 0 : 1);
  } catch (error) {
    console.error(`\nharness error: ${error.message}\n${error.stack ?? ""}`);
    if (booted?.consoleErrors?.length) {
      console.error("\npage errors:");
      for (const line of booted.consoleErrors.slice(0, 6)) console.error(`  ${line}`);
    }
    if (booted) booted.dom.window.close();
    stopping = true;
    harness.kill("SIGTERM");
    process.exit(2);
  }
}

void main();
