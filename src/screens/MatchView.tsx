//! Overhead 2D match view — the overhead ground, fielders and the ball.
//!
//! Modelled on Cricket Coach 2014's overhead view, which is deliberately plain:
//! a flat green field, a tan pitch, small dots for players with surnames
//! beside them, a photographic crowd band beyond the rope and a scorebar. No
//! perspective, no figures.
//!
//! The animation is driven by *what actually happened on the ball*. The engine
//! reports a bearing and a shot type for every delivery (`BallOutcome`), and
//! this view plays that delivery: run-up, release, the ball travelling to the
//! pitch, then out to wherever it was hit, with fielders moving to intercept.
//! Nothing loops on a fixed timer — which is why the previous version looked
//! identical on every ball, because it had nothing to animate.

import { useEffect, useMemo, useRef } from "react";
import type { BallOutcome, MatchSnapshot } from "../types";

interface Props {
  snapshot: MatchSnapshot | null;
  /** Fielder names keyed by player id (from the bowling side's XI). */
  fielderNames: Record<number, string>;
  /** Batting side's names, for the two batters at the crease. */
  batterNames: Record<number, string>;
  /** Show floodlights for day/night cricket. */
  dayNight?: boolean;
}

/** Where a fielder stands before the ball is hit. */
interface FieldSlot {
  angle: number;
  orbit: number;
  /** Seconds to reach the ball once it is struck. */
  pace: number;
}

/** The nine ring positions plus the keeper, in the original's layout. */
const FIELD_SLOTS: FieldSlot[] = [
  { angle: -1.62, orbit: 0.30, pace: 0.55 }, // slip
  { angle: -1.57, orbit: 0.20, pace: 0.5 }, // keeper
  { angle: -1.1, orbit: 0.52, pace: 0.6 }, // point
  { angle: -0.35, orbit: 0.46, pace: 0.55 }, // cover
  { angle: 0.0, orbit: 0.62, pace: 0.65 }, // mid-off
  { angle: 0.42, orbit: 0.6, pace: 0.65 }, // mid-on
  { angle: 0.85, orbit: 0.5, pace: 0.6 }, // midwicket
  { angle: 1.45, orbit: 0.45, pace: 0.55 }, // square leg
  { angle: 2.5, orbit: 0.72, pace: 0.7 }, // fine leg
];

/** Phases of one delivery, in seconds. */
const RUN_UP = 0.55;
const TO_PITCH = 0.45;
const AFTER_HIT = 0.95;
const SETTLE = 0.5;
const TOTAL = RUN_UP + TO_PITCH + AFTER_HIT + SETTLE;

/** How far along its path the ball is, and how high it flies. */
interface BallState {
  x: number;
  y: number;
  lift: number;
}

export default function MatchView({
  snapshot,
  fielderNames,
  batterNames,
  dayNight = true,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const raf = useRef(0);
  const clock = useRef(0);

  const innings = snapshot?.innings[snapshot.current_innings] ?? null;
  const seed = snapshot?.seed ?? 1;
  const over = innings ? Math.floor(innings.balls / 6) + 1 : 1;

  /*
    The newest delivery is the one being animated. `recent_balls` is newest-first
    and capped, so the head of the list is the ball just bowled; the animation
    restarts whenever its identity changes.
  */
  const latest = snapshot?.recent_balls?.[0] ?? null;
  const latestKey = latest ? JSON.stringify([latest.runs, latest.direction_deg, latest.commentary]) : "";

  // Deterministic field positions, nudged per match so two sides do not look
  // identical, but stable within one match.
  const slots = useMemo<FieldSlot[]>(() => {
    let s = ((seed ^ 0x51ed) >>> 0) || 1;
    const rand = () => {
      s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
      return s / 4294967296;
    };
    return FIELD_SLOTS.map((slot) => ({
      angle: slot.angle + (rand() - 0.5) * 0.14,
      orbit: slot.orbit,
      pace: slot.pace,
    }));
  }, [seed]);

  // Identity of the delivery currently being animated, and its clock.
  const anim = useRef<{ key: string; t: number; outcome: BallOutcome | null }>({
    key: "",
    t: TOTAL,
    outcome: null,
  });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = canvas.clientWidth * dpr;
      canvas.height = canvas.clientHeight * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    // ResizeObserver is not universal (older WebViews, jsdom-based tests), and
    // without it the whole match screen would unmount.
    let observer: ResizeObserver | null = null;
    let onWindowResize = false;
    try {
      if (typeof ResizeObserver === "function") {
        observer = new ResizeObserver(resize);
        observer.observe(canvas);
      } else {
        window.addEventListener("resize", resize);
        onWindowResize = true;
      }
    } catch {
      observer = null;
      onWindowResize = false;
    }

    /** Where a bearing (degrees off the striker) points on the ground. */
    const bearingVec = (deg: number) => {
      // 0 = straight back past the bowler (up the screen), growing clockwise.
      const rad = ((deg - 90) * Math.PI) / 180;
      return { x: Math.cos(rad), y: Math.sin(rad) };
    };

    /** Ball position for time `t` within a delivery. */
    const ballAt = (t: number, outcome: BallOutcome | null, cx: number, cy: number, R: number, pitchH: number) => {
      const striker = { x: cx + 6, y: cy + pitchH * 0.3 };
      const bowler = { x: cx - 6, y: cy - pitchH * 0.3 };

      if (!outcome) {
        // No delivery yet: the bowler just stands at the mark.
        return { x: bowler.x, y: bowler.y, lift: 0 };
      }

      if (t < RUN_UP) {
        // Run-up: the bowler moves in toward the crease.
        const p = t / RUN_UP;
        return { x: bowler.x - p * R * 0.18, y: bowler.y - p * pitchH * 0.1, lift: 0 };
      }
      if (t < RUN_UP + TO_PITCH) {
        // In flight to the batter.
        const p = (t - RUN_UP) / TO_PITCH;
        return {
          x: bowler.x + (striker.x - bowler.x) * p,
          y: bowler.y + (striker.y - bowler.y) * p,
          lift: Math.sin(p * Math.PI) * R * 0.16,
        };
      }

      // Struck. Where it goes depends on the real outcome.
      const after = t - (RUN_UP + TO_PITCH);
      const p = Math.min(1, after / AFTER_HIT);
      const v = bearingVec(outcome.direction_deg);
      const runs = outcome.runs + outcome.extra_runs;
      const distance =
        outcome.is_boundary_six ? R * 0.98 : outcome.is_boundary_four ? R * 0.9 : R * (0.2 + runs * 0.08);
      const loft = outcome.shot === "lofted";
      const edged = outcome.shot === "edged";
      return {
        x: striker.x + v.x * distance * p,
        y: striker.y + v.y * distance * p,
        // Lofted shots arc high; edged ones dribble away low.
        lift: loft
          ? Math.sin(p * Math.PI) * R * 0.42
          : edged
            ? Math.sin(p * Math.PI) * R * 0.03
            : 0,
      };
    };

    const draw = () => {
      const dt = 1 / 60;
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;
      const cx = w / 2;
      const cy = h / 2;
      const R = Math.min(w, h) * 0.46;

      // ---- advance the delivery clock ----
      const a = anim.current;
      if (latestKey && a.key !== latestKey) {
        // A new delivery: start it from the run-up.
        a.key = latestKey;
        a.t = 0;
        a.outcome = latest;
      } else if (a.t < TOTAL) {
        a.t += dt;
      }

      ctx.clearRect(0, 0, w, h);

      // ---- field ----
      const field = ctx.createRadialGradient(cx, cy, R * 0.1, cx, cy, R);
      if (dayNight) {
        field.addColorStop(0, "#2d7a3a");
        field.addColorStop(0.75, "#22622c");
        field.addColorStop(1, "#1a4d22");
      } else {
        field.addColorStop(0, "#3d9a4a");
        field.addColorStop(0.75, "#2c7538");
        field.addColorStop(1, "#215c2b");
      }
      ctx.fillStyle = field;
      ctx.beginPath();
      ctx.ellipse(cx, cy, R, R * 0.9, 0, 0, Math.PI * 2);
      ctx.fill();

      // mown arcs
      ctx.save();
      ctx.beginPath();
      ctx.ellipse(cx, cy, R, R * 0.9, 0, 0, Math.PI * 2);
      ctx.clip();
      for (let i = -6; i <= 6; i++) {
        if (i % 2 !== 0) continue;
        ctx.fillStyle = "rgba(255,255,255,0.025)";
        ctx.fillRect(cx - R + (i / 6) * R, cy - R, R / 12, R * 2);
      }
      ctx.restore();

      // ---- rope and crowd beyond it ----
      ctx.strokeStyle = "rgba(255,255,255,0.55)";
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.ellipse(cx, cy, R * 0.95, R * 0.855, 0, 0, Math.PI * 2);
      ctx.stroke();

      // Crowd: a speckled band outside the rope, as in the original's photo
      // strip. Drawn only along the arc that is visible in the viewport.
      ctx.save();
      ctx.beginPath();
      ctx.ellipse(cx, cy, R * 0.97, R * 0.875, 0, 0, Math.PI * 2);
      ctx.arc(cx, cy, R * 1.16, 0, Math.PI * 2, true);
      ctx.fillStyle = dayNight ? "#1d3a1e" : "#2b5c30";
      ctx.fill("evenodd");
      ctx.clip("evenodd");
      let s = 987654321;
      const rnd = () => {
        s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
        return s / 4294967296;
      };
      for (let i = 0; i < 900; i++) {
        const ang = rnd() * Math.PI * 2;
        const rr = R * (0.98 + rnd() * 0.16);
        const x = cx + Math.cos(ang) * rr;
        const y = cy + Math.sin(ang) * rr * 0.9;
        const shade = 90 + rnd() * 110;
        ctx.fillStyle = `rgba(${shade},${shade + 20},${shade - 10},0.55)`;
        ctx.fillRect(x, y, 1.6, 1.6);
      }
      ctx.restore();

      // ---- pitch ----
      const pitchH = R * 0.34;
      const pitchW = R * 0.075;
      ctx.fillStyle = "#d9c39a";
      ctx.fillRect(cx - pitchW, cy - pitchH, pitchW * 2, pitchH * 2);
      ctx.fillStyle = "rgba(160,140,100,0.35)";
      ctx.fillRect(cx - pitchW * 0.62, cy - pitchH * 0.9, pitchW * 1.24, pitchH * 1.8);
      // creases
      ctx.strokeStyle = "rgba(255,255,255,0.85)";
      ctx.lineWidth = 1;
      for (const dy of [-pitchH * 0.96, pitchH * 0.96]) {
        ctx.beginPath();
        ctx.moveTo(cx - pitchW * 0.75, cy + dy);
        ctx.lineTo(cx + pitchW * 0.75, cy + dy);
        ctx.stroke();
      }

      const pos = ballAt(a.t, a.outcome, cx, cy, R, pitchH);

      // ---- fielders: dots with surnames, as the original draws them ----
      // Each fielder chases the ball once it is struck; before that they hold
      // their station, so the field visibly changes shape during a shot.
      const struck = a.t > RUN_UP + TO_PITCH && !!a.outcome;
      ctx.font = "600 10px 'Segoe UI', Tahoma, sans-serif";
      ctx.textAlign = "left";
      ctx.textBaseline = "middle";
      slots.forEach((slot, i) => {
        let fx = cx + Math.cos(slot.angle) * slot.orbit * R * 1.02;
        let fy = cy + Math.sin(slot.angle) * slot.orbit * R * 0.9;

        if (struck && a.outcome) {
          // Move toward the ball's line, closing the gap over the flight.
          const p = Math.min(1, (a.t - RUN_UP - TO_PITCH) / AFTER_HIT);
          const v = bearingVec(a.outcome.direction_deg);
          // Only fielders roughly on the ball's line react.
          const dx = Math.cos(slot.angle);
          const dy = Math.sin(slot.angle);
          const align = dx * v.x + dy * v.y;
          if (align > 0.35) {
            const chase = p * align * slot.orbit * R * 0.5;
            fx += v.x * chase;
            fy += v.y * chase * 0.9;
          }
        }

        ctx.fillStyle = "#e9eef4";
        ctx.beginPath();
        ctx.arc(fx, fy, 2.6, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = "rgba(0,0,0,0.5)";
        ctx.lineWidth = 0.6;
        ctx.stroke();

        const name = fielderNames[i];
        if (name) {
          ctx.fillStyle = "#e8e600";
          ctx.fillText(name, fx + 4, fy);
        }
      });

      // ---- umpires ----
      ctx.fillStyle = "#8fd0ff";
      for (const dy of [-pitchH * 1.12, pitchH * 1.12]) {
        ctx.beginPath();
        ctx.arc(cx - pitchW * 1.6, cy + dy, 2.4, 0, Math.PI * 2);
        ctx.fill();
      }

      // ---- batters ----
      const strikerId = snapshot?.striker_id ?? null;
      const nonStrikerId = snapshot?.non_striker_id ?? null;
      ctx.fillStyle = "#e04a56";
      ctx.beginPath();
      ctx.arc(cx + 5, cy + pitchH * 0.96, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#4a90e0";
      ctx.beginPath();
      ctx.arc(cx + 5, cy + pitchH * 0.55, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#e8e600";
      ctx.fillText(batterNames[strikerId ?? 0] ?? "", cx + 11, cy + pitchH * 0.96);
      ctx.fillText(batterNames[nonStrikerId ?? 0] ?? "", cx + 11, cy + pitchH * 0.55);

      // ---- bowler ----
      ctx.fillStyle = "#e04a56";
      ctx.beginPath();
      ctx.arc(pos.x - (a.t < RUN_UP ? R * 0.18 : 0), cy - pitchH * 1.1, 2.8, 0, Math.PI * 2);
      ctx.fill();

      // ---- the ball ----
      // Only draw it once it is in play; a ball sitting at the bowler's end
      // before the delivery is just the mark.
      if (a.outcome && a.t > RUN_UP * 0.8) {
        ctx.fillStyle = "#ffffff";
        ctx.beginPath();
        ctx.arc(pos.x, pos.y - pos.lift, 2.6, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = "rgba(0,0,0,0.6)";
        ctx.lineWidth = 0.7;
        ctx.stroke();
      }

      raf.current = requestAnimationFrame(draw);
    };

    raf.current = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf.current);
      if (observer) {
        observer.disconnect();
      } else if (onWindowResize) {
        window.removeEventListener("resize", resize);
      }
    };
  }, [slots, fielderNames, batterNames, dayNight, latestKey, latest, innings, over, snapshot?.striker_id, snapshot?.non_striker_id]);

  return (
    <div className="groundwrap">
      <canvas
        ref={canvasRef}
        className="groundcanvas"
        aria-label="Overhead 2D match view"
      />
    </div>
  );
}