//! End-of-season summary: final table, awards, records and what changed.
//!
//! Shown when the last fixture is played, before the off-season screen.

import { useEffect, useState } from "react";
import { api } from "../tools/api";
import type { CareerView, Player, World } from "../types";

interface Props {
  world: World;
  career: CareerView;
  onOffSeason: () => void;
  onSave: () => void;
}

interface Awards {
  leadingRun: string | null;
  leadingWicket: string | null;
}

export default function SeasonSummary({ world, career, onOffSeason, onSave }: Props) {
  const [squad, setSquad] = useState<Player[]>([]);
  const position = career.table.findIndex((r) => r.team_id === career.state.player_team_id) + 1;
  const champions = career.table[0]?.team_id === career.state.player_team_id;

  useEffect(() => {
    api
      .getSquad(career.state.player_team_id)
      .then(setSquad)
      .catch(() => undefined);
  }, [career.state.player_team_id]);

  // Most improved and best performer from the squad.
  const sorted = [...squad].sort((a, b) => b.skills.bat - a.skills.bat);
  const best = sorted[0];
  const youngest = [...squad].sort((a, b) => a.age - b.age)[0];

  const awards: Awards = {
    leadingRun: career.records[0]?.name ?? null,
    leadingWicket: [...career.records].sort((a, b) => b.wickets - a.wickets)[0]?.name ?? null,
  };

  return (
    <div className="stack">
      <section className="panel">
        <div className="panel__head"><span className="panel__label--gold">Season {career.state.season} complete</span></div>
        <p style={{ fontSize: "1.05rem" }}>
          {champions ? (
            <>
              <b style={{ color: "var(--green-bar)" }}>Champions!</b> {career.state.player_team_name}{" "}
              win the competition.
            </>
          ) : (
            <>
              {career.state.player_team_name} finished <b>{position}</b>
              {position === 1 ? "st" : position === 2 ? "nd" : position === 3 ? "rd" : "th"} of{" "}
              {career.table.length}.
            </>
          )}
        </p>

        <div className="statgrid">
          <div className="stat">
            <div className="stat__label">Final position</div>
            <div className="stat__value stat__value--gold">
              {position}/{career.table.length}
            </div>
          </div>
          <div className="stat">
            <div className="stat__label">Played</div>
            <div className="stat__value">
              {career.table.find((r) => r.team_id === career.state.player_team_id)?.played ?? 0}
            </div>
          </div>
          <div className="stat">
            <div className="stat__label">Points</div>
            <div className="stat__value">
              {career.table.find((r) => r.team_id === career.state.player_team_id)?.points ?? 0}
            </div>
          </div>
          <div className="stat">
            <div className="stat__label">Squad size</div>
            <div className="stat__value">{squad.length}</div>
          </div>
        </div>
      </section>

      <section className="panel">
        <div className="panel__head"><span className="panel__label--gold">Season awards</span></div>
        <table className="data">
          <tbody>
            <tr>
              <td>Leading run-scorer</td>
              <td className="strong">{awards.leadingRun ?? "—"}</td>
            </tr>
            <tr>
              <td>Leading wicket-taker</td>
              <td className="strong">{awards.leadingWicket ?? "—"}</td>
            </tr>
            <tr>
              <td>Highest-rated player</td>
              <td className="strong">
                {best ? `${best.name} (bat ${best.skills.bat})` : "—"}
              </td>
            </tr>
            <tr>
              <td>Youngest in the squad</td>
              <td className="strong">{youngest ? `${youngest.name}, ${youngest.age}` : "—"}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section className="panel">
        <div className="panel__head"><span className="panel__label--gold">Final table</span></div>
        <table className="data">
          <thead>
            <tr>
              <th className="num">#</th>
              <th>Team</th>
              <th className="num">P</th>
              <th className="num">W</th>
              <th className="num">L</th>
              <th className="num">Pts</th>
            </tr>
          </thead>
          <tbody>
            {career.table.map((r, i) => (
              <tr key={r.team_id} className={r.team_id === career.state.player_team_id ? "row--mine" : ""}>
                <td className="num">{i + 1}</td>
                <td>{world.teams.find((t) => t.id === r.team_id)?.name ?? r.team_id}</td>
                <td className="num">{r.played}</td>
                <td className="num">{r.won}</td>
                <td className="num">{r.lost}</td>
                <td className="num strong">{r.points}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="panel" style={{ display: "flex", gap: "0.5rem" }}>
        <button className="btn btn--primary" onClick={onOffSeason}>
          Continue to off-season
        </button>
        <button className="btn" onClick={onSave}>
          Save career
        </button>
      </section>
    </div>
  );
}
