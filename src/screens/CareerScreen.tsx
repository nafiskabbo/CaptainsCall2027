//! Career home: fixtures, table, news, save/load.

import { useEffect, useState } from "react";
import { api } from "../tools/api";
import type {
  CareerView,
  Fixture,
  Format,
  MatchSnapshot,
  SaveSummary,
  World,
} from "../types";

interface Props {
  world: World;
  career: CareerView;
  onCareer: (view: CareerView) => void;
  onPlay: (snapshot: MatchSnapshot) => void;
  onExit: () => void;
}

const FORMAT_LABEL: Record<Format, string> = {
  test: "Test",
  odi50: "50-over",
  t20: "T20",
};

function teamName(world: World, id: number): string {
  return world.teams.find((t) => t.id === id)?.name ?? `Team ${id}`;
}

function groundName(world: World, id: number): string {
  return world.grounds.find((g) => g.id === id)?.name ?? `Ground ${id}`;
}

function formatLabel(format: Format): string {
  return FORMAT_LABEL[format] ?? format;
}

export default function CareerScreen({
  world,
  career,
  onCareer,
  onPlay,
  onExit,
}: Props) {
  const [tab, setTab] = useState<"fixtures" | "table" | "news" | "saves">("fixtures");
  const [saves, setSaves] = useState<SaveSummary[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  const { state, table, next_fixture_id, season_complete } = career;
  const myTeamId = state.player_team_id;
  const nextFixture = state.fixtures.find((f) => f.id === next_fixture_id) ?? null;
  const comp = world.competitions.find((c) => c.id === state.competition_id);

  useEffect(() => {
    api.listSaves().then(setSaves).catch(() => undefined);
  }, [career.state.id]);

  const play = async (fixture: Fixture) => {
    setBusy(true);
    setStatus(null);
    try {
      const snapshot = await api.startFixture(fixture.id);
      onPlay(snapshot);
    } catch (e) {
      setStatus(String(e));
    } finally {
      setBusy(false);
    }
  };

  const advanceDay = async () => {
    setBusy(true);
    try {
      onCareer(await api.advanceDay());
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    setBusy(true);
    try {
      const id = await api.saveCareer();
      setStatus(`Saved (slot ${id})`);
      setSaves(await api.listSaves());
      onCareer(await api.getCareer());
    } finally {
      setBusy(false);
    }
  };

  const load = async (id: number) => {
    setBusy(true);
    try {
      onCareer(await api.loadCareer(id));
      setTab("fixtures");
      setStatus(`Loaded save ${id}`);
    } finally {
      setBusy(false);
    }
  };

  const removeSave = async (id: number) => {
    await api.deleteSave(id);
    setSaves(await api.listSaves());
  };

  return (
    <div className="career">
      <section className="panel careerhead">
        <div>
          <h2>{state.player_team_name}</h2>
          <p className="muted">
            {comp?.name ?? "Competition"} · Season day {state.season_day} · db{" "}
            {state.db_version}
          </p>
        </div>
        <div className="careerhead__actions">
          <button className="btn" onClick={advanceDay} disabled={busy || season_complete}>
            Next day
          </button>
          <button className="btn" onClick={save} disabled={busy}>
            Save
          </button>
          <button className="btn btn--ghost" onClick={onExit}>
            Main menu
          </button>
        </div>
      </section>

      {status && <p className="muted">{status}</p>}

      <section className="panel nextmatch">
        <h3>Next match</h3>
        {nextFixture ? (
          <>
            <p className="nextmatch__title">
              {teamName(world, nextFixture.home_id)} vs {teamName(world, nextFixture.away_id)}
            </p>
            <p className="muted">
              {formatLabel(nextFixture.format)} · {groundName(world, nextFixture.ground_id)} ·
              Day {nextFixture.day}
              {nextFixture.home_id === myTeamId ? " · Home" : " · Away"}
            </p>
            <button
              className="btn btn--primary"
              style={{ width: "100%", marginTop: "0.6rem" }}
              disabled={busy}
              onClick={() => play(nextFixture)}
            >
              Play match
            </button>
            <p className="muted">
              Other fixtures that day are simulated automatically when you advance.
            </p>
          </>
        ) : (
          <p className="muted">
            {season_complete
              ? "Season complete. Table below is final."
              : "No fixture scheduled — advance to the next day."}
          </p>
        )}
      </section>

      <section className="panel">
        <nav className="tabs">
          {(
            [
              ["fixtures", "Fixtures"],
              ["table", "Table"],
              ["news", "News"],
              ["saves", "Saves"],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              className={`tab ${tab === key ? "tab--active" : ""}`}
              onClick={() => setTab(key)}
            >
              {label}
            </button>
          ))}
        </nav>

        {tab === "fixtures" && (
          <table className="data">
            <thead>
              <tr>
                <th className="num">Day</th>
                <th>Home</th>
                <th>Away</th>
                <th className="num">Result</th>
              </tr>
            </thead>
            <tbody>
              {state.fixtures.map((f) => {
                const mine = f.home_id === myTeamId || f.away_id === myTeamId;
                return (
                  <tr key={f.id} className={mine ? "row--mine" : ""}>
                    <td className="num">{f.day}</td>
                    <td>{teamName(world, f.home_id)}</td>
                    <td>{teamName(world, f.away_id)}</td>
                    <td className="num">
                      {f.result ? (
                        <span title={f.result.summary}>
                          {f.result.home_runs} & {f.result.away_runs}
                        </span>
                      ) : (
                        <span className="muted">—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}

        {tab === "table" && (
          <table className="data">
            <thead>
              <tr>
                <th className="num">#</th>
                <th>Team</th>
                <th className="num">P</th>
                <th className="num">W</th>
                <th className="num">L</th>
                <th className="num">T</th>
                <th className="num">Pts</th>
              </tr>
            </thead>
            <tbody>
              {table.map((r, i) => (
                <tr key={r.team_id} className={r.team_id === myTeamId ? "row--mine" : ""}>
                  <td className="num">{i + 1}</td>
                  <td>{teamName(world, r.team_id)}</td>
                  <td className="num">{r.played}</td>
                  <td className="num">{r.won}</td>
                  <td className="num">{r.lost}</td>
                  <td className="num">{r.tied}</td>
                  <td className="num strong">{r.points}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "news" && (
          <ul className="commentary">
            {state.news.map((n, i) => (
              <li key={`${n.day}-${i}`}>
                <span className="commentary__ball">d{n.day}</span>
                {n.headline}
              </li>
            ))}
            {state.news.length === 0 && <li className="muted">No news yet.</li>}
          </ul>
        )}

        {tab === "saves" && (
          <table className="data">
            <thead>
              <tr>
                <th>Name</th>
                <th>Team</th>
                <th className="num">Day</th>
                <th>db</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {saves.map((s) => (
                <tr key={s.id}>
                  <td>{s.name}</td>
                  <td>{s.team_name}</td>
                  <td className="num">{s.season_day}</td>
                  <td className="muted">{s.db_version}</td>
                  <td className="num">
                    <button className="btn" onClick={() => load(s.id)} disabled={busy}>
                      Load
                    </button>{" "}
                    <button className="btn btn--ghost" onClick={() => removeSave(s.id)}>
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel">
        <h3>Squad</h3>
        <SquadTable world={world} teamId={myTeamId} />
      </section>
    </div>
  );
}

function SquadTable({ world, teamId }: { world: World; teamId: number }) {
  const [players, setPlayers] = useState(world.players.filter((p) => p.team_id === teamId));
  useEffect(() => {
    api
      .getSquad(teamId)
      .then(setPlayers)
      .catch(() => undefined);
  }, [teamId]);

  if (players.length === 0) return <p className="muted">No squad data.</p>;
  return (
    <div className="tablewrap">
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
          {players.map((p) => (
            <tr key={p.id}>
              <td>{p.name}</td>
              <td className="num">{p.age}</td>
              <td className="role">{p.role.replace(/([A-Z])/g, " $1").trim()}</td>
              <td className="num">{p.skills.bat}</td>
              <td className="num">{Math.max(p.skills.pace, p.skills.spin)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
