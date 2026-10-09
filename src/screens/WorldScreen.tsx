import { useEffect, useMemo, useState } from "react";
import { api } from "../tools/api";
import type { Player, Team, World } from "../types";

interface Props {
  world: World;
  onStartCareer: (teamId: number, name: string, seed: number) => void | Promise<void>;
}

function teamLabel(world: World, team: Team): string {
  const nation = world.nations.find((n) => n.id === team.nation_id);
  return nation ? `${nation.short_name} · ${team.name}` : team.name;
}

function roleLabel(role: string): string {
  const spaced = role.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

function skillBar(value: number, tone: "bat" | "bowl") {
  const pct = Math.max(4, Math.min(100, value));
  return (
    <span className={`skillbar skillbar--${tone}`}>
      <span className="skillbar__fill" style={{ width: `${pct}%` }} />
      <span className="skillbar__num">{value}</span>
    </span>
  );
}

export default function WorldScreen({ world, onStartCareer }: Props) {
  const [homeId, setHomeId] = useState<number>(
    () => world.teams.find((t) => !t.is_national)?.id ?? world.teams[0]?.id ?? 0,
  );
  const [careerName, setCareerName] = useState<string>("My Career");
  const [seed, setSeed] = useState<number>(() => 20270101);
  const [squad, setSquad] = useState<Player[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const teamsByNation = useMemo(() => {
    const map = new Map<string, Team[]>();
    for (const t of world.teams) {
      const nation = world.nations.find((n) => n.id === t.nation_id);
      const key = nation ? nation.name : "Unknown";
      const list = map.get(key) ?? [];
      list.push(t);
      map.set(key, list);
    }
    return [...map.entries()];
  }, [world]);

  const homeTeam = world.teams.find((t) => t.id === homeId);
  const competitions = world.competitions.filter((c) => c.team_ids.includes(homeId));

  useEffect(() => {
    if (!homeId) return;
    api
      .getSquad(homeId)
      .then(setSquad)
      .catch((e) => setError(String(e)));
  }, [homeId]);

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      await onStartCareer(homeId, careerName.trim() || "My Career", seed);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid grid--setup">
      <section className="panel">
        <h2>New career</h2>
        <div className="field">
          <label htmlFor="career-name">Career name</label>
          <input
            id="career-name"
            type="text"
            value={careerName}
            onChange={(e) => setCareerName(e.target.value)}
          />
        </div>

        <div className="field">
          <label htmlFor="home">Take charge of</label>
          <select
            id="home"
            value={homeId}
            onChange={(e) => setHomeId(Number(e.target.value))}
          >
            {teamsByNation.map(([nation, teams]) => (
              <optgroup key={nation} label={nation}>
                {teams.map((t) => (
                  <option key={t.id} value={t.id}>
                    {teamLabel(world, t)}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </div>

        <div className="field">
          <label htmlFor="seed">Seed (same seed = same season)</label>
          <input
            id="seed"
            type="number"
            value={seed}
            onChange={(e) => setSeed(Number(e.target.value))}
          />
        </div>

        <button
          className="btn btn--primary"
          onClick={start}
          disabled={busy || !homeId || competitions.length === 0}
        >
          {busy ? "Starting…" : "Start career"}
        </button>
        {competitions.length === 0 && (
          <p className="muted">
            This team has no competition in the current database — pick another side.
          </p>
        )}
        {error && <p className="error">{error}</p>}
        <p className="muted">
          Your side plays a full round-robin. Other fixtures are simulated automatically;
          you play your own matches ball by ball.
        </p>
      </section>

      <section className="panel">
        <h2>
          Squad — {homeTeam?.name ?? "—"}{" "}
          <span className="muted">({squad.length} players)</span>
        </h2>
        {homeTeam && competitions.length > 0 && (
          <p className="muted">
            Competitions: {competitions.map((c) => c.name).join(", ")}
          </p>
        )}
        <div className="tablewrap">
          <table className="data">
            <thead>
              <tr>
                <th>Player</th>
                <th className="num">Age</th>
                <th>Role</th>
                <th className="num">Bat</th>
                <th className="num">Bowl</th>
                <th className="num">Fld</th>
              </tr>
            </thead>
            <tbody>
              {squad.map((p) => (
                <tr key={p.id}>
                  <td>{p.name}</td>
                  <td className="num">{p.age}</td>
                  <td className="role">{roleLabel(p.role)}</td>
                  <td className="num">{skillBar(p.skills.bat, "bat")}</td>
                  <td className="num">{skillBar(Math.max(p.skills.pace, p.skills.spin), "bowl")}</td>
                  <td className="num">{p.skills.fielding}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
