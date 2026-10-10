//! Match screen — 2D ground, live tactics, scorecards.
//! Layout modelled on Cricket Coach 2014's match/scoreboard screens.

import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../tools/api";
import { cueForBall, sound } from "../lib/audio";
import type {
  BallOutcome,
  FieldPreset,
  Format,
  MatchInput,
  MatchSnapshot,
  ShotChoice,
  World,
} from "../types";
import { SHOT_CHOICES, SHOT_LABEL } from "../types";
import MatchView from "./MatchView";
import { Button, Meter, Panel, Stat } from "../ui";
import ManhattanChart from "../components/ManhattanChart";

interface Props {
  world: World;
  snapshot: MatchSnapshot;
  onSnapshot: (s: MatchSnapshot) => void;
  onExit: () => void;
  /** Raised while the shell's Options sidebar item is the active one. */
  optionsOpen?: boolean;
}

const FIELD_PRESETS: { value: FieldPreset; label: string }[] = [
  { value: "attacking", label: "Attacking" },
  { value: "balanced", label: "Balanced" },
  { value: "defensive", label: "Defensive" },
  { value: "boundary_riding", label: "Boundary riding" },
  { value: "spin_trap", label: "Spin trap" },
];

const AGRESSION_LABELS: [number, string][] = [
  [0, "Block"],
  [25, "Cautious"],
  [50, "Normal"],
  [75, "Aggressive"],
  [100, "Slog"],
];

function oversText(balls: number): string {
  return `${Math.floor(balls / 6)}.${balls % 6}`;
}

function strikeRate(runs: number, balls: number): string {
  if (balls === 0) return "—";
  return ((runs / balls) * 100).toFixed(1);
}

function howOut(out: { out_type: string } | null): string {
  if (!out) return "not out";
  switch (out.out_type) {
    case "caught":
      return "c fielder";
    case "bowled":
      return "b bowler";
    case "lbw":
      return "lbw";
    case "run_out":
      return "run out";
    case "stumped":
      return "st";
    case "caught_and_bowled":
      return "c & b";
    case "hit_wicket":
      return "hit wkt";
    default:
      return "out";
  }
}

function roleShort(role: string): string {
  return role === "wicketkeeper" ? "WK" : role === "pace_bowler" ? "RF" : role === "spin_bowler" ? "SP" : role === "allrounder" ? "AR" : "BAT";
}

export default function MatchScreen({ world, snapshot, onSnapshot, onExit, optionsOpen }: Props) {
  const [tab, setTab] = useState<"batting" | "bowling" | "overs" | "chart">("batting");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  // Orders are real state: mutating a ref would never re-render the controls.
  const [orders, setOrders] = useState<MatchInput>({
    batting_aggression: 50,
    keep_strike_toggle: false,
    hit_and_run_toggle: false,
    target_bowler_id: 0,
    field_preset: "balanced",
    shot_choice: null,
  });
  const input = orders;
  const patchOrders = (patch: Partial<MatchInput>) =>
    setOrders((prev) => ({ ...prev, ...patch }));

  const innings = snapshot.innings[snapshot.current_innings] ?? null;
  const battingTeam = world.teams.find((t) => t.id === innings?.batting_team_id);
  const bowlingTeam = world.teams.find((t) => t.id === innings?.bowling_team_id);
  const striker = world.players.find((p) => p.id === snapshot.striker_id);
  const nonStriker = world.players.find((p) => p.id === snapshot.non_striker_id);
  const bowler = world.players.find((p) => p.id === snapshot.bowler_id);

  const maxOvers: Record<Format, number | null> = {
    test: null,
    odi50: 50,
    t20: 20,
  };
  const totalOvers = maxOvers[snapshot.format];
  const runRate = innings && innings.balls > 0 ? (innings.runs / (innings.balls / 6)) : 0;

  // Fielder name lookup (surname only) for the ground view.
  const fielderNames = useMemo(() => {
    const map: Record<number, string> = {};
    if (bowlingTeam) {
      const squad = world.players.filter((p) => p.team_id === bowlingTeam.id);
      squad.slice(0, 9).forEach((p, i) => {
        map[i] = p.name.split(" ").slice(-1)[0];
      });
    }
    return map;
  }, [bowlingTeam, world.players]);

  const batterNames = useMemo(() => {
    const map: Record<number, string> = {};
    for (const p of [striker, nonStriker]) {
      if (p) map[p.id] = p.name.split(" ").slice(-1)[0];
    }
    return map;
  }, [striker, nonStriker]);

  // Keep Rust-side orders in sync.
  useEffect(() => {
    api.setMatchInput(input).catch(() => undefined);
  }, [orders]);

  /*
    The odds of each shot against the bowler currently in.

    These come from the engine, not from the risk table: the same distribution
    the next ball will actually be rolled against. They are fetched after the
    orders are pushed, because the preview reflects the committed shot — picking
    "6" and seeing its odds fall is the feedback that makes the risk legible.
  */
  const [odds, setOdds] = useState<Partial<Record<ShotChoice, number>>>({});
  const [outOdds, setOutOdds] = useState<number | undefined>(undefined);
  useEffect(() => {
    let stale = false;
    api
      .previewBall()
      .then((p) => {
        if (stale) return;
        const next: Partial<Record<ShotChoice, number>> = {};
        for (const c of SHOT_CHOICES) next[c] = p[c] ?? 0;
        setOdds(next);
        setOutOdds(p.out);
      })
      .catch(() => undefined);
    return () => {
      stale = true;
    };
  }, [orders, snapshot]);

  const advance = async (n: number) => {
    setBusy(true);
    try {
      onSnapshot(await api.simMatch(n));
    } finally {
      setBusy(false);
    }
  };

  const endInnings = async () => {
    setBusy(true);
    try {
      onSnapshot(await api.simInnings());
    } finally {
      setBusy(false);
    }
  };

  const exportCard = async () => {
    setBusy(true);
    try {
      const path = await api.exportScorecard();
      setStatus(`Scorecard saved to ${path}`);
    } catch (e) {
      setStatus(`Could not export scorecard: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const complete = snapshot.phase === "complete";
  const strikerCard = innings?.batting_cards.find((c) => c.player_id === snapshot.striker_id);
  const nonStrikerCard = innings?.batting_cards.find(
    (c) => c.player_id === snapshot.non_striker_id,
  );
  const bowlerCard = innings?.bowling_cards.find((c) => c.player_id === snapshot.bowler_id);

  /*
    Partnership: runs and balls since the last wicket.

    The ball count was previously `runs * 3` — a guess that printed "19 runs
    from 57 balls" for a partnership that had faced far fewer. Balls are now
    counted from the actual deliveries since the last wicket.
  */
  const partnership = useMemo(() => {
    if (!innings) return null;
    const start = innings.fow.length > 0 ? innings.fow[innings.fow.length - 1] : 0;
    const runs = innings.runs - start;
    // `recent_balls` is newest-first; count back to the last wicket.
    let balls = 0;
    for (const b of snapshot.recent_balls) {
      if (b.dismissal) break;
      balls += 1;
    }
    return { runs, balls };
  }, [innings, snapshot.recent_balls]);

  const homeTeam = world.teams.find((t) => t.id === snapshot.home_team_id);
  const awayTeam = world.teams.find((t) => t.id === snapshot.away_team_id);

  // Nation short code, used on the scorebug the way the original shows "ENG 43/1".
  const batCode = (battingTeam?.name ?? "---").slice(0, 3).toUpperCase();
  const bowlCode = (bowlingTeam?.name ?? "---").slice(0, 3).toUpperCase();

  /**
   * The last few overs rebuilt as ball-by-ball rows.
   *
   * The original shows this as a grid of coloured outcome icons — the single
   * most recognisable part of its match screen, and genuinely useful: you can
   * read the shape of an over at a glance instead of scanning commentary.
   * `over_summaries` only carries totals, so the balls are reconstructed from
   * the engine's own delivery history via `recent_balls`, newest first.
   */
  const overRows = useMemo(() => {
    if (!innings) return [];
    const currentOver = Math.floor(innings.balls / 6);
    const firstOver = Math.max(1, currentOver - 2);
    const rows: { over: number; bowler: string; balls: BallOutcome[]; runs: number }[] = [];
    // `recent_balls` is capped, so only claim the overs we can actually show.
    const available = snapshot.recent_balls.length;
    for (let over = firstOver; over <= currentOver; over++) {
      const start = (currentOver - over) * 6;
      const slice = snapshot.recent_balls.slice(start, start + 6);
      if (slice.length === 0) continue;
      rows.push({
        over,
        bowler:
          innings.over_summaries.find((o) => o.over === over)?.bowler_id !== undefined
            ? world.players.find((p) => p.id === innings.over_summaries.find((o) => o.over === over)?.bowler_id)
                ?.name.split(" ")
                .slice(-1)[0] ?? ""
            : "",
        balls: [...slice].reverse(),
        runs: slice.reduce((sum, b) => sum + b.runs + b.extra_runs, 0),
      });
    }
    void available;
    return rows;
  }, [innings, snapshot.recent_balls, world.players]);

  // ---- sound ----
  // Play a cue whenever the engine reports deliveries the UI has not seen yet.
  // `recent_balls` is newest-first and capped, so we track how many have been
  // consumed per innings rather than diffing the array.
  const soundRef = useRef<{ innings: number; seen: number; phase: string }>({
    innings: -1,
    seen: 0,
    phase: "pre_match",
  });
  useEffect(() => {
    const balls = snapshot.recent_balls ?? [];
    const marker = soundRef.current;
    if (snapshot.current_innings !== marker.innings) {
      // A new innings: reset the counter rather than replaying old balls.
      soundRef.current = { innings: snapshot.current_innings, seen: balls.length, phase: snapshot.phase };
      if (marker.innings !== -1 && marker.phase !== "complete") sound.play("inningsEnd");
      return;
    }
    const fresh = balls.length - marker.seen;
    if (fresh > 0) {
      // Newest-first, so play the newest cue and cap at a couple so a fast
      // forward does not machine-gun.
      for (const ball of balls.slice(0, Math.min(fresh, 2))) {
        sound.play(cueForBall(ball));
      }
      soundRef.current = { ...marker, seen: balls.length };
    }
    if (snapshot.phase === "complete" && marker.phase !== "complete") {
      sound.play("matchEnd");
    }
    soundRef.current.phase = snapshot.phase;
  }, [snapshot]);

  const [soundOn, setSoundOn] = useState(sound.getSettings().enabled);
  const toggleSound = () => {
    const next = !soundOn;
    setSoundOn(next);
    sound.setEnabled(next);
    if (next) sound.play("uiClick");
  };

  // Browsers only allow audio after a user gesture, so ask the engine to arm
  // its unlock listeners the first time the match screen mounts.
  useEffect(() => {
    sound.unlock();
  }, []);

  return (
    <div className="match">
      {/* innings strip */}
      <div className="inningsstrip">
        {snapshot.innings.map((inn, i) => {
          const team = world.teams.find((t) => t.id === inn.batting_team_id);
          return (
            <button
              key={i}
              type="button"
              className={`inningsstrip__item ${i === snapshot.current_innings ? "inningsstrip__item--active" : ""}`}
              onClick={() => undefined}
              title={`${team?.name ?? ""} innings`}
            >
              {team ? team.name.slice(0, 3).toUpperCase() : "???"} {inn.runs}/{inn.wickets}
            </button>
          );
        })}
        <div className="inningsstrip__spacer" />
        <span className="pill">
          {world.grounds.find((g) => g.id === snapshot.ground_id)?.name ?? "Ground"}
        </span>
        {snapshot.format === "test" && <span className="pill">Day {snapshot.clock.day}</span>}
      </div>

      <div className="match__body">
        {/* ---- ground + overlays ---- */}
        <div className="stack">
          {/* Big central score line, as the original shows above the ground. */}
          <div className="bigscore">
            <span className="bigscore__team">{battingTeam?.name}</span>
            <span className="bigscore__value">
              {innings ? `${innings.runs}/${innings.wickets}` : "0/0"}
            </span>
            <span className="bigscore__overs">
              {innings ? oversText(innings.balls) : "0.0"} overs
            </span>
            <span className="bigscore__batters">
              {striker && (
                <span>
                  {strikerCard?.runs ?? 0} ({strikerCard?.balls ?? 0}){" "}
                  <b>{striker.name.split(" ").slice(-1)[0]}</b>
                </span>
              )}
              {nonStriker && (
                <span>
                  {nonStrikerCard?.runs ?? 0} ({nonStrikerCard?.balls ?? 0}){" "}
                  <b>{nonStriker.name.split(" ").slice(-1)[0]}</b>
                </span>
              )}
              {bowler && (
                <span className="muted">
                  {bowlerCard?.wickets ?? 0}/{bowlerCard?.runs ?? 0}{" "}
                  <b>{bowler.name.split(" ").slice(-1)[0]}</b>
                </span>
              )}
            </span>
          </div>

          <div style={{ position: "relative" }}>
            <MatchView
              snapshot={snapshot}
              fielderNames={fielderNames}
              batterNames={batterNames}
              dayNight
            />

            {partnership !== null && (
              <div className="groundtag">
                Current Partnership
                <b>
                  {partnership.runs} run{partnership.runs === 1 ? "" : "s"} from {partnership.balls}{" "}
                  ball{partnership.balls === 1 ? "" : "s"}
                </b>
              </div>
            )}

            {/* commentary stack */}
            <div className="commentary-overlay">
              {snapshot.recent_balls
                .slice(0, 4)
                .reverse()
                .map((b, i) => (
                  <div
                    key={i}
                    className={`commentary-overlay__line ${
                      b.dismissal
                        ? "commentary-overlay__line--event"
                        : i === 0
                          ? "commentary-overlay__line--ball"
                          : ""
                    }`}
                  >
                    {i === 0 && innings
                      ? `${oversText(innings.balls)} ${bowler?.name.split(" ").slice(-1)[0] ?? "Bowler"} to ${striker?.name.split(" ").slice(-1)[0] ?? "Batter"}`
                      : b.commentary}
                  </div>
                ))}
            </div>

            {/* scorebar */}
            <div className="scorebar">
              <span className="scorebar__team">{batCode}</span>
              <span className="scorebar__score">
                {innings ? `${innings.runs}/${innings.wickets}` : "—"}
              </span>
              <span className="scorebar__overs">
                {innings ? `${oversText(innings.balls)} ov` : ""}
                {totalOvers ? ` / ${totalOvers}` : ""}
              </span>
              <div className="scorebar__batters">
                {striker && (
                  <span className="scorebar__batter">
                    <b>{striker.name.split(" ").slice(-1)[0]}</b>{" "}
                    <span>
                      {strikerCard?.runs ?? 0} ({strikerCard?.balls ?? 0})
                    </span>
                  </span>
                )}
                {nonStriker && (
                  <span className="scorebar__batter">
                    <b>{nonStriker.name.split(" ").slice(-1)[0]}</b>{" "}
                    <span>
                      {nonStrikerCard?.runs ?? 0} ({nonStrikerCard?.balls ?? 0})
                    </span>
                  </span>
                )}
                <span className="scorebar__batter">
                  <b>{bowler?.name.split(" ").slice(-1)[0] ?? "Bowler"}</b>{" "}
                  <span>
                    {bowlerCard?.wickets ?? 0}/{bowlerCard?.runs ?? 0}
                  </span>{" "}
                  <small>{bowlerCard ? oversText(bowlerCard.balls) : "0.0"}</small>
                </span>
              </div>
            </div>
          </div>

          {/* sim controls */}
          <div className="panel">
            <div className="btnrow" style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap" }}>
              <Button disabled={busy || complete} onClick={() =>advance(1)}>
                Ball</Button>
              <Button disabled={busy || complete} onClick={() =>advance(6)}>
                Over</Button>
              <Button disabled={busy || complete}
                onClick={() =>advance(snapshot.format === "test" ? 36 : 12)}
              >
                Session</Button>
              <Button disabled={busy || complete} onClick={endInnings}>End innings</Button>
              <Button variant="danger" disabled={busy || complete}
                onClick={() =>advance(100000)}
              >
                To result</Button>
              <span className="sidebar__spacer" />
              <Button
                variant="ghost"
                onClick={toggleSound}
                title={soundOn ? "Mute match sound" : "Enable match sound"}
                aria-pressed={soundOn}
              >
                {soundOn ? "Sound on" : "Sound off"}
              </Button>
              <Button variant="ghost" onClick={exportCard}>Export scorecard</Button>
            </div>
            {complete && (
              <p style={{ marginTop: "0.5rem", fontWeight: 700, color: "var(--gold)" }}>
                {snapshot.result?.summary ?? "Match complete"}
              </p>
            )}
            {status && <p className="muted">{status}</p>}
          </div>

          {/*
            The shot selector. This is the batting mechanic: you commit to an
            outcome and the engine resolves that attempt. Committing raises the
            chance of the outcome you asked for *and* the chance of being
            dismissed, by an amount that scales with how much the shot risks —
            so the choice is a real trade, not a formality.

            The icons are the same chips used in the over grid below, which is
            how the original does it: one control, read as both an input and a
            record of what happened.
          */}
          {!complete && (
            <div className="shotpicker">
              <span className="shotpicker__label">
                Play the shot
                <span className="muted"> — pick an outcome to commit to it</span>
              </span>
              {SHOT_CHOICES.map((choice) => (
                <button
                  key={choice}
                  type="button"
                  className={[
                    "ball",
                    `ball--${choice}`,
                    "ball--button",
                    orders.shot_choice === choice ? "ball--chosen" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() =>
                    patchOrders({
                      shot_choice: orders.shot_choice === choice ? null : choice,
                    })
                  }
                  aria-pressed={orders.shot_choice === choice}
                  title={
                    orders.shot_choice === choice
                      ? "Click to let the engine choose"
                      : `Attempt ${SHOT_LABEL[choice]}`
                  }
                >
                  <span className="ball__label">{SHOT_LABEL[choice]}</span>
                  {/*
                    The odds of this shot coming off, straight from the engine's
                    own distribution for this bowler. Without them the picker is a
                    guess with six buttons; with them it is a priced decision.
                  */}
                  {odds[choice] !== undefined && (
                    <em className="ball__odds">{Math.round(odds[choice] * 100)}%</em>
                  )}
                </button>
              ))}
              {/*
                The price of the attempt, as its own chip.

                This is not decoration: it is the other half of every decision in
                this picker. Choosing a six because it is worth 23% while the
                wicket sits at 41% would be indefensible — so the wicket is
                shown in the same visual language as the runs, where the player
                is already comparing. An earlier pass printed a hard-coded "risk
                86%" from a static table, which had nothing to do with the
                number the engine actually rolls and quietly contradicted the
                real odds sitting next to it.
              */}
              {outOdds !== undefined && (
                <span
                  className={[
                    "ball",
                    "ball--w",
                    "ball--chip",
                    "ball--readonly",
                    outOdds > (orders.shot_choice ? (odds[orders.shot_choice] ?? 0) : 0)
                      ? "ball--danger"
                      : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  title="Chance of being dismissed on this delivery"
                >
                  <span className="ball__label">W</span>
                  <em className="ball__odds">{Math.round(outOdds * 100)}%</em>
                </span>
              )}
            </div>
          )}

          {/*
            The outcome grid, in the original's layout: the last three overs
            as coloured ball icons. It is the most recognisable part of its
            match screen and genuinely useful — you read the shape of an over
            at a glance instead of scanning commentary.
          */}
          <section className="panel overgrid">
            <div className="overgrid__row overgrid__row--head">
              <span className="overgrid__cell">Over</span>
              <span className="overgrid__cell">Bowler</span>
              <span className="overgrid__cell overgrid__cell--balls">Balls</span>
              <span className="overgrid__cell overgrid__cell--num">Runs</span>
            </div>
            {overRows.map((row) => (
              <div className="overgrid__row" key={row.over}>
                <span className="overgrid__cell">
                  {row.over}
                  {ordinalSuffix(row.over)}
                </span>
                <span className="overgrid__cell">{row.bowler}</span>
                <span className="overgrid__cell overgrid__cell--balls">
                  {row.balls.map((b, i) => (
                    <span
                      key={i}
                      className={`ball ball--${ballClass(b)}`}
                      title={`${b.runs + b.extra_runs} — ${b.commentary}`}
                    >
                      {ballLabel(b)}
                    </span>
                  ))}
                </span>
                <span className="overgrid__cell overgrid__cell--num">{row.runs}</span>
              </div>
            ))}
            {overRows.length === 0 && (
              <p className="muted" style={{ padding: "0.35rem 0.5rem" }}>
                The first over is under way.
              </p>
            )}
          </section>
        </div>

        {/* ---- tactics + scorecards ---- */}
        {/*
          `optionsOpen` marks this column when the shell's Options sidebar item
          is active, so the sidebar selection is visible on the page.
        */}
        <div
          className={`stack ${optionsOpen ? "stack--active" : ""}`}
          id="match-tactics"
        >
          <Panel goldTitle="Tactics">

            <div className="field">
              <label htmlFor="aggression">Batting instructions</label>
              <select
                id="aggression"
                value={orders.batting_aggression}
                onChange={(e) => patchOrders({ batting_aggression: Number(e.target.value) })}
              >
                {AGRESSION_LABELS.map(([v, label]) => (
                  <option key={v} value={v}>
                    {label} ({v})
                  </option>
                ))}
              </select>
            </div>

            <div className="field">
              <label htmlFor="preset">Field setting</label>
              <select
                id="preset"
                value={input.field_preset}
                onChange={(e) => {
                  // Must go through state: assigning to `input` mutates the
                  // React state object, which never re-renders and leaves the
                  // field preset stuck on whatever the engine last saw.
                  patchOrders({ field_preset: e.target.value as FieldPreset });
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
                value={orders.target_bowler_id}
                onChange={(e) => patchOrders({ target_bowler_id: Number(e.target.value) })}
              >
                <option value={0}>Engine choice</option>
                {bowlingTeam &&
                  world.players
                    .filter((p) => p.team_id === bowlingTeam.id)
                    .filter((p) => Math.max(p.skills.pace, p.skills.spin) > 40)
                    .map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name} · {roleShort(p.role)} ·{" "}
                        {Math.max(p.skills.pace, p.skills.spin)}
                      </option>
                    ))}
              </select>
            </div>

            <label style={{ display: "flex", gap: "0.4rem", alignItems: "center" }}>
              <input
                type="checkbox"
                checked={orders.keep_strike_toggle}
                onChange={(e) => patchOrders({ keep_strike_toggle: e.target.checked })}
              />
              Order batsman to keep the strike
            </label>
            <label style={{ display: "flex", gap: "0.4rem", alignItems: "center" }}>
              <input
                type="checkbox"
                checked={orders.hit_and_run_toggle}
                onChange={(e) => patchOrders({ hit_and_run_toggle: e.target.checked })}
              />
              Order batsman to hit and run
            </label>
          </Panel>

          {/* conditions */}
          <Panel goldTitle="Conditions">
            <div className="statgrid">
              <Stat label="Run rate" value={runRate.toFixed(2)} emphasis />
              <Stat
                label="Last 10 ov"
                value={`${lastTenRuns(innings)}/${lastTenWickets(innings)}`}
              />
              {snapshot.format === "test" ? (
                <Stat label="Ball age" value={snapshot.clock.balls_today} />
              ) : (
                <Stat
                  label="Req. rate"
                  value={
                    snapshot.current_innings === 1 && snapshot.innings.length > 1
                      ? requiredRate(snapshot)
                      : "—"
                  }
                />
              )}
              <Stat label="Weather" value="Sunny" />
            </div>

            <Meter label="Pitch" value={pitchWear(innings)} />
            <Meter label="Spin info" value={pitchWear(innings) * 1.3} />
          </Panel>
        </div>
      </div>

      {/* scorecards */}
      <div className="panel" id="match-scorecard">
        <nav className="tabs">
          {(
            [
              ["batting", "Batting"],
              ["bowling", "Bowling"],
              ["overs", "Over by over"],
              ["chart", "Manhattan"],
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
                <th className="num">#</th>
                <th>Batter</th>
                <th>How out</th>
                <th className="num">R</th>
                <th className="num">B</th>
                <th className="num">4s</th>
                <th className="num">6s</th>
                <th className="num">SR</th>
              </tr>
            </thead>
            <tbody>
              {(innings?.batting_cards ?? []).map((c) => {
                const player = world.players.find((p) => p.id === c.player_id);
                return (
                  <tr
                    key={c.player_id}
                    className={c.player_id === snapshot.striker_id ? "row--mine" : ""}
                  >
                    <td className="num">{c.batting_position}</td>
                    <td>
                      {c.name}
                      {player ? <span className="muted"> ({roleShort(player.role)})</span> : null}
                    </td>
                    <td className="muted">{howOut(c.out)}</td>
                    <td className="num strong">{c.runs}</td>
                    <td className="num">{c.balls}</td>
                    <td className="num">{c.fours}</td>
                    <td className="num">{c.sixes}</td>
                    <td className="num">{strikeRate(c.runs, c.balls)}</td>
                  </tr>
                );
              })}
              <tr>
                <td colSpan={3} className="muted">
                  Extras
                </td>
                <td className="num strong">{innings?.extras ?? 0}</td>
                <td colSpan={4} />
              </tr>
            </tbody>
          </table>
        )}

        {tab === "bowling" && (
          <table className="data">
            <thead>
              <tr>
                <th>Bowler</th>
                <th className="num">O</th>
                <th className="num">M</th>
                <th className="num">R</th>
                <th className="num">W</th>
                <th className="num">Econ</th>
              </tr>
            </thead>
            <tbody>
              {(innings?.bowling_cards ?? []).map((c) => (
                <tr key={c.player_id} className={c.player_id === snapshot.bowler_id ? "row--mine" : ""}>
                  <td>{c.name}</td>
                  <td className="num">{oversText(c.balls)}</td>
                  <td className="num">{c.maidens}</td>
                  <td className="num">{c.runs}</td>
                  <td className="num strong">{c.wickets}</td>
                  <td className="num">
                    {c.balls > 0 ? ((c.runs / c.balls) * 6).toFixed(2) : "0.00"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {tab === "overs" && (
          <div style={{ display: "flex", flexWrap: "wrap", gap: "0.3rem" }}>
            {(innings?.over_summaries ?? []).map((o) => (
              <span className="pill" key={o.over} title={world.players.find((p) => p.id === o.bowler_id)?.name}>
                <b style={{ color: "var(--gold)" }}>{o.over}</b> {o.runs}r
                {o.wickets > 0 ? ` ${o.wickets}w` : ""}
              </span>
            ))}
            {(innings?.over_summaries.length ?? 0) === 0 && (
              <p className="muted">No overs bowled yet.</p>
            )}
          </div>
        )}

        {tab === "chart" && <ManhattanChart innings={innings} />}
      </div>

      {/* match summary */}
      <div className="panel" style={{ display: "flex", justifyContent: "space-between", gap: "1rem", flexWrap: "wrap" }}>
        <span>
          <b>{homeTeam?.name}</b> vs <b>{awayTeam?.name}</b>
          <span className="muted"> · {snapshot.format.toUpperCase()}</span>
        </span>
        <Button onClick={onExit}>Back to career</Button>
      </div>
    </div>
  );
}

type Innings = MatchSnapshot["innings"][number];

/** Colour class for a delivery, matching the original's outcome icons. */
function ballClass(b: BallOutcome): string {
  if (b.dismissal) return "w";
  if (b.is_boundary_six) return "six";
  if (b.is_boundary_four) return "four";
  if (b.extra) return "extra";
  if (b.runs > 0) return "run";
  return "dot";
}

/** The character shown inside an outcome icon. */
function ballLabel(b: BallOutcome): string {
  if (b.dismissal) return "W";
  if (b.extra) return b.extra === "Wide" ? "wd" : "nb";
  if (b.is_boundary_six) return "6";
  if (b.runs > 0) return String(b.runs);
  return "·";
}

/** 1st, 2nd, 3rd, 4th, 11th… */
function ordinalSuffix(n: number): string {
  const rem100 = n % 100;
  if (rem100 >= 11 && rem100 <= 13) return "th";
  return ["th", "st", "nd", "rd"][n % 10] ?? "th";
}

function lastTenRuns(innings: Innings | undefined): number {
  if (!innings) return 0;
  return innings.over_summaries.slice(-10).reduce((sum, o) => sum + o.runs, 0);
}

function lastTenWickets(innings: Innings | undefined): number {
  if (!innings) return 0;
  return innings.over_summaries.slice(-10).reduce((sum, o) => sum + o.wickets, 0);
}

function requiredRate(snapshot: MatchSnapshot): string {
  const second = snapshot.innings[1];
  const first = snapshot.innings[0];
  if (!second || !first) return "—";
  const target = first.runs + 1;
  const left = target - second.runs;
  const ballsTotal = snapshot.format === "odi50" ? 300 : 120;
  const ballsLeft = ballsTotal - second.balls;
  if (ballsLeft <= 0) return "—";
  return ((left / ballsLeft) * 6).toFixed(2);
}

function pitchWear(innings: MatchSnapshot["innings"][number] | undefined): number {
  if (!innings) return 5;
  // Wear grows with overs bowled, faster in Tests.
  const overs = innings.balls / 6;
  return Math.min(96, 6 + overs * 1.6);
}
