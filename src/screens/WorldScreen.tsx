import { useEffect, useMemo, useState } from "react";
import { api } from "../tools/api";
import type { DbPackInfo, Player, Team, World } from "../types";
import UpdatePanel from "./UpdatePanel";
import { Button } from "../ui";

interface Props {
  world: World;
  /** `firstSeason` is 1 for the 2027 season; `resetForm` wipes roster form. */
  onStartCareer: (
    teamId: number,
    name: string,
    seed: number,
    firstSeason: number,
    resetForm: boolean,
  ) => void | Promise<void>;
  /** Called after the active database is swapped, so App can reload the world. */
  onWorldChanged: (world: World) => void;
}

const SEASONS = ["2027", "2028", "2029"];
/** Season year -> career season number (the engine stores 1-based seasons). */
const SEASON_INDEX: Record<string, number> = { "2027": 1, "2028": 2, "2029": 3 };

function teamLabel(world: World, team: Team): string {
  const nation = world.nations.find((n) => n.id === team.nation_id);
  return nation ? `${nation.short_name} · ${team.name}` : team.name;
}

export default function WorldScreen({ world, onStartCareer, onWorldChanged }: Props) {
  const [teamId, setTeamId] = useState<number>(
    () => world.teams.find((t) => !t.is_national)?.id ?? world.teams[0]?.id ?? 0,
  );
  const [careerName, setCareerName] = useState<string>("My Career");
  const [season, setSeason] = useState<string>("2027");
  const [seed, setSeed] = useState<number>(() => 20270101);
  const [resetForm, setResetForm] = useState(false);

  const [squad, setSquad] = useState<Player[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [packs, setPacks] = useState<DbPackInfo[]>([]);
  const [packKey, setPackKey] = useState<string>("");
  const [switching, setSwitching] = useState(false);

  useEffect(() => {
    api
      .listDbPacks()
      .then((list) => {
        setPacks(list);
        // Preselect whatever database is actually loaded.
        setPackKey(list.find((p) => p.dbVersion === world.db_version)?.key ?? "default");
      })
      .catch(() => undefined);
  }, [world.db_version]);

  /** Swap the active database, then reset the selection to a valid team. */
  const changeDatabase = async (key: string) => {
    setSwitching(true);
    setError(null);
    try {
      const next = await api.loadDbPack(key);
      onWorldChanged(next);
      const firstPlayable =
        next.teams.find((t) => !t.is_national)?.id ?? next.teams[0]?.id ?? 0;
      setTeamId(firstPlayable);
      setPackKey(key);
      setSquad([]);
    } catch (e) {
      setError(String(e));
    } finally {
      setSwitching(false);
    }
  };

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

  const team = world.teams.find((t) => t.id === teamId);
  const isNational = team?.is_national ?? false;
  const competitions = world.competitions.filter((c) => c.team_ids.includes(teamId));

  useEffect(() => {
    if (!teamId) return;
    api
      .getSquad(teamId)
      .then(setSquad)
      .catch((e) => setError(String(e)));
  }, [teamId]);

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      await onStartCareer(
        teamId,
        careerName.trim() || "My Career",
        seed,
        SEASON_INDEX[season] ?? 1,
        resetForm,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="stack">
      <section className="panel">
        <h2 className="panel__title">Database</h2>
        <div className="setuprow">
          <span className="setuprow__label">Active database</span>
          <div className="setuprow__control">
            <select
              value={packKey}
              onChange={(e) => void changeDatabase(e.target.value)}
              disabled={switching}
              aria-label="Database"
              title="Choose which world to play"
            >
              {packs.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>
        </div>
        {packs
          .filter((p) => p.key === packKey)
          .map((p) => (
            <div key={p.key}>
              {p.realNames ? (
                <p className="note note--warn">
                  <b>Test database.</b> {p.description} Real player names carry
                  right-of-publicity risk and are not shipped as first-party
                  content.
                </p>
              ) : (
                <p className="muted">{p.description}</p>
              )}
              <p className="muted">
                {p.nations} nations · {p.teams} teams · {p.players} players ·
                version {p.dbVersion}
              </p>
            </div>
          ))}
      </section>

      <section className="panel">
        <h2 className="panel__title">Setup Career</h2>

        <div className="setuprow">
          <span className="setuprow__label">Manager name</span>
          <div className="setuprow__control">
            <input
              type="text"
              value={careerName}
              onChange={(e) => setCareerName(e.target.value)}
              aria-label="Manager name"
            />
          </div>
        </div>

        <div className="setuprow">
          <span className="setuprow__label">Take charge of</span>
          <div className="setuprow__control">
            <select
              value={teamId}
              onChange={(e) => setTeamId(Number(e.target.value))}
              aria-label="Club"
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
        </div>

        <div className="setuprow">
          <span className="setuprow__label">Starting season</span>
          <div className="setuprow__control">
            <select
              value={season}
              onChange={(e) => setSeason(e.target.value)}
              aria-label="Season"
              title="Which season this career opens on"
            >
              {SEASONS.map((s) => (
                <option key={s} value={SEASON_INDEX[s]}>
                  {s} season
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="setuprow">
          <span className="setuprow__label">Database seed</span>
          <div className="setuprow__control">
            <input
              type="number"
              value={seed}
              onChange={(e) => setSeed(Number(e.target.value))}
              aria-label="Database seed"
            />
            <label className="radio" title="Clears form, fatigue and injuries across the roster">
              <input type="checkbox" checked={resetForm} onChange={(e) => setResetForm(e.target.checked)} />
              Reset player form?
            </label>
          </div>
        </div>

        <div style={{ marginTop: "0.8rem", display: "flex", gap: "0.5rem", alignItems: "center" }}>
          <Button onClick={start} disabled={busy || !teamId || (competitions.length === 0 && !isNational)}>{busy ? "Starting…" : "Begin Season"}</Button>
          <span className="muted">
            {competitions.length > 0
              ? `Enters ${competitions.map((c) => c.name).join(", ")}`
              : "This club has no competition — pick another side."}
          </span>
        </div>
        {error && <p className="error">{error}</p>}
      </section>

      <section className="panel panel--flush">
        <div style={{ padding: "0.6rem 0.8rem" }}>
          <UpdatePanel />
        </div>
      </section>

      <section className="panel panel--flush">
        <div style={{ padding: "0.6rem 0.8rem 0" }}>
          <h2 className="panel__title" style={{ marginBottom: "0.4rem" }}>
            {team?.name ?? "Squad"} — {squad.length} players
          </h2>
          <p className="muted">
            Your side plays a full round-robin. Other fixtures simulate automatically; you play
            your own matches ball by ball. All names are fictional.
          </p>
        </div>
        <div className="tablewrap" style={{ maxHeight: 420 }}>
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
                  <td className="muted">{p.role.replace(/_/g, " ")}</td>
                  <td className="num">{p.skills.bat}</td>
                  <td className="num">{Math.max(p.skills.pace, p.skills.spin)}</td>
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
