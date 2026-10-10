// Render the match screen to static HTML with the real stylesheet, so the
// layout can be looked at rather than assumed. The UI e2e harness asserts
// structure; this exists to catch the things assertions do not — spacing,
// alignment, and whether a control is legible in context.
//
//   bun tools/shot-match.mjs [out.html] [--shot six]

import { build } from "esbuild";
import { readFileSync, writeFileSync } from "node:fs";

const outPath = process.argv[2]?.startsWith("--") ? "/tmp/shot-match.html" : process.argv[2] ?? "/tmp/shot-match.html";
const shotIdx = process.argv.indexOf("--shot");
const wanted = shotIdx > -1 ? process.argv[shotIdx + 1] : null;

const HARNESS_URL = process.env.CC_HARNESS ?? "http://127.0.0.1:8787";

async function call(cmd, args = {}) {
  const res = await fetch(`${HARNESS_URL}/invoke`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ cmd, args }),
  });
  const p = await res.json();
  if (!p.ok) throw new Error(`${cmd}: ${p.error}`);
  return p.data;
}

// The real stylesheets, concatenated in import order.
const css = ["src/index.css", "src/design.css", "src/design-components.css", "src/design-match.css"]
  .map((f) => {
    try {
      return readFileSync(f, "utf8");
    } catch {
      return "";
    }
  })
  .join("\n");

// Build the app with CSS inlined as a string the render step can inject.
const out = await build({
  entryPoints: ["src/main.tsx"],
  bundle: true,
  write: false,
  format: "iife",
  jsx: "automatic",
  loader: { ".css": "text", ".svg": "text" },
  define: { "process.env.NODE_ENV": '"production"' },
});

await call("start_career", { name: "Shot", teamId: 101, seed: 7 });
await new Promise((r) => setTimeout(r, 300));
const career = await call("get_career");
const next = career.state.fixtures.find((f) => f.id === career.nextFixtureId);
await call("start_fixture", { fixtureId: next.id });
await call("sim_match", { count: 37 });

if (wanted) {
  await call("set_match_input", {
    input: {
      batting_aggression: 62,
      keep_strike_toggle: false,
      hit_and_run_toggle: false,
      target_bowler_id: 0,
      field_preset: "balanced",
      shot_choice: wanted,
    },
  });
}

const { JSDOM } = await import("jsdom");
const dom = new JSDOM('<!doctype html><html><head></head><body><div id="root"></div></body></html>', {
  url: "http://localhost:1420/",
  pretendToBeVisual: true,
  runScripts: "outside-only",
});
const { window } = dom;

const seeded = {};
for (const [cmd, args] of [
  ["get_career", {}],
  ["get_match", {}],
  ["get_finance_summary", {}],
  ["preview_ball", {}],
]) {
  try {
    seeded[cmd] = await call(cmd, args);
  } catch {
    /* optional */
  }
}

window.__TAURI_INTERNALS__ = {
  async invoke(cmd, args = {}) {
    if (cmd in seeded && Object.keys(args).length === 0) return seeded[cmd];
    return call(cmd, args);
  },
  transformCallback: () => 0,
  unregisterCallback: () => undefined,
  convertFileSrc: (p) => p,
};
window.HTMLCanvasElement.prototype.getContext = () => ({
  fillRect() {},
  clearRect() {},
  beginPath() {},
  moveTo() {},
  lineTo() {},
  arc() {},
  fill() {},
  stroke() {},
  save() {},
  restore() {},
  translate() {},
  rotate() {},
  scale() {},
  fillText() {},
  measureText: () => ({ width: 10 }),
  createLinearGradient: () => ({ addColorStop() {} }),
  setTransform() {},
  drawImage() {},
  closePath() {},
  strokeText() {},
});

const code = out.outputFiles[0].text;
window.eval(code);
await new Promise((r) => setTimeout(r, 2500));

// Drive the flow to the match screen.
const click = (el) => el?.dispatchEvent(new window.MouseEvent("click", { bubbles: true, cancelable: true }));
const btn = (re) => [...window.document.querySelectorAll("button")].find((b) => re.test((b.textContent || "").trim()));
click(btn(/begin season/i));
await new Promise((r) => setTimeout(r, 1200));
click(btn(/^continue/i));
await new Promise((r) => setTimeout(r, 1200));
click(btn(/play match/i));
await new Promise((r) => setTimeout(r, 1500));

if (wanted) {
  click(window.document.querySelector(`.shotpicker .ball--${wanted}`));
  await new Promise((r) => setTimeout(r, 400));
}

const root = window.document.getElementById("root");
writeFileSync(
  outPath,
  `<!doctype html><html><head><meta charset="utf-8"><style>${css}</style></head><body style="margin:0">${root.outerHTML}</body></html>`,
);
console.log(`wrote ${outPath}`);