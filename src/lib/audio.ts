//! Match sound effects, synthesised at runtime with the Web Audio API.
//!
//! The original ships recorded commentary and crowd audio. We have no rights
//! to those recordings, and no binary assets at all, so every cue here is
//! *generated*: a short noise burst shaped by filters and envelopes stands in
//! for a bat on leather, a stumping clatter, a crowd swell and so on. That
//! keeps the build asset-free, offline-capable and instant to load, and the
//! cues can be re-tuned without shipping new files.
//!
//! Browsers refuse to start an AudioContext until the user has interacted with
//! the page, so the context is created lazily on the first real gesture and
//! the first cue may be dropped — normal, and handled silently.

export type SoundCue =
  | "dot"
  | "single"
  | "boundary"
  | "six"
  | "wicket"
  | "caught"
  | "overEnd"
  | "inningsEnd"
  | "matchEnd"
  | "wicketKeeper"
  | "uiClick"
  | "uiBack";

const STORAGE_KEY = "cc2027.audio";

interface AudioSettings {
  enabled: boolean;
  /** 0..1 master volume. */
  volume: number;
}

const DEFAULTS: AudioSettings = { enabled: true, volume: 0.55 };

/** Read persisted settings, tolerating absent/corrupt storage. */
function loadSettings(): AudioSettings {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw) as Partial<AudioSettings>;
    return {
      enabled: typeof parsed.enabled === "boolean" ? parsed.enabled : DEFAULTS.enabled,
      volume:
        typeof parsed.volume === "number" && parsed.volume >= 0 && parsed.volume <= 1
          ? parsed.volume
          : DEFAULTS.volume,
    };
  } catch {
    return { ...DEFAULTS };
  }
}

/** Noise buffer reused by every percussive cue, built once per context. */
let noiseBuffer: AudioBuffer | null = null;

class SoundEngine {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private settings = loadSettings();
  private listeners = new Set<(s: AudioSettings) => void>();

  getSettings(): AudioSettings {
    return { ...this.settings };
  }

  setEnabled(enabled: boolean): void {
    this.settings.enabled = enabled;
    this.persist();
    this.notify();
  }

  setVolume(volume: number): void {
    this.settings.volume = Math.max(0, Math.min(1, volume));
    this.persist();
    this.notify();
  }

  subscribe(fn: (s: AudioSettings) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private notify(): void {
    for (const fn of this.listeners) fn(this.getSettings());
  }

  private persist(): void {
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
    } catch {
      // Storage can be unavailable (private mode); sound still works.
    }
  }

  /**
   * Create the context on demand.
   *
   * Must be called from a user gesture the first time or the browser will
   * leave the context suspended.
   */
  private ensureContext(): AudioContext | null {
    if (this.ctx) {
      if (this.ctx.state === "suspended") void this.ctx.resume();
      return this.ctx;
    }
    const Ctor =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext;
    if (!Ctor) return null;
    try {
      this.ctx = new Ctor();
    } catch {
      return null;
    }
    this.master = this.ctx.createGain();
    this.master.gain.value = this.settings.volume;
    this.master.connect(this.ctx.destination);

    // 1s of white noise, reused for every impact/whoosh cue.
    const len = Math.floor(this.ctx.sampleRate);
    noiseBuffer = this.ctx.createBuffer(1, len, this.ctx.sampleRate);
    const data = noiseBuffer.getChannelData(0);
    for (let i = 0; i < len; i++) data[i] = Math.random() * 2 - 1;
    return this.ctx;
  }

  /**
   * Resume audio after a user gesture.
   *
   * Browsers hold the context suspended until the user interacts, so this
   * registers one-shot listeners for the first interaction. Every step is
   * guarded: some embedded webviews — and jsdom running under Bun — throw on
   * `addEventListener`, and audio support must never be able to take the game
   * down.
   */
  unlock(): boolean {
    try {
      if (typeof window === "undefined" || !window) return false;
      const target = window as unknown as EventTarget;
      const opts = { once: true } as AddEventListenerOptions;
      const resume = () => {
        try {
          this.ensureContext();
        } catch {
          // Never let an audio failure surface to the player.
        }
      };
      target.addEventListener?.("pointerdown", resume, opts);
      target.addEventListener?.("keydown", resume, opts);
      target.addEventListener?.("touchstart", resume, opts);
      return true;
    } catch {
      // Host refuses listeners: audio still works once something calls play().
      return false;
    }
  }

  /**
   * A filtered noise burst: the backbone of every impact cue.
   *
   * @param duration seconds
   * @param freq    band-pass centre — low is a thud, high is a crack
   * @param q       resonance; higher is more metallic
   * @param gain    peak level
   * @param type    filter shape
   */
  private noise(
    duration: number,
    freq: number,
    q: number,
    gain: number,
    type: BiquadFilterType = "bandpass",
  ): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master || !noiseBuffer) return;

    const src = ctx.createBufferSource();
    src.buffer = noiseBuffer;
    src.loop = true;

    const filter = ctx.createBiquadFilter();
    filter.type = type;
    filter.frequency.value = freq;
    filter.Q.value = q;

    const env = ctx.createGain();
    const now = ctx.currentTime;
    env.gain.setValueAtTime(0, now);
    env.gain.linearRampToValueAtTime(gain, now + 0.004);
    env.gain.exponentialRampToValueAtTime(0.0001, now + duration);

    src.connect(filter).connect(env).connect(master);
    src.start(now);
    src.stop(now + duration + 0.02);
  }

  /** A pitched tone with an exponential decay — bells, calls, stumps. */
  private tone(
    freq: number,
    duration: number,
    gain: number,
    type: OscillatorType = "sine",
    slideTo?: number,
  ): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master) return;

    const osc = ctx.createOscillator();
    osc.type = type;
    const env = ctx.createGain();
    const now = ctx.currentTime;
    osc.frequency.setValueAtTime(freq, now);
    if (slideTo) osc.frequency.exponentialRampToValueAtTime(slideTo, now + duration);

    env.gain.setValueAtTime(0, now);
    env.gain.linearRampToValueAtTime(gain, now + 0.008);
    env.gain.exponentialRampToValueAtTime(0.0001, now + duration);

    osc.connect(env).connect(master);
    osc.start(now);
    osc.stop(now + duration + 0.02);
  }

  /** A swelling crowd bed: filtered noise with a slow amplitude ramp. */
  private crowd(peak: number, seconds: number, bright: number): void {
    const ctx = this.ctx;
    const master = this.master;
    if (!ctx || !master || !noiseBuffer) return;

    const src = ctx.createBufferSource();
    src.buffer = noiseBuffer;
    src.loop = true;

    // Heavy low-pass: a distant crowd is mostly rumble, not hiss.
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = bright;
    filter.Q.value = 0.7;

    const env = ctx.createGain();
    const now = ctx.currentTime;
    env.gain.setValueAtTime(0.0001, now);
    env.gain.exponentialRampToValueAtTime(peak, now + seconds * 0.35);
    env.gain.exponentialRampToValueAtTime(0.0001, now + seconds);

    src.connect(filter).connect(env).connect(master);
    src.start(now);
    src.stop(now + seconds + 0.05);
  }

  /**
   * Play a cue.
   *
   * Safe to call before the context exists and while sound is off — it simply
   * does nothing, so call sites never need to guard.
   */
  play(cue: SoundCue): void {
    if (!this.settings.enabled) return;
    const ctx = this.ensureContext();
    if (!ctx) return;

    switch (cue) {
      // Bat meets ball: a tight mid crack.
      case "dot":
        this.noise(0.09, 1500, 3.5, 0.35);
        break;
      // Scattered single: softer, with a little bat-and-pad.
      case "single":
        this.noise(0.12, 900, 2.0, 0.3);
        break;
      // Four: a louder crack plus a crowd lift.
      case "boundary":
        this.noise(0.16, 2000, 4.0, 0.55);
        this.crowd(0.09, 1.1, 900);
        break;
      // Six: a big clean strike and a real roar.
      case "six":
        this.noise(0.2, 2600, 5.0, 0.75);
        this.crowd(0.2, 1.9, 1200);
        break;
      // Wicket: the crack, then timber.
      case "wicket":
        this.noise(0.14, 1700, 3.5, 0.6);
        this.tone(180, 0.28, 0.28, "triangle", 90);
        this.crowd(0.16, 2.0, 1000);
        break;
      // Caught: a softer catch, then appeal.
      case "caught":
        this.noise(0.1, 1200, 2.2, 0.4);
        this.crowd(0.14, 1.6, 850);
        break;
      // Over complete: two short claps.
      case "overEnd":
        this.noise(0.07, 1100, 1.6, 0.22);
        break;
      // Innings break: a longer swell.
      case "inningsEnd":
        this.crowd(0.14, 2.4, 800);
        break;
      // Match over: the biggest cheer.
      case "matchEnd":
        this.crowd(0.28, 3.6, 1400);
        break;
      // Keeper taking the ball: a soft glove pat.
      case "wicketKeeper":
        this.noise(0.07, 700, 1.4, 0.2);
        break;
      // Interface feedback.
      case "uiClick":
        this.tone(880, 0.05, 0.16, "sine");
        break;
      case "uiBack":
        this.tone(520, 0.06, 0.14, "sine");
        break;
    }
  }
}

export const sound = new SoundEngine();

/**
 * Pick the cue for a completed delivery.
 *
 * Derived from the engine's `BallOutcome` so the sound always matches what
 * actually happened on the field — a wicket always sounds like a wicket,
 * whatever the commentary line says.
 */
export function cueForBall(ball: {
  runs: number;
  is_boundary_four: boolean;
  is_boundary_six: boolean;
  dismissal: unknown | null;
}): SoundCue {
  if (ball.dismissal) {
    const type = (ball.dismissal as { out_type?: string }).out_type;
    return type === "caught" || type === "caught_and_bowled" ? "caught" : "wicket";
  }
  if (ball.is_boundary_six) return "six";
  if (ball.is_boundary_four) return "boundary";
  if (ball.runs > 0) return "single";
  return "dot";
}