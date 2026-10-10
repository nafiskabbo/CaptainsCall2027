//! Career home — today's fixtures, table, news, saves.
//! Layout modelled on Cricket Coach 2014's "Today's Fixtures" screen.

import { useEffect, useState } from "react";
import { api } from "../tools/api";
import type { CareerView, MatchSnapshot, SaveSummary, World } from "../types";
import { Button, Panel, Select, Tabs } from "../ui";
import SelectionScreen from "./SelectionScreen";

interface Props {
  world: World;
  career: CareerView;
  onCareer: (view: CareerView) => void;
  onPlay: (snapshot: MatchSnapshot) => void;
  onOffSeason: () => void;
  /** Controlled from the sidebar so nav rows and tabs stay in sync. */
  tab: string;
  onTab: (tab: string) => void;
  /** Squad filter from the sidebar's quick search. */
  searchQuery: string;
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
  onOffSeason,
  tab,
  onTab,
  searchQuery,
}: Props) {
  const setTab = onTab as (t: string) => void;
  // Tolerate either key ("fixtures" is the sidebar label, "today" the body).
  const activeTab = tab === "fixtures" ? "today" : tab;
  const [filter, setFilter] = useState<"all" | "mine">("all");
  const filterValue: string = filter;
  const [saves, setSaves] = useState<SaveSummary[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [selectingId, setSelectingId] = useState<number | null>(null);

  const { state, table, nextFixtureId, seasonComplete } = career;
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
      {nextFixtureId !== null && (
        <section className="panel">
          <h2 className="panel__title">
            Your next match
            {state.selections[nextFixtureId] ? (
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
            onSelect={() => setSelectingId(nextFixtureId)}
            onPlay={() => play(nextFixtureId)}
            busy={busy}
          />
        </section>
      )}

      {seasonComplete && (
        <section className="panel">
          <h2 className="panel__title">Season complete</h2>
          <p>
            {table[0]?.team_id === myTeamId
              ? `Champions — ${teamName(world, table[0].team_id)} win ${comp?.name ?? "the competition"}.`
              : `${teamName(world, table[0].team_id)} win ${comp?.name ?? "the competition"}. You finished ${
                  table.findIndex((r) => r.team_id === myTeamId) + 1
                }.`}
          </p>
          <Button style={{ marginTop: "0.5rem" }} onClick={onOffSeason}>Go to off-season</Button>
        </section>
      )}

      {status && <p className="muted">{status}</p>}

      <Panel
        flush
        goldTitle={TAB_LABELS[activeTab] ?? "Fixtures"}
        actions={
          <>
            {activeTab === "today" && (
              <Select
                inlineLabel={false}
                label="Filter"
                options={[
                  { value: "all", label: "All leagues" },
                  { value: "mine", label: "My team" },
                ]}
                value={filterValue}
                onChange={(e) => setFilter(e.target.value as "all" | "mine")}
              />
            )}
            <Tabs
              tabs={[
                { id: "today", label: "Fixtures" },
                { id: "table", label: "Table" },
                { id: "records", label: "Records" },
                { id: "news", label: "News" },
                { id: "squad", label: "Squad" },
                { id: "saves", label: "Saves" },
              ]}
              active={activeTab as "today"}
              onChange={(id) => setTab(id)}
            />
          </>
        }
      >
        <div className="panel__body">
        {activeTab === "today" && (
          <div>
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
                      // International programme: show the stage alongside the teams.
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
                          <span className="fixturerow__vs">{f.stage ?? "vs"}</span>
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

        {activeTab === "table" && (
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

        {activeTab === "records" && (
          <table className="data">
            <thead>
              <tr>
                <th>Player</th>
                <th className="num">M</th>
                <th className="num">Runs</th>
                <th className="num">Ave</th>
                <th className="num">SR</th>
                <th className="num">100s</th>
                <th className="num">50s</th>
                <th className="num">Wkts</th>
                <th className="num">Bowl Ave</th>
                <th className="num">Econ</th>
                <th className="num">5W</th>
              </tr>
            </thead>
            <tbody>
              {career.records.map((r) => (
                <tr key={r.name}>
                  <td>{r.name}</td>
                  <td className="num">{r.matches}</td>
                  <td className="num strong">{r.runs}</td>
                  <td className="num">{r.average === null ? "—" : r.average.toFixed(1)}</td>
                  <td className="num">{r.strikeRate.toFixed(0)}</td>
                  <td className="num">{r.hundreds}</td>
                  <td className="num">{r.fifties}</td>
                  <td className="num">{r.wickets}</td>
                  <td className="num">
                    {r.bowlingAverage === null ? "—" : r.bowlingAverage.toFixed(1)}
                  </td>
                  <td className="num">{r.economy.toFixed(2)}</td>
                  <td className="num">{r.fiveWicketHauls}</td>
                </tr>
              ))}
              {career.records.length === 0 && (
                <tr>
                  <td colSpan={11} className="muted">
                    Play a match to start building career records.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}

        {activeTab === "news" && (
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

        {activeTab === "squad" && (
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
                <th className="num">Trend</th>
                <th className="num">Fit</th>
                <th className="num">Form</th>
                <th className="num">Exp</th>
              </tr>
            </thead>
            <tbody>
              <SquadRows world={world} teamId={myTeamId} searchQuery={searchQuery} />
            </tbody>
          </table>
        )}

        {activeTab === "saves" && (
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
        </div>
      </Panel>
    </div>
  );
}

const TAB_LABELS: Record<string, string> = {
  today: "Fixtures",
  table: "Table",
  records: "Records",
  news: "News",
  squad: "Squad",
  saves: "Saves",
};

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
  const fixture = career.state.fixtures.find((f) => f.id === career.nextFixtureId);
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
      <Button onClick={onSelect} disabled={busy}>Select XI</Button>
      <Button onClick={onPlay} disabled={busy}>Play match</Button>
    </div>
  );
}

function SquadRows({
  world,
  teamId,
  searchQuery,
}: {
  world: World;
  teamId: number;
  searchQuery: string;
}) {
  const [players, setPlayers] = useState(
    world.players.filter((p) => p.team_id === teamId),
  );

  useEffect(() => {
    api
      .getSquad(teamId)
      .then(setPlayers)
      .catch(() => undefined);
  }, [teamId]);

  // Quick search from the sidebar filters the squad list.
  const q = searchQuery.trim().toLowerCase();
  const visible = q
    ? players.filter((p) => p.name.toLowerCase().includes(q))
    : players;

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
      {visible.map((p) => {
        const bestSkill = Math.max(p.skills.bat, p.skills.pace, p.skills.spin);
        const bestPotential = Math.max(p.potential.bat, p.potential.pace, p.potential.spin);
        const headroom = bestPotential - bestSkill;
        const trend =
          p.age <= 24 && headroom > 3 ? "▲" : p.age >= 32 && headroom <= 3 ? "▼" : headroom > 3 ? "△" : "·";
        return (
          <tr key={p.id}>
            <td>
              {p.name}
              {p.injury_days > 0 && (
                <span className="pill pill--warn" style={{ marginLeft: "0.4rem" }}>
                  inj {p.injury_days}d
                </span>
              )}
            </td>
            <td className="num">{p.age}</td>
            <td className="muted">{p.role.replace(/_/g, " ")}</td>
            <td className="num">{p.skills.bat}</td>
            <td className="num">{Math.max(p.skills.pace, p.skills.spin)}</td>
            <td className="num">{p.skills.fielding}</td>
            <td className="num muted">{p.potential.bat}</td>
            <td className="num" title={`Headroom: ${headroom} points to potential`}>
              <span
                className={
                  trend === "▲" ? "pill pill--ok" : trend === "▼" ? "pill pill--warn" : "pill"
                }
              >
                {trend}
              </span>
            </td>
            <td className="num" title="Fitness — drops when playing, recovers on rest days">
              <FitBar value={p.fitness} />
            </td>
            <td className="num" title="Form — recent performances">
              <FormArrow value={p.form} />
            </td>
            <td className="num muted" title="Experience — builds with playing time">
              {p.experience}
            </td>
          </tr>
        );
      })}
    </>
  );
}

function FitBar({ value }: { value: number }) {
  const tone = value >= 85 ? "pill--ok" : value >= 60 ? "" : "pill--warn";
  return (
    <span className={`pill ${tone}`} style={{ minWidth: 42, justifyContent: "center" }}>
      {value}
    </span>
  );
}

function FormArrow({ value }: { value: number }) {
  if (value > 1) return <span style={{ color: "var(--green-bar)" }}>▲ {value}</span>;
  if (value < -1) return <span style={{ color: "var(--red)" }}>▼ {value}</span>;
  return <span className="muted">{value > 0 ? `+${value}` : value}</span>;
}

function dayHasMyTeam(fixtures: CareerView["state"]["fixtures"], day: number, teamId: number) {
  return fixtures.some((f) => f.day === day && (f.home_id === teamId || f.away_id === teamId));
}
