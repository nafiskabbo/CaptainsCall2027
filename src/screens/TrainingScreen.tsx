//! Training screen: choose the squad's training focus each season.
//!
//! The engine applies a development multiplier from the focus (core/
//! development::TrainingFocus); this screen is where the manager picks it and
//! sees the squad's biggest development prospects.

import { useEffect, useMemo, useState } from "react";
import { api } from "../tools/api";
import type { CareerView, Player, TrainingFocus } from "../types";

interface Props {
  career: CareerView;
  onSaved: (message: string) => void;
}

const FOCUSES: { value: TrainingFocus; label: string; blurb: string }[] = [
  { value: "batting", label: "Batting", blurb: "Faster improvement in all batting skills" },
  { value: "bowling", label: "Bowling", blurb: "Faster improvement in pace and spin" },
  { value: "fielding", label: "Fielding", blurb: "Faster improvement in fielding and keeping" },
  { value: "fitness", label: "Fitness", blurb: "Better recovery between matches" },
  { value: "none", label: "Balanced", blurb: "No specialisation — normal development" },
];

export default function TrainingScreen({ career, onSaved }: Props) {
  const [squad, setSquad] = useState<Player[]>([]);
  const [focus, setFocus] = useState<TrainingFocus>("none");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api
      .getSquad(career.state.player_team_id)
      .then(setSquad)
      .catch(() => undefined);
  }, [career.state.player_team_id]);

  // Development prospects: young players with the most room to grow.
  const prospects = useMemo(
    () =>
      [...squad]
        .map((p) => {
          const best = Math.max(p.skills.bat, p.skills.pace, p.skills.spin);
          const ceiling = Math.max(p.potential.bat, p.potential.pace, p.potential.spin);
          return { player: p, headroom: ceiling - best };
        })
        .filter((p) => p.player.age <= 25 && p.headroom > 2)
        .sort((a, b) => b.headroom - a.headroom)
        .slice(0, 5),
    [squad],
  );

  const veterans = useMemo(
    () =>
      [...squad]
        .filter((p) => p.age >= 31)
        .sort((a, b) => b.age - a.age)
        .slice(0, 4),
    [squad],
  );

  const save = async () => {
    setBusy(true);
    try {
      await api.setTrainingFocus(focus);
      onSaved(`Training focus set to ${focus === "none" ? "Balanced" : focus}.`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="stack">
      <section className="panel">
        <h3 className="panel__title">Training focus</h3>
        <p className="muted">
          Pick what the squad works on between matches. The focus multiplies how
          quickly the matching skills improve over the season.
        </p>
        <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", margin: "0.6rem 0" }}>
          {FOCUSES.map((f) => (
            <button
              key={f.value}
              type="button"
              className={`tab ${focus === f.value ? "tab--active" : ""}`}
              onClick={() => setFocus(f.value)}
              title={f.blurb}
            >
              {f.label}
            </button>
          ))}
        </div>
        <p className="muted">
          {FOCUSES.find((f) => f.value === focus)?.blurb}
        </p>
        <button className="btn btn--primary" onClick={save} disabled={busy} style={{ width: "auto", marginTop: "0.5rem" }}>
          {busy ? "Saving…" : "Set focus"}
        </button>
      </section>

      <section className="panel">
        <h3 className="panel__title">Development prospects</h3>
        <table className="data">
          <thead>
            <tr>
              <th>Player</th>
              <th className="num">Age</th>
              <th>Role</th>
              <th className="num">Bat</th>
              <th className="num">Bowl</th>
              <th className="num">Headroom</th>
            </tr>
          </thead>
          <tbody>
            {prospects.map(({ player, headroom }) => (
              <tr key={player.id}>
                <td>{player.name}</td>
                <td className="num">{player.age}</td>
                <td className="muted">{player.role.replace(/_/g, " ")}</td>
                <td className="num">{player.skills.bat}</td>
                <td className="num">{Math.max(player.skills.pace, player.skills.spin)}</td>
                <td className="num strong" style={{ color: "var(--green-bar)" }}>
                  +{headroom}
                </td>
              </tr>
            ))}
            {prospects.length === 0 && (
              <tr>
                <td colSpan={6} className="muted">
                  No obvious prospects in this squad — everyone is at or near their potential.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      <section className="panel">
        <h3 className="panel__title">Nearing retirement</h3>
        <table className="data">
          <thead>
            <tr>
              <th>Player</th>
              <th className="num">Age</th>
              <th>Role</th>
              <th className="num">Bat</th>
              <th className="num">Bowl</th>
            </tr>
          </thead>
          <tbody>
            {veterans.map((p) => (
              <tr key={p.id}>
                <td>{p.name}</td>
                <td className="num" style={{ color: "var(--gold)" }}>
                  {p.age}
                </td>
                <td className="muted">{p.role.replace(/_/g, " ")}</td>
                <td className="num">{p.skills.bat}</td>
                <td className="num">{Math.max(p.skills.pace, p.skills.spin)}</td>
              </tr>
            ))}
            {veterans.length === 0 && (
              <tr>
                <td colSpan={5} className="muted">
                  A young squad — no one is close to retirement.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>
    </div>
  );
}
