import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../tools/api";
import type {
  FieldPreset,

  MatchInput,
  MatchSnapshot,
  World,
} from "../types";
import MatchView from "./MatchView";

interface Props {
  world: World;
  snapshot: MatchSnapshot;
  onSnapshot: (s: MatchSnapshot) => void;
  onExit: () => void;
}

const FIELD_PRESETS: { value: FieldPreset; label: string }[] = [
  { value: "Attacking", label: "Attacking" },
  { value: "Balanced", label: "Balanced" },
  { value: "Defensive", label: "Defensive" },
  { value: "BoundaryRiding", label: "Boundary riding" },
  { value: "SpinTrap", label: "Spin trap" },
];


function oversText(balls: number): string {
  return `${Math.floor(balls / 6)}.${balls % 6}`;
}

function strikeRate(runs: number, balls: number): string {
  if (balls === 0) return "—";
  return ((runs / balls) * 100).toFixed(1);
}

function cardsFor(innings: MatchSnapshot["innings"][number], batting: boolean) {
  const rows = batting
    ? innings.batting_cards.map((c) => ({
        id: c.player_id,
        name: c.name,
        primary: `${c.runs}`,
        secondary: `${c.balls} b · ${c.fours}x4 · ${c.sixes}x6 · SR ${strikeRate(c.runs, c.balls)}`,
        note: c.out ? dismissalText(c.out.out_type) : "not out",
      }))
    : innings.bowling_cards.map((c) => ({
        id: c.player_id,
        name: c.name,
        primary: `${c.wickets}/${c.runs}`,
        secondary: `${oversText(c.balls)} ov · wd ${c.wides} · nb ${c.no_balls} · Econ ${
          c.balls > 0 ? ((c.runs / c.balls) * 6).toFixed(2) : "0.00"
        }`,
        note: c.balls > 0 ? `${(c.balls / 6).toFixed(1)} overs` : "did not bowl",
      }));
  return rows;
}

function dismissalText(kind: string): string {
  switch (kind) {
    case "Caught":
      return "c fielder";
    case "Bowled":
      return "b bowler";
    case "Lbw":
      return "lbw";
    case "RunOut":
      return "run out";
    case "Stumped":
      return "stumped";
    case "CaughtAndBowled":
      return "c & b";
    case "HitWicket":
      return "hit wicket";
    default:
      return "out";
  }
}

export default function MatchScreen({ world, snapshot, onSnapshot, onExit }: Props) {
  const [tab, setTab] = useState<"batting" | "bowling" | "overs" | "comment">("batting");
  const [busy, setBusy] = useState(false);
  const [scorecard, setScorecard] = useState<string | null>(null);
  const inputRef = useRef<MatchInput>({
    batting_aggression: 50,
    keep_strike_toggle: false,
    hit_and_run_toggle: false,
    target_bowler_id: 0,
    field_preset: "Balanced",
  });
  const input = inputRef.current;

  const innings = snapshot.innings[snapshot.current_innings];
  const battingTeam = world.teams.find((t) => t.id === innings?.batting_team_id);
  const bowlingTeam = world.teams.find((t) => t.id === innings?.bowling_team_id);
  const bowler = world.players.find((p) => p.id === snapshot.bowler_id);
  const striker = world.players.find((p) => p.id === snapshot.striker_id);
  const maxOvers = snapshot.format === "odi50" ? 50 : snapshot.format === "t20" ? 20 : null;
  const runRate = innings && innings.balls > 0 ? (innings.runs / (innings.balls / 6)) : 0;

  const battingRows = useMemo(
    () => (innings ? cardsFor(innings, true) : []),
    [innings],
  );
  const bowlingRows = useMemo(
    () => (innings ? cardsFor(innings, false) : []),
    [innings],
  );

  // Keep the Rust-side orders in sync whenever the UI changes them.
  useEffect(() => {
    api.setMatchInput(input).catch(() => undefined);
  }, [input]);

  const advance = async (n: number) => {
    setBusy(true);
    try {
      const next = await api.simMatch(n);
      onSnapshot(next);
    } finally {
      setBusy(false);
    }
  };

  const toEndOfInnings = async () => {
    setBusy(true);
    try {
      const next = await api.simInnings();
      onSnapshot(next);
    } finally {
      setBusy(false);
    }
  };

  const exportCard = async () => {
    const html = await api.exportScorecard();
    setScorecard(html);
  };

  const setAggression = (v: number) => {
    input.batting_aggression = v;
  };

  const complete = snapshot.phase === "Complete";
  const winner = snapshot.result?.winner_team_id
    ? world.teams.find((t) => t.id === snapshot.result?.winner_team_id)
    : null;

  return (
    <div className="match">
      <div className="scorebug">
        <div className="scorebug__team">
          <span className="scorebug__name">{battingTeam?.name ?? "—"}</span>
          <span className="scorebug__score">
            {innings ? `${innings.runs}/${innings.wickets}` : "—"}
          </span>
          <span className="scorebug__meta">
            {innings ? overheadText(innings.balls, maxOvers) : ""}
          </span>
        </div>
        <div className="scorebug__right">
          <span className="scorebug__rr">RR {runRate.toFixed(2)}</span>
          {snapshot.format === "test" && (
            <span className="scorebug__meta">Day {snapshot.clock.day}</span>
          )}
          <span className="scorebug__meta">
            {world.grounds.find((g) => g.id === snapshot.ground_id)?.name ?? ""}
          </span>
        </div>
      </div>

      <div className="match__body">
        <section className="panel panel--pitch">
          <MatchView snapshot={snapshot} />
          <div className="pitchmeta">
            <div>
              <strong>{innings ? striker?.name ?? "—" : "—"}</strong> on strike
              {striker && ` · SR ${strikeRate(
                innings?.batting_cards.find((c) => c.player_id === striker.id)?.runs ?? 0,
                innings?.batting_cards.find((c) => c.player_id === striker.id)?.balls ?? 0,
              )}`}
            </div>
            <div>
              Bowling <strong>{bowler?.name ?? "—"}</strong>
              {bowler && ` · rating ${Math.max(bowler.skills.pace, bowler.skills.spin)}`}
            </div>
          </div>
        </section>

        <section className="panel panel--controls">
          <h3>Tactics</h3>

          <label className="slider">
            <span>
              Batting aggression <b>{input.batting_aggression}</b>
            </span>
            <input
              type="range"
              min={0}
              max={100}
              value={input.batting_aggression}
              onChange={(e) => setAggression(Number(e.target.value))}
            />
          </label>

          <div className="toggles">
            <label>
              <input
                type="checkbox"
                checked={input.keep_strike_toggle}
                onChange={(e) => {
                  input.keep_strike_toggle = e.target.checked;
                }}
              />
              Keep the strike
            </label>
            <label>
              <input
                type="checkbox"
                checked={input.hit_and_run_toggle}
                onChange={(e) => {
                  input.hit_and_run_toggle = e.target.checked;
                }}
              />
              Hit and run
            </label>
          </div>

          <div className="field">
            <label htmlFor="preset">Field preset</label>
            <select
              id="preset"
              value={input.field_preset}
              onChange={(e) => {
                input.field_preset = e.target.value as FieldPreset;
              }}
            >
              {FIELD_PRESETS.map((p) => (
                <option key={p.value} value={p.value}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>

          <div className="field">
            <label htmlFor="bowler">Bring on</label>
            <select
              id="bowler"
              value={input.target_bowler_id}
              onChange={(e) => {
                input.target_bowler_id = Number(e.target.value);
              }}
            >
              <option value={0}>Engine choice</option>
              {bowlingTeam &&
                world.players
                  .filter((p) => p.team_id === bowlingTeam.id)
                  .filter((p) => Math.max(p.skills.pace, p.skills.spin) > 40)
                  .map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name} · {Math.max(p.skills.pace, p.skills.spin)}
                    </option>
                  ))}
            </select>
          </div>

          <h3>Sim controls</h3>
          <div className="btnrow">
            <button className="btn" disabled={busy || complete} onClick={() => advance(1)}>
              Ball
            </button>
            <button className="btn" disabled={busy || complete} onClick={() => advance(6)}>
              Over
            </button>
            <button
              className="btn"
              disabled={busy || complete}
              onClick={() => advance(snapshot.format === "test" ? 30 : 12)}
            >
              Session
            </button>
            <button className="btn" disabled={busy || complete} onClick={toEndOfInnings}>
              End innings
            </button>
            <button className="btn" disabled={busy || complete} onClick={() => advance(100000)}>
              To result
            </button>
          </div>

          <div className="btnrow">
            <button className="btn btn--ghost" onClick={exportCard}>
              Export scorecard
            </button>
            <button className="btn btn--ghost" onClick={onExit}>
              Back to career
            </button>
          </div>

          {complete && (
            <p className="result">
              {snapshot.result?.summary ?? "Complete"}
              {winner && ` — ${winner.name}`}
            </p>
          )}

          {scorecard && (
            <p className="muted">Scorecard HTML generated ({scorecard.length} bytes).</p>
          )}
        </section>
      </div>

      <section className="panel">
        <nav className="tabs">
          {(
            [
              ["batting", "Batting"],
              ["bowling", "Bowling"],
              ["overs", "Overs"],
              ["comment", "Commentary"],
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

        {tab === "batting" && (
          <table className="data">
            <thead>
              <tr>
                <th>Batter</th>
                <th className="num">Runs</th>
                <th>Detail</th>
                <th>Out</th>
              </tr>
            </thead>
            <tbody>
              {battingRows.map((r) => (
                <tr key={r.id}>
                  <td>{r.name}</td>
                  <td className="num strong">{r.primary}</td>
                  <td className="muted">{r.secondary}</td>
                  <td className="muted">{r.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "bowling" && (
          <table className="data">
            <thead>
              <tr>
                <th>Bowler</th>
                <th className="num">W/R</th>
                <th>Detail</th>
                <th>Overs</th>
              </tr>
            </thead>
            <tbody>
              {bowlingRows.map((r) => (
                <tr key={r.id}>
                  <td>{r.name}</td>
                  <td className="num strong">{r.primary}</td>
                  <td className="muted">{r.secondary}</td>
                  <td className="muted">{r.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "overs" && (
          <div className="overgrid">
            {(innings?.over_summaries ?? []).map((o) => (
              <div key={o.over} className="overchip">
                <b>{o.over}</b>
                <span>
                  {o.runs}r{o.wickets > 0 ? ` ${o.wickets}w` : ""}
                </span>
                <small>{world.players.find((p) => p.id === o.bowler_id)?.name.split(" ")[1] ?? ""}</small>
              </div>
            ))}
            {(innings?.over_summaries.length ?? 0) === 0 && <p className="muted">No overs bowled yet.</p>}
          </div>
        )}

        {tab === "comment" && (
          <ul className="commentary">
            {snapshot.recent_balls.map((b, i) => (
              <li key={i}>
                <span className="commentary__ball">
                  {b.dismissal ? "W" : b.runs === 0 ? "•" : b.runs}
                  {b.extra_runs > 0 ? "e" : ""}
                </span>
                {b.commentary}
              </li>
            ))}
            {snapshot.recent_balls.length === 0 && <li className="muted">No balls bowled yet.</li>}
          </ul>
        )}
      </section>
    </div>
  );
}

function overheadText(balls: number, maxOvers: number | null): string {
  const overs = `${Math.floor(balls / 6)}.${balls % 6}`;
  return maxOvers ? `${overs} / ${maxOvers} ov` : `${overs} ov`;
}
