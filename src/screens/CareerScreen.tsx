//! Career home — today's fixtures, table, news, saves.
//! Layout modelled on Cricket Coach 2014's "Today's Fixtures" screen.

import { useEffect, useState } from "react";
import { api } from "../tools/api";
import type { CareerView, MatchSnapshot, SaveSummary, World } from "../types";
import SelectionScreen from "./SelectionScreen";

interface Props {
  world: World;
  career: CareerView;
  onCareer: (view: CareerView) => void;
  onPlay: (snapshot: MatchSnapshot) => void;
}

function teamName(world: World, id: number): string {
  return world.teams.find((t) => t.id === id)?.name ?? `Team ${id}`;
}

function groundName(world: World, id: number): string {
  return world.grounds.find((g) => g.id === id)?.name ?? `Ground ${id}`;
}

export default function CareerScreen({
  world,
  career,
  onCareer,
  onPlay,
}: Props) {
  const [tab, setTab] = useState<"today" | "table" | "news" | "saves" | "squad">("today");
  const [filter, setFilter] = useState<"all" | "mine">("all");
  const [saves, setSaves] = useState<SaveSummary[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [selectingId, setSelectingId] = useState<number | null>(null);

  const { state, table, next_fixture_id, season_complete } = career;
  const myTeamId = state.player_team_id;
  const comp = world.competitions.find((c) => c.id === state.competition_id);

  useEffect(() => {
    api.listSaves().then(setSaves).catch(() => undefined);
  }, [state.id]);

  const play = async (fixtureId: number) => {
    setBusy(true);
    setStatus(null);
    try {
      onPlay(await api.startFixture(fixtureId));
    } catch (e) {
      setStatus(String(e));
    } finally {
      setBusy(false);
    }
  };

  const load = async (id: number) => {
    setBusy(true);
    try {
      onCareer(await api.loadCareer(id));
      setStatus(`Game ${id} loaded`);
      setTab("today");
    } finally {
      setBusy(false);
    }
  };

  const removeSave = async (id: number) => {
    await api.deleteSave(id);
    setSaves(await api.listSaves());
  };

  // Group fixtures by day, like the original's fixture list.
  const days = [...new Set(state.fixtures.map((f) => f.day))].sort((a, b) => a - b);
  const today = days.find((d) => d >= state.season_day) ?? days[days.length - 1];

  if (selectingId !== null) {
    const target = state.fixtures.find((f) => f.id === selectingId);
    if (target) {
      return (
        <SelectionScreen
          world={world}
          fixture={target}
          teamId={myTeamId}
          onCancel={() => setSelectingId(null)}
          onSaved={(message) => {
            setSelectingId(null);
            setStatus(message);
            api.getCareer().then(onCareer);
          }}
        />
      );
    }
  }

  return (
    <div className="stack">
      {/* next match */}
      {next_fixture_id !== null && (
        <section className="panel">
          <h2 className="panel__title">
            Your next match
            {state.selections[next_fixture_id] ? (
              <span className="pill pill--ok" style={{ marginLeft: "0.5rem" }}>
                XI set
              </span>
            ) : (
              <span className="pill pill--warn" style={{ marginLeft: "0.5rem" }}>
                no XI set
              </span>
            )}
          </h2>
          <NextMatchCard
            world={world}
            career={career}
            onSelect={() => setSelectingId(next_fixture_id)}
            onPlay={() => play(next_fixture_id)}
            busy={busy}
          />
        </section>
      )}

      {season_complete && (
        <section className="panel">
          <h2 className="panel__title">Season complete</h2>
          <p>
            {table[0]?.team_id === myTeamId
              ? `Champions — ${teamName(world, table[0].team_id)} win ${comp?.name ?? "the competition"}.`
              : `${teamName(world, table[0].team_id)} win ${comp?.name ?? "the competition"}. You finished ${
                  table.findIndex((r) => r.team_id === myTeamId) + 1
                }.`}
          </p>
        </section>
      )}

      {status && <p className="muted">{status}</p>}

      <section className="panel panel--flush">
        <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", padding: "0.5rem 0.6rem 0" }}>
          {(
            [
              ["today", "Fixtures"],
              ["table", "Table"],
              ["news", "News"],
              ["squad", "Squad"],
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
          <span className="sidebar__spacer" />
          <label htmlFor="filter" className="muted" style={{ margin: 0 }}>
            Filter
          </label>
          <select
            id="filter"
            value={filter}
            onChange={(e) => setFilter(e.target.value as "all" | "mine")}
            style={{ width: 150 }}
          >
            <option value="all">All leagues</option>
            <option value="mine">My team</option>
          </select>
        </div>

        {tab === "today" && (
          <div style={{ padding: "0.5rem 0.2rem 0.7rem" }}>
            {days
              .filter((d) => (filter === "mine" ? dayHasMyTeam(state.fixtures, d, myTeamId) : true))
              .map((day) => (
                <div key={day}>
                  <div className="fixtureday">
                    {day === today ? "Today" : day < state.season_day ? "Played" : "Upcoming"} · day{" "}
                    {day}
                  </div>
                  {state.fixtures
                    .filter((f) => f.day === day)
                    .filter((f) => (filter === "mine" ? f.home_id === myTeamId || f.away_id === myTeamId : true))
                    .map((f) => {
                      const mine = f.home_id === myTeamId || f.away_id === myTeamId;
                      const playable = mine && f.result === null;
                      return (
                        <button
                          key={f.id}
                          type="button"
                          className={`fixturerow ${mine ? "fixturerow--mine" : ""}`}
                          onClick={() => (playable ? play(f.id) : undefined)}
                          disabled={!playable}
                          title={playable ? "Play this match" : undefined}
                        >
                          <span className="fixturerow__team">{teamName(world, f.home_id)}</span>
                          <span className="fixturerow__vs">vs</span>
                          <span className="fixturerow__team fixturerow__team--away">
                            {teamName(world, f.away_id)}
                          </span>
                          <span className="fixturerow__status">
                            {f.result ? (
                              <span title={f.result.summary}>
                                {f.result.home_runs}/{f.result.away_runs}
                              </span>
                            ) : playable ? (
                              "▶"
                            ) : (
                              <span className="muted">—</span>
                            )}
                          </span>
                          <span className="fixturerow__venue">{groundName(world, f.ground_id)}</span>
                        </button>
                      );
                    })}
                </div>
              ))}
          </div>
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
                <th className="num">NRR</th>
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
                  <td className="num">{(r.net_run_rate_x100 / 100).toFixed(2)}</td>
                  <td className="num strong">{r.points}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "news" && (
          <ul style={{ listStyle: "none", margin: 0, padding: "0.4rem 0.6rem" }}>
            {[...state.news].reverse().map((n, i) => (
              <li
                key={i}
                style={{
                  padding: "0.3rem 0",
                  borderBottom: "1px solid rgba(255,255,255,0.07)",
                  fontSize: "0.85rem",
                }}
              >
                <span className="pill" style={{ marginRight: "0.5rem" }}>
                  d{n.day}
                </span>
                {n.headline}
              </li>
            ))}
            {state.news.length === 0 && <li className="muted">No news yet.</li>}
          </ul>
        )}

        {tab === "squad" && (
          <table className="data">
            <thead>
              <tr>
                <th>Player</th>
                <th className="num">Age</th>
                <th>Role</th>
                <th className="num">Bat</th>
                <th className="num">Bowl</th>
                <th className="num">Fld</th>
                <th className="num">Pot</th>
              </tr>
            </thead>
            <tbody>
              <SquadRows world={world} teamId={myTeamId} />
            </tbody>
          </table>
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
                    <button className="btn btn--sm" onClick={() => load(s.id)} disabled={busy}>
                      Load
                    </button>{" "}
                    <button className="btn btn--sm btn--ghost" onClick={() => removeSave(s.id)}>
                      Delete
                    </button>
                  </td>
                </tr>
              ))}
              {saves.length === 0 && (
                <tr>
                  <td colSpan={5} className="muted">
                    No saved games yet — use Save in the bottom bar.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

function NextMatchCard({
  world,
  career,
  onSelect,
  onPlay,
  busy,
}: {
  world: World;
  career: CareerView;
  onSelect: () => void;
  onPlay: () => void;
  busy: boolean;
}) {
  const fixture = career.state.fixtures.find((f) => f.id === career.next_fixture_id);
  if (!fixture) return <p className="muted">No fixture left to play.</p>;

  const opponent = fixture.home_id === career.state.player_team_id ? fixture.away_id : fixture.home_id;

  return (
    <div style={{ display: "flex", alignItems: "center", gap: "1rem", flexWrap: "wrap" }}>
      <div style={{ flex: 1, minWidth: 220 }}>
        <div style={{ fontSize: "1.05rem", fontWeight: 700 }}>
          {teamName(world, fixture.home_id)} <span className="muted">vs</span>{" "}
          {teamName(world, fixture.away_id)}
        </div>
        <div className="muted">
          {fixture.format.toUpperCase()} · {groundName(world, fixture.ground_id)} · day{" "}
          {fixture.day} ·{" "}
          {fixture.home_id === career.state.player_team_id ? "home" : "away"} vs{" "}
          {teamName(world, opponent)}
        </div>
      </div>
      <button className="btn" onClick={onSelect} disabled={busy}>
        Select XI
      </button>
      <button className="btn btn--primary" onClick={onPlay} disabled={busy}>
        Play match
      </button>
    </div>
  );
}

function SquadRows({ world, teamId }: { world: World; teamId: number }) {
  const [players, setPlayers] = useState(
    world.players.filter((p) => p.team_id === teamId),
  );

  useEffect(() => {
    api
      .getSquad(teamId)
      .then(setPlayers)
      .catch(() => undefined);
  }, [teamId]);

  if (players.length === 0) {
    return (
      <tr>
        <td colSpan={7} className="muted">
          No squad data.
        </td>
      </tr>
    );
  }

  return (
    <>
      {players.map((p) => (
        <tr key={p.id}>
          <td>{p.name}</td>
          <td className="num">{p.age}</td>
          <td className="muted">{p.role.replace(/_/g, " ")}</td>
          <td className="num">{p.skills.bat}</td>
          <td className="num">{Math.max(p.skills.pace, p.skills.spin)}</td>
          <td className="num">{p.skills.fielding}</td>
          <td className="num muted">{p.potential.bat}</td>
        </tr>
      ))}
    </>
  );
}

function dayHasMyTeam(fixtures: CareerView["state"]["fixtures"], day: number, teamId: number) {
  return fixtures.some((f) => f.day === day && (f.home_id === teamId || f.away_id === teamId));
}
