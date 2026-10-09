//! Runs-per-over Manhattan chart with wicket markers — a Cricket Coach staple.

import type { Innings } from "../types";

interface Props {
  innings: Innings | null;
}

export default function ManhattanChart({ innings }: Props) {
  if (!innings) return null;

  const overs = innings.over_summaries;
  if (overs.length === 0) {
    return (
      <div className="manhattan" aria-label="No overs bowled yet">
        <span className="muted" style={{ margin: "auto" }}>
          No overs bowled yet
        </span>
      </div>
    );
  }

  const maxRuns = Math.max(6, ...overs.map((o) => o.runs));
  const last = overs[overs.length - 1].over;

  return (
    <div>
      <div className="manhattan" role="img" aria-label="Runs per over chart">
        {overs.map((o) => (
          <div key={o.over} className="manhattan__col" title={`Over ${o.over}: ${o.runs} runs`}>
            <span className="manhattan__w">{o.wickets > 0 ? "W" : ""}</span>
            <div
              className={`manhattan__bar ${o.wickets > 0 ? "manhattan__bar--wicket" : ""}`}
              style={{ height: `${Math.max(4, (o.runs / maxRuns) * 100)}%` }}
            />
            <span className="muted" style={{ fontSize: "0.56rem" }}>
              {o.over}
            </span>
          </div>
        ))}
      </div>
      <div className="manhattan__axis">
        <span>1</span>
        <span>…{last}</span>
      </div>
    </div>
  );
}
