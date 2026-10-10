//! Dump the rendered DOM of a screen for debugging the UI e2e harness.
//! `node tools/ui-dom.mjs [scenario]`
import { spawn } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
import path from "node:path";
import fs from "node:fs";
import esbuild from "esbuild";
import jsdomPkg from "jsdom";

const { JSDOM } = jsdomPkg;
const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const HARNESS_URL = process.env.CC_HARNESS ?? "http://127.0.0.1:8787";

function fakeCanvasContext() {
  const gradient = { addColorStop() {} };
  return new Proxy(
    {},
    {
      get(_t, p) {
        if (p === "canvas") return { width: 800, height: 500 };
        if (p === "createRadialGradient" || p === "createLinearGradient") return () => gradient;
        if (p === "measureText") return () => ({ width: 10 });
        return () => undefined;
      },
      set() {
        return true;
      },
    },
  );
}

const proc = spawn("cargo", ["run", "--example", "e2e", "--", "127.0.0.1:8787", "target/e2e"], {
  cwd: path.join(ROOT, "src-tauri"),
  stdio: "ignore",
});

const deadline = Date.now() + 180_000;
for (;;) {
  try {
    if ((await fetch(`${HARNESS_URL}/health`)).ok) break;
  } catch {}
  if (Date.now() > deadline) throw new Error("harness did not start");
  await delay(500);
}

const out = await esbuild.build({
  entryPoints: [path.join(ROOT, "src/main.tsx")],
  bundle: true,
  write: false,
  format: "iife",
  platform: "browser",
  target: ["es2022"],
  jsx: "automatic",
  define: { "process.env.NODE_ENV": '"development"' },
  plugins: [
    {
      name: "stub-css",
      setup(b) {
        b.onResolve({ filter: /\.css$/ }, (a) => ({ path: a.path, namespace: "stub" }));
        b.onLoad({ filter: /.*/, namespace: "stub" }, () => ({ contents: "export default {}", loader: "js" }));
      },
    },
  ],
});

const dom = new JSDOM('<!doctype html><html><body><div id="root"></div></body></html>', {
  url: "http://localhost:1420/e2e.html",
  pretendToBeVisual: true,
  runScripts: "outside-only",
});
const { window } = dom;
window.HTMLCanvasElement.prototype.getContext = (k) => (k === "2d" ? fakeCanvasContext() : null);
window.__TAURI_INTERNALS__ = {
  async invoke(cmd, args = {}) {
    const res = await fetch(`${HARNESS_URL}/invoke`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ cmd, args }),
    });
    const p = await res.json();
    if (!p.ok) throw new Error(p.error);
    return p.data;
  },
  transformCallback: () => 0,
  unregisterCallback: () => undefined,
  convertFileSrc: (p) => p,
};
window.console.error = (...a) => console.error("[react]", ...a);
window.addEventListener("error", (e) => console.error("[window error]", e.message));
window.addEventListener("unhandledrejection", (e) =>
  console.error("[rejection]", e.reason?.stack ?? e.reason),
);

const mode = process.argv[2] ?? "world";

if (mode === "world") {
  window.eval(out.outputFiles[0].text);
  await delay(2500);
  dump(window, "WORLD SCREEN");
  const begin = [...window.document.querySelectorAll("button")].find((b) =>
    /begin season/i.test(b.textContent || ""),
  );
  if (begin) {
    begin.click();
    await delay(1500);
    dump(window, "AFTER BEGIN SEASON");
  }
} else if (mode === "match") {
  // Seed a career + match straight through the harness, then boot.
  await fetch(`${HARNESS_URL}/invoke`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ cmd: "start_career", args: { name: "X", teamId: 101, seed: 7 } }),
  }).then(() => {});
  await delay(300);
  // drive the whole flow like a player would
  const call = async (cmd, args = {}) => {
    const res = await fetch(`${HARNESS_URL}/invoke`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ cmd, args }),
    });
    const p = await res.json();
    if (!p.ok) throw new Error(p.error);
    return p.data;
  };
  window.eval(out.outputFiles[0].text);
  await delay(2200);
  dump(window, "WORLD");
  const club = window.document.querySelector('select[aria-label="Club"]');
  if (club) {
    const setter = Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype, "value").set;
    setter.call(club, "101");
    club.dispatchEvent(new window.Event("change", { bubbles: true }));
  }
  await delay(300);
  const begin = [...window.document.querySelectorAll("button")].find((b) =>
    /begin season/i.test(b.textContent || ""),
  );
  if (begin) begin.click();
  await delay(1500);
  dump(window, "CAREER");
  const cont = [...window.document.querySelectorAll("button")].find((b) =>
    /^continue/i.test((b.textContent || "").trim()),
  );
  console.log("clicking CONTINUE...");
  if (cont) cont.click();
  await delay(2500);
  dump(window, "MATCH");
  const snap = await call("get_match");
  console.log("match phase:", snap && snap.phase, "innings:", snap && snap.innings.length);
}

proc.kill("SIGTERM");
process.exit(0);

function dump(window, title) {
  const root = window.document.querySelector("#root");
  console.log(`\n===== ${title} =====`);
  console.log(root.innerHTML.replace(/></g, ">\n<").slice(0, 12000));
  const btns = [...window.document.querySelectorAll("button")].map(
    (b) => `${(b.textContent || "").replace(/\s+/g, " ").trim()}${b.disabled ? " [disabled]" : ""}`,
  );
  console.log("\nbuttons:", JSON.stringify(btns, null, 1));
}
