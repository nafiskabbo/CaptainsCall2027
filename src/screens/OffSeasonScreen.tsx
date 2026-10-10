//! Off-season screen: season report, transfer window, finances.

import { useEffect, useState } from "react";
import { api } from "../tools/api";
import type { CareerView, SaveSummary, World, Player } from "../types";
import TrainingScreen from "./TrainingScreen";
import { Button } from "../ui";

interface Props {
  world: World;
  career: CareerView;
  onCareer: (view: CareerView) => void;
  onExit: () => void;
  onNewSeason: () => void;
}

const ROLES = ["batter", "wicketkeeper", "allrounder", "pace_bowler", "spin_bowler"];

interface Listing {
  player_id: number;
  name: string;
  age: number;
  role: string;
  from_team: string;
  wage: number;
  reputation: number;
  interested_in_you: boolean;
  value: number;
}

interface Finances {
  budget: number;
  wageBill: number;
  squadSize: number;
}

export default function OffSeasonScreen({
  world,
  career,
  onCareer,
  onExit,
  onNewSeason,
}: Props) {
  const [tab, setTab] = useState<
  | "report"
  | "transfers"
  | "squad"
  | "training"
  | "saves"
>("report");
  const [pool, setPool] = useState<Listing[]>([]);
  const [finances, setFinances] = useState<Finances | null>(null);
  const [saves, setSaves] = useState<SaveSummary[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [roleFilter, setRoleFilter] = useState<string>("all");

  const myTeam = career.state.player_team_id;

  useEffect(() => {
    api.listSaves().then(setSaves).catch(() => undefined);
    api.getFinances().then(setFinances).catch(() => undefined);
    api
      .getTransferPool()
      .then(setPool)
      .catch(() => undefined);
  }, [career.state.season, career.state.players.length]);

  const squad: Player[] = career.state.players.filter((p) => p.team_id === myTeam);

  const bid = async (listing: Listing) => {
    setBusy(true);
    setError(null);
    try {
      const outcome = await api.bidForPlayer(listing.player_id, listing.wage);
      setStatus(`${listing.name}: ${describeOutcome(outcome)}`);
      if (outcome === "Accepted" || (typeof outcome === "object" && "wage" in outcome)) {
        onCareer(await api.getCareer());
        setPool(await api.getTransferPool());
        setFinances(await api.getFinances());
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const release = async (playerId: number) => {
    setBusy(true);
    setError(null);
    try {
      await api.releasePlayer(playerId);
      setStatus("Player released.");
      onCareer(await api.getCareer());
      setFinances(await api.getFinances());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const load = async (id: number) => {
    setBusy(true);
    onCareer(await api.loadCareer(id));
    setStatus(`Game ${id} loaded`);
  };

  const filteredPool = pool.filter(
    (l) => roleFilter === "all" || l.role === roleFilter,
  );

  return (
    <div className="stack">
      <section className="panel">
        <div className="panel__head"><span className="panel__label--gold">Off-season · Season {career.state.season} complete</span></div>
        <p className="muted">
          Roll over into season {career.state.season + 1}: players age, retirees are
          announced, the academy replenishes squads and a fresh fixture list is drawn.
        </p>
        <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.6rem", flexWrap: "wrap" }}>
          <Button onClick={onNewSeason} disabled={busy}>Begin season {career.state.season + 1}</Button>
          <button
            className="btn"
            onClick={() => {
              api.saveCareer();
              setStatus("Saved");
            }}
          >
            Save
          </button>
          <Button variant="ghost" onClick={onExit}>Main menu</Button>
        </div>
      </section>

      {finances && (
        <section className="panel">
          <div className="panel__head"><span className="panel__label--gold">Club finances</span></div>
          <div className="statgrid">
            <div className="stat">
              <div className="stat__label">Budget</div>
              <div className="stat__value stat__value--gold">
                {formatMoney(finances.budget)}
              </div>
            </div>
            <div className="stat">
              <div className="stat__label">Wage bill</div>
              <div className="stat__value">{formatMoney(finances.wageBill)}</div>
            </div>
            <div className="stat">
              <div className="stat__label">Squad</div>
              <div className="stat__value">{finances.squadSize}</div>
            </div>
          </div>
        </section>
      )}

      {status && <p className="muted">{status}</p>}
      {error && <p className="error">{error}</p>}

      <section className="panel panel--flush">
        <div style={{ display: "flex", gap: "0.4rem", padding: "0.5rem 0.6rem 0", alignItems: "center" }}>
          {(
            [
              ["report", "Season report"],
              ["transfers", "Transfer window"],
              ["squad", "Squad"],
              ["training", "Training"],
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
        </div>

        {tab === "report" && (
          <div style={{ padding: "0.6rem" }}>
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
                  <tr key={r.team_id} className={r.team_id === myTeam ? "row--mine" : ""}>
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
          </div>
        )}

        {tab === "transfers" && (
          <div style={{ padding: "0.6rem" }}>
            <div className="field" style={{ maxWidth: 220 }}>
              <label htmlFor="role">Filter by role</label>
              <select
                id="role"
                value={roleFilter}
                onChange={(e) => setRoleFilter(e.target.value)}
              >
                <option value="all">All roles</option>
                {ROLES.map((r) => (
                  <option key={r} value={r}>
                    {r.replace(/_/g, " ")}
                  </option>
                ))}
              </select>
            </div>
            <table className="data">
              <thead>
                <tr>
                  <th>Player</th>
                  <th className="num">Age</th>
                  <th>Role</th>
                  <th>From</th>
                  <th className="num">Rating</th>
                  <th className="num">Wage</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {filteredPool.map((l) => (
                  <tr key={l.player_id}>
                    <td>{l.name}</td>
                    <td className="num">{l.age}</td>
                    <td className="muted">{l.role.replace(/_/g, " ")}</td>
                    <td className="muted">{l.from_team}</td>
                    <td className="num">{l.reputation}</td>
                    <td className="num">{formatMoney(l.wage)}</td>
                    <td className="num">
                      {l.interested_in_you ? (
                        <button
                          className="btn btn--sm btn--primary"
                          onClick={() => bid(l)}
                          disabled={busy}
                        >
                          Sign
                        </button>
                      ) : (
                        <span className="pill pill--warn">not interested</span>
                      )}
                    </td>
                  </tr>
                ))}
                {filteredPool.length === 0 && (
                  <tr>
                    <td colSpan={7} className="muted">
                      No players available in this window.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}

        {tab === "squad" && (
          <div style={{ padding: "0.6rem" }}>
            <table className="data">
              <thead>
                <tr>
                  <th>Player</th>
                  <th className="num">Age</th>
                  <th>Role</th>
                  <th className="num">Bat</th>
                  <th className="num">Bowl</th>
                  <th className="num">Fit</th>
                  <th />
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
                    <td className="num">{p.fitness}</td>
                    <td className="num">
                      <button
                        className="btn btn--sm btn--ghost"
                        onClick={() => release(p.id)}
                        disabled={busy || squad.length <= 11}
                        title={
                          squad.length <= 11
                            ? "You need at least 11 players"
                            : "Release from contract"
                        }
                      >
                        Release
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {tab === "training" && (
          <div style={{ padding: "0.6rem" }}>
            <TrainingScreen
              career={career}
              onSaved={(message) => setStatus(message)}
            />
          </div>
        )}

        {tab === "saves" && (
          <table className="data">
            <thead>
              <tr>
                <th>Name</th>
                <th>Team</th>
                <th className="num">Day</th>
                <th className="num">Season</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {saves.map((s) => (
                <tr key={s.id}>
                  <td>{s.name}</td>
                  <td>{s.team_name}</td>
                  <td className="num">{s.season_day}</td>
                  <td className="num">—</td>
                  <td className="num">
                    <button className="btn btn--sm" onClick={() => load(s.id)} disabled={busy}>
                      Load
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

function describeOutcome(outcome: string | { wage: number } | Record<string, unknown>): string {
  if (typeof outcome === "string") return outcome;
  if (outcome && typeof outcome === "object") {
    if ("wage" in outcome) return "signed!";
    if ("wanted" in outcome) return "refused — wage too low";
    if ("available" in outcome) return "not enough funds";
    return "not interested";
  }
  return "unknown";
}

function formatMoney(amount: number): string {
  if (amount >= 1_000_000) return `${(amount / 1_000_000).toFixed(1)}M`;
  if (amount >= 1_000) return `${Math.round(amount / 1000)}k`;
  return String(amount);
}
