//! Team selection screen: pick the XI and batting order for the next fixture.

import { useEffect, useMemo, useState } from "react";
import { api } from "../tools/api";
import type { Fixture, Player, World } from "../types";
import { Button } from "../ui";

interface Props {
  world: World;
  fixture: Fixture;
  teamId: number;
  onCancel: () => void;
  onSaved: (message: string) => void;
}

const BATTING_SKILL: (p: Player) => number = (p) => p.skills.bat;
const BOWLING_SKILL: (p: Player) => number = (p) =>
  Math.max(p.skills.pace, p.skills.spin);

function roleLabel(role: string): string {
  // Data is snake_case ("pace_bowler"); show "Pace bowler".
  const spaced = role.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

export default function SelectionScreen({
  world,
  fixture,
  teamId,
  onCancel,
  onSaved,
}: Props) {
  const [squad, setSquad] = useState<Player[]>([]);
  const [xi, setXi] = useState<number[]>([]);
  const [keeper, setKeeper] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const team = world.teams.find((t) => t.id === teamId);

  useEffect(() => {
    api
      .getSquad(teamId)
      .then((players) => {
        setSquad(players);
        return api.getSelection(fixture.id);
      })
      .then((selection) => {
        setXi(selection.xi_ids);
        setKeeper(selection.keeper_id);
      })
      .catch((e) => setError(String(e)));
  }, [teamId, fixture.id]);

  const byId = useMemo(() => new Map(squad.map((p) => [p.id, p])), [squad]);
  const available = squad.filter((p) => !xi.includes(p.id));
  const selected = xi.map((id) => byId.get(id)).filter((p): p is Player => !!p);

  // Composition hints, the way Cricket Coach nudges you toward a balanced XI.
  const bowlers = selected.filter((p) => BOWLING_SKILL(p) >= 55).length;
  const keepers = selected.filter((p) => p.role === "wicketkeeper").length;
  const avgBat =
    selected.length > 0
      ? Math.round(selected.reduce((sum, p) => sum + BATTING_SKILL(p), 0) / selected.length)
      : 0;
  const avgBowl =
    selected.length > 0
      ? Math.round(selected.reduce((sum, p) => sum + BOWLING_SKILL(p), 0) / selected.length)
      : 0;

  const add = (id: number) => {
    if (xi.length >= 11) return;
    setXi((prev) => [...prev, id]);
  };

  const remove = (index: number) => {
    const player = byId.get(xi[index]);
    setXi((prev) => prev.filter((_, i) => i !== index));
    if (player && keeper === player.id) setKeeper(null);
  };

  const move = (index: number, delta: number) => {
    const target = index + delta;
    if (target < 0 || target >= xi.length) return;
    setXi((prev) => {
      const next = [...prev];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  };

  const autoPick = () => {
    api
      .autoSelect(teamId, fixture.format)
      .then((selection) => {
        setXi(selection.xi_ids);
        setKeeper(selection.keeper_id);
      })
      .catch((e) => setError(String(e)));
  };

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.setSelection(fixture.id, {
        team_id: teamId,
        xi_ids: xi,
        batting_order: xi,
        keeper_id: keeper,
      });
      onSaved("XI saved for this match.");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const opponent =
    fixture.home_id === teamId ? fixture.away_id : fixture.home_id;
  const opponentName =
    world.teams.find((t) => t.id === opponent)?.name ?? "Opponents";

  return (
    <div className="selection">
      <section className="panel">
        <div className="selection__head">
          <div>
            <h2>Pick your XI</h2>
            <p className="muted">
              vs {opponentName} · {fixture.format.toUpperCase()} ·{" "}
              {world.grounds.find((g) => g.id === fixture.ground_id)?.name}
            </p>
          </div>
          <div className="selection__headactions">
            <Button onClick={autoPick} disabled={busy}>Auto-pick</Button>
            <Button variant="ghost" onClick={onCancel}>Close</Button>
          </div>
        </div>

        <div className="selection__stats">
          <span className="pill">
            {xi.length}/11 picked
          </span>
          <span className={`pill ${bowlers >= 4 ? "pill--ok" : "pill--warn"}`}>
            {bowlers} bowling options
          </span>
          <span className={`pill ${keepers >= 1 ? "pill--ok" : "pill--warn"}`}>
            {keepers} wicketkeeper{keepers === 1 ? "" : "s"}
          </span>
          <span className="pill">bat {avgBat}</span>
          <span className="pill">bowl {avgBowl}</span>
        </div>

        <table className="data">
          <thead>
            <tr>
              <th className="num">Bat</th>
              <th>Player</th>
              <th>Role</th>
              <th className="num">Bat</th>
              <th className="num">Bowl</th>
              <th>Keeper</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {selected.map((p, i) => (
              <tr key={p.id}>
                <td className="num strong">{i + 1}</td>
                <td>
                  <button
                    className="btn btn--ghost btn--icon"
                    aria-label={`Move ${p.name} up`}
                    onClick={() => move(i, -1)}
                    disabled={i === 0}
                  >
                    ↑
                  </button>{" "}
                  <button
                    className="btn btn--ghost btn--icon"
                    aria-label={`Move ${p.name} down`}
                    onClick={() => move(i, 1)}
                    disabled={i === selected.length - 1}
                  >
                    ↓
                  </button>{" "}
                  {p.name}
                </td>
                <td className="role">{roleLabel(p.role)}</td>
                <td className="num">{BATTING_SKILL(p)}</td>
                <td className="num">{BOWLING_SKILL(p)}</td>
                <td className="num">
                  <input
                    type="radio"
                    name="keeper"
                    checked={keeper === p.id}
                    onChange={() => setKeeper(p.id)}
                    aria-label={`${p.name} keeps wicket`}
                  />
                </td>
                <td className="num">
                  <button className="btn btn--ghost" onClick={() => remove(i)}>
                    Drop
                  </button>
                </td>
              </tr>
            ))}
            {selected.length === 0 && (
              <tr>
                <td colSpan={7} className="muted">
                  No players picked yet — add 11 from the squad list.
                </td>
              </tr>
            )}
          </tbody>
        </table>

        <div className="selection__foot">
          <span className="muted">
            You need exactly 11 players, all from {team?.name ?? "your squad"}.
          </span>
          <Button style={{ width: "auto", padding: "0.5rem 1.4rem" }} onClick={save} disabled={busy || xi.length !== 11}>{busy ? "Saving…" : "Confirm XI"}</Button>
        </div>
        {error && <p className="error">{error}</p>}
      </section>

      <section className="panel">
        <h3>Squad</h3>
        <div className="tablewrap">
          <table className="data">
            <thead>
              <tr>
                <th>Player</th>
                <th className="num">Age</th>
                <th>Role</th>
                <th className="num">Bat</th>
                <th className="num">Bowl</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {available.map((p) => (
                <tr key={p.id}>
                  <td>{p.name}</td>
                  <td className="num">{p.age}</td>
                  <td className="role">{roleLabel(p.role)}</td>
                  <td className="num">{BATTING_SKILL(p)}</td>
                  <td className="num">{BOWLING_SKILL(p)}</td>
                  <td className="num">
                    {p.injury_days > 0 ? (
                      <span className="pill pill--warn">inj {p.injury_days}d</span>
                    ) : (
                      <button
                        className="btn"
                        onClick={() => add(p.id)}
                        disabled={xi.length >= 11}
                      >
                        Add
                      </button>
                    )}
                  </td>
                </tr>
              ))}
              {available.length === 0 && (
                <tr>
                  <td colSpan={6} className="muted">
                    Whole squad selected.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
