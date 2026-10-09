import { useEffect, useState } from "react";
import { api } from "./tools/api";
import type { AppInfo, CareerView, MatchSnapshot, World } from "./types";
import CareerScreen from "./screens/CareerScreen";
import MatchScreen from "./screens/MatchScreen";
import WorldScreen from "./screens/WorldScreen";

type Screen = "setup" | "career" | "match";

export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [world, setWorld] = useState<World | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [screen, setScreen] = useState<Screen>("setup");
  const [career, setCareer] = useState<CareerView | null>(null);
  const [snapshot, setSnapshot] = useState<MatchSnapshot | null>(null);

  useEffect(() => {
    Promise.all([api.getAppInfo(), api.getWorld()])
      .then(([appInfo, loadedWorld]) => {
        setInfo(appInfo);
        setWorld(loadedWorld);
      })
      .catch((e) => setError(String(e)));
  }, []);

  if (error) {
    return (
      <main className="shell">
        <h1>Captain&apos;s Call 2027</h1>
        <p className="error">
          Failed to reach the Rust core: {error}
          <br />
          Run with <code>npm run tauri:dev</code> to load the engine.
        </p>
      </main>
    );
  }

  if (!info || !world) {
    return (
      <main className="shell">
        <h1>Captain&apos;s Call 2027</h1>
        <p className="muted">Loading world database…</p>
      </main>
    );
  }

  const startCareer = async (teamId: number, name: string, seed: number) => {
    const view = await api.startCareer({ teamId, name, seed });
    setCareer(view);
    setScreen("career");
  };

  return (
    <div className="app">
      <header className="topbar">
        <div>
          <h1>
            Captain&apos;s Call <span>2027</span>
          </h1>
          <p className="topbar__sub">
            2D cricket management sim · {info.phase} · db {world.db_version} · tribute to Cricket
            Coach 2014
          </p>
        </div>
        <div className="topbar__right">
          <span className="pill">{world.players.length} players</span>
          <span className="pill">{world.teams.length} teams</span>
          <span className="pill">{world.nations.length} nations</span>
          {career && <span className="pill">Day {career.state.season_day}</span>}
        </div>
      </header>

      <main className="content">
        {screen === "setup" && (
          <WorldScreen world={world} onStartCareer={startCareer} />
        )}

        {screen === "career" && career && (
          <CareerScreen
            world={world}
            career={career}
            onCareer={setCareer}
            onPlay={(snap) => {
              setSnapshot(snap);
              setScreen("match");
            }}
            onExit={() => {
              setCareer(null);
              setScreen("setup");
            }}
          />
        )}

        {screen === "match" && snapshot && (
          <MatchScreen
            world={world}
            snapshot={snapshot}
            onSnapshot={setSnapshot}
            onExit={() => {
              setSnapshot(null);
              api.getCareer().then(setCareer);
              setScreen("career");
            }}
          />
        )}
      </main>

      <footer className="footer">
        <span>
          Fictional players, clubs and leagues — real-name packs are community mods only.
        </span>
      </footer>
    </div>
  );
}
