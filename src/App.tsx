import { useEffect, useState } from "react";
import { api } from "./tools/api";
import type { AppInfo, CareerView, MatchSnapshot, World } from "./types";
import AppShell, { type SidebarItem } from "./components/AppShell";
import CareerScreen from "./screens/CareerScreen";
import MatchScreen from "./screens/MatchScreen";
import WorldScreen from "./screens/WorldScreen";
import OffSeasonScreen from "./screens/OffSeasonScreen";
import SeasonSummary from "./screens/SeasonSummary";
import { seasonClock, seasonDate } from "./lib/date";

type Screen = "setup" | "career" | "match" | "summary" | "offseason";

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [world, setWorld] = useState<World | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [screen, setScreen] = useState<Screen>("setup");
  const [career, setCareer] = useState<CareerView | null>(null);
  const [snapshot, setSnapshot] = useState<MatchSnapshot | null>(null);
  /** Sidebar-driven tab + squad search, shared with CareerScreen. */
  const [careerTab, setCareerTab] = useState("fixtures");
  const [query, setQuery] = useState("");

  useEffect(() => {
    Promise.all([api.getAppInfo(), api.getWorld()])
      .then(([appInfo, loadedWorld]) => {
        setInfo(appInfo);
        setWorld(loadedWorld);
      })
      .catch((e) => setError(String(e)));
  }, []);

  if (error || !info || !world) {
    return (
      <main className="shell">
        <h1>Captain&apos;s Call 2027</h1>
        {error ? (
          <p className="error">
            Failed to reach the Rust core: {error}
            <br />
            Run with <code>npm run tauri:dev</code>.
          </p>
        ) : (
          <p className="muted">Loading world database…</p>
        )}
      </main>
    );
  }

  const startCareer = async (teamId: number, name: string, seed: number) => {
    const view = await api.startCareer({ teamId, name, seed });
    setCareer(view);
    setCareerTab("fixtures");
    setQuery("");
    setScreen("career");
  };

  if (screen === "setup") {
    return (
      <div className="app">
        <header className="topbar">
          <div className="topbar__clock">
            <b>09:00</b>
            <span>Pre-season</span>
          </div>
          <div className="topbar__title">
            <h1>Captain&apos;s Call 2027</h1>
            <p>2D cricket management · tribute to Cricket Coach 2014</p>
          </div>
          <div className="topbar__flags">
            <span className="flag flag--home">CC27</span>
          </div>
        </header>
        <div className="content">
          <div className="content__inner">
            <WorldScreen world={world} onStartCareer={startCareer} />
          </div>
        </div>
        <div className="actionbar">
          <span className="actionbar__message">
            {world.players.length} players · {world.teams.length} teams ·{" "}
            {world.nations.length} nations · database {world.db_version}
          </span>
          <span className="muted">Pick a club to continue</span>
        </div>
      </div>
    );
  }

  if (screen === "career" && career) {
    const nation = world.nations.find((n) => n.id === career.state.nation_id);
    const items: SidebarItem[] = [
      { id: "fixtures", label: "Fixtures", icon: "▤", active: true },
      { id: "table", label: "Table", icon: "▮", onClick: () => setCareerTab("table") },
      { id: "records", label: "Records", icon: "▮", onClick: () => setCareerTab("records") },
      { id: "squad", label: "Squad", icon: "●", onClick: () => setCareerTab("squad") },
      { id: "news", label: "News", icon: "✉", onClick: () => setCareerTab("news") },
    ];

    return (
      <AppShell
        clock={seasonClock(career.state.season_day)}
        clockTitle={`${seasonDate(career.state.season_day)} · Day ${career.state.season_day}`}
        title={career.state.player_team_name}
        subtitle={career.state.name}
        homeFlag={(nation?.short_name ?? "CC").slice(0, 3).toUpperCase()}
        items={items}
        quickSearch={
          <input
            type="text"
            placeholder="Search squad…"
            aria-label="Search players"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        }
        actionBar={
          <>
            <button
              className="btn"
              onClick={() => api.advanceDay().then(setCareer)}
              disabled={career.season_complete}
            >
              ««
            </button>
            <button
              className="btn"
              onClick={() => api.advanceDay().then(setCareer)}
              disabled={career.season_complete}
            >
              Fast Forward
            </button>
            <button
              className="btn"
              onClick={() => api.advanceDay().then(setCareer)}
              disabled={career.season_complete}
            >
              »»
            </button>
            <span className="actionbar__message">
              {career.season_complete
                ? "Season complete"
                : `${world.competitions.find((c) => c.id === career.state.competition_id)?.name ?? "Competition"} · next match day ${
                    career.state.fixtures.find((f) => f.id === career.next_fixture_id)?.day ?? "—"
                  }`}
            </span>
            <button className="btn" onClick={() => api.saveCareer().then(() => undefined)}>
              Save
            </button>
            <button className="btn btn--ghost" onClick={() => { setCareer(null); setScreen("setup"); }}>
              Main Menu
            </button>
            <button className="btn btn--continue">
              Continue
              <span className="btn--continue__arrow">➜</span>
            </button>
          </>
        }
      >
        {career.dbVersionWarning && (
          <p className="error" style={{ marginBottom: "0.7rem" }}>
            {career.dbVersionWarning}
          </p>
        )}

        <CareerScreen
          world={world}
          career={career}
          onCareer={setCareer}
          tab={careerTab}
          onTab={setCareerTab}
          searchQuery={query}
          onPlay={(snap) => {
            setSnapshot(snap);
            setScreen("match");
          }}
          onOffSeason={() => setScreen("offseason")}
        />
      </AppShell>
    );
  }

  if (screen === "summary" && career) {
    return (
      <AppShell
        clock={seasonClock(1)}
        clockTitle="Season review"
        title={career.state.player_team_name}
        subtitle={`Season ${career.state.season} final standings`}
        homeFlag={(world.nations.find((n) => n.id === career.state.nation_id)?.short_name ?? "CC")
          .slice(0, 3)
          .toUpperCase()}
        items={[
          { id: "news", label: "Season review", icon: "✉", active: true },
          { id: "records", label: "Records", icon: "▮" },
        ]}
        actionBar={
          <>
            <button className="btn" onClick={() => api.saveCareer().then(() => undefined)}>
              Save
            </button>
            <span className="actionbar__message">
              {career.season_complete
                ? "Every fixture played — roll into the off-season when you are ready."
                : "Season still in progress."}
            </span>
            <button
              className="btn btn--continue"
              onClick={() => setScreen("offseason")}
              disabled={!career.season_complete}
            >
              Off-season
              <span className="btn--continue__arrow">➜</span>
            </button>
          </>
        }
      >
        <SeasonSummary
          world={world}
          career={career}
          onOffSeason={() => setScreen("offseason")}
          onSave={() => api.saveCareer().then(() => undefined)}
        />
      </AppShell>
    );
  }

  if (screen === "offseason" && career) {
    return (
      <AppShell
        clock={seasonClock(1)}
        clockTitle="Off-season"
        title={career.state.player_team_name}
        subtitle={`Season ${career.state.season} review`}
        homeFlag={(world.nations.find((n) => n.id === career.state.nation_id)?.short_name ?? "CC")
          .slice(0, 3)
          .toUpperCase()}
        items={[
          { id: "news", label: "Off-season", icon: "✉", active: true },
          { id: "team", label: "Squad", icon: "●" },
        ]}
        actionBar={
          <>
            <button className="btn" onClick={() => api.saveCareer().then(() => undefined)}>
              Save
            </button>
            <span className="actionbar__message">
              Retirees, academy intake and the transfer window are available before rolling
              the season.
            </span>
            <button
              className="btn btn--continue"
              onClick={async () => {
                const next = await api.startNewSeason();
                setCareer(next);
                setScreen("career");
              }}
            >
              Begin season {career.state.season + 1}
              <span className="btn--continue__arrow">➜</span>
            </button>
          </>
        }
      >
        <OffSeasonScreen
          world={world}
          career={career}
          onCareer={setCareer}
          onExit={() => {
            setCareer(null);
            setScreen("setup");
          }}
          onNewSeason={async () => {
            const next = await api.startNewSeason();
            setCareer(next);
            setScreen("career");
          }}
        />
      </AppShell>
    );
  }

  if (screen === "match" && snapshot) {
    const homeTeam = world.teams.find((t) => t.id === snapshot.home_team_id);
    const awayTeam = world.teams.find((t) => t.id === snapshot.away_team_id);
    const homeNation = world.nations.find((n) => n.id === homeTeam?.nation_id);
    const awayNation = world.nations.find((n) => n.id === awayTeam?.nation_id);

    return (
      <AppShell
        clock={`Day ${snapshot.clock.day}`}
        clockTitle={
          snapshot.format === "test"
            ? `Day ${snapshot.clock.day} of 5`
            : seasonClock(career?.state.season_day ?? 1)
        }
        title={`${homeTeam?.name ?? "Home"} vs ${awayTeam?.name ?? "Away"}`}
        subtitle={`${snapshot.format === "odi50" ? "One Day" : snapshot.format === "t20" ? "Twenty20" : "Test"} match · ${
          world.grounds.find((g) => g.id === snapshot.ground_id)?.name ?? ""
        }`}
        homeFlag={(homeNation?.short_name ?? "HM").slice(0, 3).toUpperCase()}
        awayFlag={(awayNation?.short_name ?? "AW").slice(0, 3).toUpperCase()}
        items={[
          { id: "news", label: "Match", icon: "✉", active: true },
          { id: "score", label: "Scorecard", icon: "●" },
          { id: "opts", label: "Options", icon: "✓" },
        ]}
        actionBar={
          <>
            <button className="btn">Main Screen</button>
            <button className="btn btn--ghost">Options</button>
            <button
              className="btn"
              onClick={() => advanceQuick(snapshot, setSnapshot)}
            >
              Autoplay
            </button>
            <span className="actionbar__message">
              {snapshot.result?.summary ??
                (snapshot.innings[snapshot.current_innings]
                  ? `${battingTeamName(world, snapshot)} ${snapshot.innings[snapshot.current_innings].runs}/${
                      snapshot.innings[snapshot.current_innings].wickets
                    }`
                  : "Match in progress")}
            </span>
            <button
              className="btn btn--continue"
              onClick={() => {
                api.getCareer().then((next) => {
                  setCareer(next);
                  setScreen(next.season_complete ? "summary" : "career");
                });
              }}
            >
              Continue
              <span className="btn--continue__arrow">➜</span>
            </button>
          </>
        }
      >
        <MatchScreen
          world={world}
          snapshot={snapshot}
          onSnapshot={setSnapshot}
          onExit={() => {
            api.getCareer().then(setCareer);
            setScreen("career");
          }}
        />
      </AppShell>
    );
  }

  return (
    <main className="shell">
      <h1>Captain&apos;s Call 2027</h1>
      <p className="muted">Nothing to show.</p>
    </main>
  );
}

function battingTeamName(world: World, snapshot: MatchSnapshot): string {
  const inn = snapshot.innings[snapshot.current_innings];
  return world.teams.find((t) => t.id === inn?.batting_team_id)?.name ?? "";
}

function advanceQuick(
  snapshot: MatchSnapshot,
  setSnapshot: (s: MatchSnapshot) => void,
) {
  if (snapshot.phase === "complete") return;
  api.simMatch(100000).then(setSnapshot).catch(() => undefined);
}
