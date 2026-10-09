//! Database update panel: channel, opt-in check, local manifest inspection.
//!
//! Offline-first by default: nothing is checked unless you turn it on, and the
//! game is fully playable with the bundled database at all times.

import { useEffect, useState } from "react";
import { api } from "../tools/api";

type UpdateStatus =
  | { kind: "UpToDate"; installed: string }
  | {
      kind: "Available";
      version: string;
      changelog: string;
      bytes: number;
      channel: string;
    }
  | { kind: "Unreachable"; installed: string }
  | { kind: "Rejected"; installed: string; reason: string };

interface UpdateState {
  installed: string;
  settings: {
    checkForUpdates: boolean;
    channel: "stable" | "beta" | "local";
    previousDbVersion: string | null;
  };
  status: UpdateStatus;
}

export default function UpdatePanel() {
  const [state, setState] = useState<UpdateState | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [manifestPath, setManifestPath] = useState(
    "data/updates/manifest.2027.02.json",
  );

  useEffect(() => {
    api.getUpdateState().then(setState).catch(() => undefined);
  }, []);

  const toggle = async (enabled: boolean, channel?: string) => {
    setBusy(true);
    try {
      setState(await api.setUpdateSettings(enabled, channel));
      setMessage(
        enabled
          ? `Update checks on (${channel ?? state?.settings.channel ?? "stable"} channel).`
          : "Update checks off — you will stay on the bundled database.",
      );
    } finally {
      setBusy(false);
    }
  };

  const checkLocal = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const next = await api.checkLocalUpdate(manifestPath);
      setState(next);
      setMessage(describe(next.status));
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  };

  if (!state) {
    return (
      <section className="panel">
        <h3 className="panel__title">Database updates</h3>
        <p className="muted">Loading update settings…</p>
      </section>
    );
  }

  return (
    <section className="panel">
      <h3 className="panel__title">Database updates</h3>

      <div className="statgrid">
        <div className="stat">
          <div className="stat__label">Installed db</div>
          <div className="stat__value stat__value--gold">{state.installed}</div>
        </div>
        <div className="stat">
          <div className="stat__label">Channel</div>
          <div className="stat__value">{state.settings.channel}</div>
        </div>
        <div className="stat">
          <div className="stat__label">Checks</div>
          <div className="stat__value">{state.settings.checkForUpdates ? "on" : "off"}</div>
        </div>
      </div>

      <p className="muted">
        The game ships with a bundled fictional database and works fully offline.
        Updates are optional, and only signed manifests are ever installed.
        Real-name packs are community mods and are never bundled by us.
      </p>

      <div className="toggles">
        <label className="radio">
          <input
            type="checkbox"
            checked={state.settings.checkForUpdates}
            onChange={(e) => toggle(e.target.checked)}
            disabled={busy}
          />
          Check for database updates
        </label>
      </div>

      <div className="field" style={{ maxWidth: 220 }}>
        <label htmlFor="channel">Channel</label>
        <select
          id="channel"
          value={state.settings.channel}
          onChange={(e) => toggle(state.settings.checkForUpdates, e.target.value)}
          disabled={busy}
        >
          <option value="stable">Stable (signed)</option>
          <option value="beta">Beta (opt-in)</option>
          <option value="local">Local file</option>
        </select>
      </div>

      <div className="field">
        <label htmlFor="manifest">Local manifest path</label>
        <div style={{ display: "flex", gap: "0.4rem" }}>
          <input
            id="manifest"
            type="text"
            value={manifestPath}
            onChange={(e) => setManifestPath(e.target.value)}
          />
          <button className="btn" onClick={checkLocal} disabled={busy}>
            Check
          </button>
        </div>
      </div>

      {message && <p className="muted">{message}</p>}

      {state.status.kind === "Available" && (
        <div className="panel" style={{ background: "rgba(63, 174, 74, 0.12)" }}>
          <strong>Update available: {state.status.version}</strong>
          <p className="muted">
            Channel {state.status.channel} · {(state.status.bytes / 1024).toFixed(0)} kB
          </p>
          <p>{state.status.changelog}</p>
          <p className="muted">
            Installing backs up your current database first, so you can roll back.
          </p>
        </div>
      )}

      {state.status.kind === "Rejected" && (
        <p className="error">Update refused: {state.status.reason}</p>
      )}
      {state.status.kind === "Unreachable" && (
        <p className="muted">Feed unreachable — continuing on the bundled database.</p>
      )}
    </section>
  );
}

function describe(status: UpdateStatus): string {
  switch (status.kind) {
    case "Available":
      return `Update ${status.version} is available and verified.`;
    case "Rejected":
      return `Refused: ${status.reason}`;
    case "Unreachable":
      return "Could not reach the feed.";
    default:
      return "You are already on the newest database.";
  }
}
