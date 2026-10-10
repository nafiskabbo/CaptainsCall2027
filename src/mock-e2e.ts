//! Browser-side Tauri IPC shim for the e2e harness.
//!
//! Loaded only by `e2e.html` (dev harness), never by `index.html` (the real
//! app). Implements the one function `@tauri-apps/api/core` actually calls,
//! `window.__TAURI_INTERNALS__.invoke`, and forwards it to the Rust harness.
//!
//! Every invoke is recorded in `window.__ccCalls` so a test can assert on what
//! the UI asked for, not just what came back.

interface InvokeLog {
  cmd: string;
  args: unknown;
  startedAt: number;
  error?: string;
}

const calls: InvokeLog[] = [];

interface HarnessWindow extends Window {
  __TAURI_INTERNALS__?: {
    invoke: (cmd: string, args?: unknown) => Promise<unknown>;
    transformCallback: unknown;
    unregisterCallback: unknown;
    convertFileSrc: (path: string) => string;
  };
  __ccCalls: InvokeLog[];
}

const w = window as unknown as HarnessWindow;
w.__ccCalls = calls;

/** Base URL of the Rust harness, overridable for remote/CI runs. */
const HARNESS =
  (w as unknown as { __CC_HARNESS__?: string }).__CC_HARNESS__ ??
  "http://127.0.0.1:8787";

w.__TAURI_INTERNALS__ = {
  invoke: async (cmd: string, args: unknown = {}): Promise<unknown> => {
    const entry: InvokeLog = { cmd, args, startedAt: Date.now() };
    calls.push(entry);
    let response: Response;
    try {
      response = await fetch(`${HARNESS}/invoke`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ cmd, args }),
      });
    } catch (cause) {
      // Tauri rejects with a string rather than the network error, so mirror
      // that or the UI's `catch` blocks behave differently under test.
      const message = `harness unreachable at ${HARNESS}: ${String(cause)}`;
      entry.error = message;
      throw new Error(message);
    }
    const payload = (await response.json()) as
      | { ok: true; data: unknown }
      | { ok: false; error: string };
    if (payload.ok) {
      return payload.data;
    }
    entry.error = payload.error;
    // Surface the real command error in the console so failures are visible.
    console.error(`[e2e] ${cmd} failed:`, payload.error);
    throw new Error(payload.error);
  },
  transformCallback: () => 0,
  unregisterCallback: () => undefined,
  convertFileSrc: (path: string) => path,
};

// A quiet marker so a test can wait for the shim to be live.
console.info(
  `[e2e] Tauri IPC shim active (${calls.length === 0 ? "0" : "0"} calls) -> ${HARNESS}`,
);
