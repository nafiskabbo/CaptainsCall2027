import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./tools/api";
import type { AppInfo, CareerView, MatchSnapshot, World } from "./types";
import AppShell, { type SidebarItem } from "./components/AppShell";
import CareerScreen from "./screens/CareerScreen";
import MatchScreen from "./screens/MatchScreen";
import WorldScreen from "./screens/WorldScreen";
import OffSeasonScreen from "./screens/OffSeasonScreen";
import SeasonSummary from "./screens/SeasonSummary";
import { seasonClock, seasonDate } from "./lib/date";
import { Button } from "./ui";

type Screen = "setup" | "career" | "match" | "summary" | "offseason";

/** Autoplay cadence: one ball per tick, matching a watchable pace. */
const AUTOPLAY_TICK_MS = 260;

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [world, setWorld] = useState<World | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [screen, setScreen] = useState<Screen>("setup");
  const [career, setCareer] = useState<CareerView | null>(null);
  const [snapshot, setSnapshot] = useState<MatchSnapshot | null>(null);
  /** Sidebar-driven tab + squad search, shared with CareerScreen. */
  const [careerTab, setCareerTab] = useState("today");
  const [query, setQuery] = useState("");
  /** Action-bar confirmation ("Career saved"), auto-clears. */
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  /** Match-screen sidebar selection, mirroring the career tab switch. */
  const [matchTab, setMatchTab] = useState<"batting" | "options">("options");
  /** Autoplay: advances one ball per tick while the match is running. */
  const [autoplay, setAutoplay] = useState(false);
  const autoplayRef = useRef<number | null>(null);

  useEffect(() => {
    Promise.all([api.getAppInfo(), api.getWorld()])
      .then(([appInfo, loadedWorld]) => {
        setInfo(appInfo);
        setWorld(loadedWorld);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const startAutoplay = useCallback(() => {
    setAutoplay(true);
  }, []);

  const stopAutoplay = useCallback(() => {
    setAutoplay(false);
  }, []);

  /** Leave a match, refreshing the career so results and news are current. */
  const exitMatch = useCallback(
    (to: Screen) => {
      stopAutoplay();
      api
        .getCareer()
        .then((next) => {
          setCareer(next);
          setScreen(to);
        })
        .catch(() => setScreen(to));
    },
    [stopAutoplay],
  );

  /** Confirmation for the summary screen's Save (no action bar message there). */
  const showSaveMessageFromSummary = useCallback((message: string) => {
    setSaveMessage(message);
    window.setTimeout(() => setSaveMessage(null), 4000);
  }, []);

  // Autoplay loop: one ball per tick, stops itself when the match ends.
  useEffect(() => {
    if (!autoplay) {
      if (autoplayRef.current !== null) {
        window.clearInterval(autoplayRef.current);
        autoplayRef.current = null;
      }
      return;
    }
    autoplayRef.current = window.setInterval(() => {
      setSnapshot((current) => {
        if (!current || current.phase === "complete") {
          stopAutoplay();
          return current;
        }
        void api.simMatch(1).then(setSnapshot);
        return current;
      });
    }, AUTOPLAY_TICK_MS);
    return () => {
      if (autoplayRef.current !== null) {
        window.clearInterval(autoplayRef.current);
        autoplayRef.current = null;
      }
    };
  }, [autoplay, stopAutoplay, setSnapshot]);

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

  const startCareer = async (
    teamId: number,
    name: string,
    seed: number,
    firstSeason = 1,
    resetForm = false,
  ) => {
    const view = await api.startCareer({ teamId, name, seed, firstSeason, resetForm });
    setCareer(view);
    setCareerTab("today");
    setQuery("");
    setSaveMessage(null);
    setScreen("career");
  };

  /** Transient confirmation shown in the career action bar. */
  const showSaveMessage = (message: string) => {
    setSaveMessage(message);
    window.setTimeout(() => setSaveMessage(null), 4000);
  };

  const quitToMenu = () => {
    setCareer(null);
    setScreen("setup");
  };

  /**
   * Continue: jump to the player's next fixture and open it if it is on today
   * or already past due. This is the button a player presses to keep playing,
   * so it must never dead-end.
   */
  const continueCareer = async () => {
    if (!career) return;
    if (career.seasonComplete) {
      setScreen("summary");
      return;
    }
    try {
      const next = career.state.fixtures.find((f) => f.id === career.nextFixtureId);
      if (next && next.day <= career.state.season_day) {
        setSnapshot(await api.startFixture(next.id));
        setScreen("match");
        return;
      }
      const advanced = await api.advanceToMyMatch();
      setCareer(advanced);
      const nowPlayable = advanced.state.fixtures.find(
        (f) => f.id === advanced.nextFixtureId,
      );
      if (nowPlayable && nowPlayable.day <= advanced.state.season_day) {
        setSnapshot(await api.startFixture(nowPlayable.id));
        setScreen("match");
        return;
      }
      showSaveMessage("No match left to play — season complete.");
    } catch (e) {
      showSaveMessage(`Cannot continue: ${e}`);
    }
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
    const activeTab = careerTab === "fixtures" ? "today" : careerTab;
    const items: SidebarItem[] = [
      { id: "fixtures", label: "Fixtures", icon: "list", active: activeTab === "today", onClick: () => setCareerTab("today") },
      { id: "table", label: "Table", icon: "chart", onClick: () => setCareerTab("table") },
      { id: "records", label: "Records", icon: "chart", onClick: () => setCareerTab("records") },
      { id: "squad", label: "Squad", icon: "team", onClick: () => setCareerTab("squad") },
      { id: "news", label: "News", icon: "news", onClick: () => setCareerTab("news") },
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
            {/* «« steps a single day, Fast Forward skips to the next match day,
                »» skips to the player's own next match. */}
            <button
              className="btn"
              title="Advance one day"
              aria-label="Advance one day"
              onClick={() => api.advanceOneDay().then(setCareer)}
              disabled={career.seasonComplete}
            >
              ««
            </button>
            <button
              className="btn"
              title="Fast forward to the next match day"
              onClick={() => api.advanceDay().then(setCareer)}
              disabled={career.seasonComplete}
            >
              Fast Forward
            </button>
            <button
              className="btn"
              title="Skip to my next match"
              aria-label="Skip to my next match"
              onClick={() => api.advanceToMyMatch().then(setCareer)}
              disabled={career.seasonComplete}
            >
              »»
            </button>
            <span className="actionbar__message">
              {saveMessage
                ? saveMessage
                : career.seasonComplete
                  ? "Season complete"
                  : `${world.competitions.find((c) => c.id === career.state.competition_id)?.name ?? "Competition"} · next match day ${
                      career.state.fixtures.find((f) => f.id === career.nextFixtureId)?.day ?? "—"
                    }`}
            </span>
            <button
              className="btn"
              title="Save career"
              onClick={() =>
                api
                  .saveCareer()
                  .then(() => showSaveMessage("Career saved"))
                  .catch((e) => showSaveMessage(`Could not save: ${e}`))
              }
            >
              Save
            </button>
            <Button variant="ghost" onClick={quitToMenu}>Main Menu</Button>
            <button
              className="btn btn--continue"
              title="Play my next match"
              onClick={continueCareer}
            >
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
          { id: "news", label: "Season review", icon: "news", active: true },
          {
            id: "records",
            label: "Records",
            icon: "chart",
            onClick: () => document.getElementById("records")?.scrollIntoView({ behavior: "smooth" }),
          },
        ]}
        actionBar={
          <>
            <button className="btn" onClick={() => api.saveCareer().then(() => undefined)}>
              Save
            </button>
            <span className="actionbar__message">
              {career.seasonComplete
                ? "Every fixture played — roll into the off-season when you are ready."
                : "Season still in progress."}
            </span>
            <button
              className="btn btn--continue"
              onClick={() => setScreen("offseason")}
              disabled={!career.seasonComplete}
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
          onSave={() => showSaveMessageFromSummary("Career saved")}
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
          { id: "news", label: "Off-season", icon: "news", active: true },
          {
            id: "team",
            label: "Squad",
            icon: "team",
            onClick: () => document.getElementById("offseason-squad")?.scrollIntoView({ behavior: "smooth" }),
          },
        ]}
        actionBar={
          <>
            <button
              className="btn"
              title="Save career"
              onClick={() =>
                api
                  .saveCareer()
                  .then(() => showSaveMessage("Career saved"))
                  .catch((e) => showSaveMessage(`Could not save: ${e}`))
              }
            >
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
          {
            id: "news",
            label: "Match",
            icon: "news",
            active: true,
          },
          {
            id: "score",
            label: "Scorecard",
            icon: "team",
            onClick: () =>
              document.getElementById("match-scorecard")?.scrollIntoView({ behavior: "smooth" }),
          },
          {
            id: "opts",
            label: "Options",
            icon: "options",
            active: matchTab === "options",
            onClick: () => {
              setMatchTab("options");
              document.getElementById("match-tactics")?.scrollIntoView({ behavior: "smooth" });
            },
          },
        ]}
        actionBar={
          <>
            <button
              className="btn"
              title="Back to the career screen"
              onClick={() => exitMatch("career")}
            >
              Main Screen
            </button>
            <button
              className="btn btn--ghost"
              title="Tactics and match conditions"
              onClick={() => {
                setMatchTab("options");
                document.getElementById("match-tactics")?.scrollIntoView({ behavior: "smooth" });
              }}
            >
              Options
            </button>
            <button
              className="btn"
              title={autoplay ? "Stop autoplay" : "Sim the match to the finish"}
              onClick={() => (autoplay ? stopAutoplay() : startAutoplay())}
            >
              {autoplay ? "Pause" : "Autoplay"}
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
              title="Finish and return to the career screen"
              onClick={() =>
                api
                  .getCareer()
                  .then((next) => exitMatch(next.seasonComplete ? "summary" : "career"))
                  .catch(() => exitMatch("career"))
              }
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
          onExit={() => exitMatch("career")}
          optionsOpen={matchTab === "options"}
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
