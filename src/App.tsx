import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import "./App.css";

const MILESTONES = [
  "M0 — Foundations (scaffold, schemas, RNG, saves)",
  "M1 — Match engine core",
  "M2 — Competitions & scheduling",
  "M3 — Squad, growth & management",
  "M4 — 2D match view + UX",
  "M5 — 2027 fictional DB + editors",
  "M6 — Update & release infra",
  "M7 — Phase 1 acceptance",
] as const;

export default function App() {
  const [version, setVersion] = useState<string>("dev (browser)");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion("dev (browser)"));
  }, []);

  return (
    <main className="shell">
      <header>
        <h1>Captain&apos;s Call 2027</h1>
        <p className="sub">
          2D cricket management sim · v{version} · tribute to Cricket Coach 2014
        </p>
      </header>
      <section>
        <h2>Build plan</h2>
        <ol>
          {MILESTONES.map((m) => (
            <li key={m}>{m}</li>
          ))}
        </ol>
        <p className="hint">
          Track progress in <code>tasks.plan.md</code>. Spec in{" "}
          <code>specs/requirement.md</code>.
        </p>
      </section>
    </main>
  );
}
