//! The match view — the ground, the crowd, the fielders and the ball.
//!
//! Rebuilt as SVG rather than canvas, for two reasons.
//!
//! The first is fidelity. The original draws a *tilted* stadium: the ground is
//! an ellipse seen from high and behind, compressed towards a horizon, ringed
//! by boundary advertising, with packed seating sweeping across the top of the
//! frame. A flat overhead circle — which is what the canvas version drew, and
//! what `pics/pitch2.bmp` is — is a different picture entirely. Perspective
//! geometry wants real coordinates rather than an immediate-mode raster.
//!
//! The second is that it can be *seen*. The canvas version was invisible to the
//! screenshot harness, which stubs the 2D context, so the field had never
//! actually been looked at. SVG is DOM: it renders in jsdom, it can be asserted
//! on by the e2e, and a regression in it is visible instead of silent.
//!
//! The animation is still driven by *what actually happened on the ball*. The
//! engine reports a bearing and a shot type for every delivery (`BallOutcome`),
//! and this plays that delivery: run-up, release, the ball travelling to the
//! pitch, then out to wherever it was hit, with fielders moving to intercept.

import { useEffect, useMemo, useRef, useState } from "react";
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

/** A point on the ground, in ground space. */
interface Ground {
  /** Degrees, 0 = straight back past the bowler, growing clockwise. */
  bearing: number;
  /** 0 at the middle of the pitch, 1 at the boundary rope. */
  radius: number;
}

/** Where a fielder stands before the ball is hit. */
interface FieldSlot {
  bearing: number;
  radius: number;
  /** Seconds to reach the ball once it is struck. */
  pace: number;
}

/** The ring positions plus the keeper, as the original lays them out. */
const FIELD_SLOTS: FieldSlot[] = [
  { bearing: 168, radius: 0.20, pace: 0.5 }, // keeper
  { bearing: 158, radius: 0.30, pace: 0.55 }, // slip
  { bearing: 148, radius: 0.52, pace: 0.6 }, // point
  { bearing: 135, radius: 0.62, pace: 0.6 }, // cover
  { bearing: 120, radius: 0.70, pace: 0.65 }, // extra cover
  { bearing: 100, radius: 0.66, pace: 0.65 }, // mid-off
  { bearing: 80, radius: 0.58, pace: 0.6 }, // mid-on
  { bearing: 55, radius: 0.50, pace: 0.6 }, // midwicket
  { bearing: 25, radius: 0.55, pace: 0.55 }, // square leg
  { bearing: 5, radius: 0.72, pace: 0.7 }, // fine leg
];

/** Boundary advertising, as the original rings the ground with it. */
const ADS = ["MARLEY", "HEALTHPUMP", "Sam..hupp", "PULSEWAVE", "THE NATION BANK"];

/** Phases of one delivery, in seconds. */
const RUN_UP = 0.55;
const TO_PITCH = 0.45;
const AFTER_HIT = 0.95;
const SETTLE = 0.5;
const TOTAL = RUN_UP + TO_PITCH + AFTER_HIT + SETTLE;

/* --- ground projection ------------------------------------------------- */

/*
  The screen geometry of the ground.

  `HORIZON` is where the ground meets the stands; `CX`/`CY` place the middle of
  the pitch low and left of centre, which is what puts the crowd sweeping across
  the top right the way it does in the original. `FAR` compresses the far side
  of the ground, so the ellipse reads as a tilted plane rather than a circle
  seen face-on — the whole difference between a stadium and a diagram.
*/
const CX = 430;
const CY = 470;
const RX = 700;
const RY = 560;
const FAR = 0.46;
/*
  Shear. The original is not a disc seen square-on: the ground is rotated so the
  far boundary sweeps up and to the right, with the stands wrapping the top and
  right of the frame. Without this the projection produced a clean circle,
  which is a diagram of a cricket ground rather than a view of one.
*/
const SHEAR = 0.30;

/** Project a ground point to the screen. */
function project(g: Ground): { x: number; y: number } {
  const rad = (g.bearing * Math.PI) / 180;
  // 0 points away from the camera (up the screen, past the bowler).
  const gx = Math.sin(rad);
  const gy = -Math.cos(rad);
  // Depth squash: points beyond the middle of the ground compress towards the
  // horizon, points nearer the camera spread out.
  const depth = FAR + (1 - FAR) * ((gy + 1) / 2);
  const x = CX + gx * g.radius * RX;
  const y = CY + gy * g.radius * RY * depth;
  // Shear the far side to the right, which is what tilts the plane.
  return { x: x + (y - CY) * SHEAR, y };
}

/** The boundary rope, as an SVG path across the far side of the ground. */
function boundaryPath(): string {
  const pts: string[] = [];
  for (let deg = 0; deg <= 360; deg += 4) {
    const p = project({ bearing: deg, radius: 1 });
    pts.push(`${p.x.toFixed(1)},${p.y.toFixed(1)}`);
  }
  return `M${pts.join("L")}Z`;
}

/** Ball position for time `t` within one delivery, in ground space. */
function ballAt(t: number, outcome: BallOutcome | null): Ground & { lift: number } {
  const bowlerMark = { bearing: 0, radius: 0.16 };
  const striker = { bearing: 180, radius: 0.055 };
  const crease = { bearing: 180, radius: 0.075 };

  if (!outcome) return { ...bowlerMark, lift: 0 };

  if (t < RUN_UP) {
    const p = t / RUN_UP;
    return { bearing: 0, radius: 0.16 + p * 0.1, lift: 0 };
  }
  if (t < RUN_UP + TO_PITCH) {
    const p = (t - RUN_UP) / TO_PITCH;
    return {
      bearing: 180 * p,
      radius: 0.26 + (crease.radius - 0.26) * p,
      lift: Math.sin(p * Math.PI) * 0.09,
    };
  }
  const p = (t - RUN_UP - TO_PITCH) / AFTER_HIT;
  if (p >= 1) return { ...crease, lift: 0 };

  // Where the ball went, from the engine's own bearing for this delivery.
  const reach = outcome.dismissal
    ? 0.55
    : outcome.is_boundary_six
      ? 1.15
      : outcome.is_boundary_four
        ? 1.02
        : 0.18 + outcome.runs * 0.16;
  const bearing = outcome.direction_deg;
  const eased = p * p * (3 - 2 * p);
  return {
    bearing: striker.bearing + bearing * eased,
    radius: crease.radius + (reach - crease.radius) * eased,
    lift: Math.sin(p * Math.PI) * (outcome.is_boundary_six ? 0.5 : 0.16),
  };
}

/** Fielders closing on the ball once it is struck. */
function fielderTo(slot: FieldSlot, ball: Ground, t: number): { bearing: number; radius: number } {
  if (t < RUN_UP + TO_PITCH) return slot;
  // Whoever is closest to the ball's line goes for it.
  const d = Math.abs(((slot.bearing - ball.bearing + 540) % 360) - 180);
  const close = Math.min(1, (180 - d) / 90);
  const travel = Math.min(slot.pace / AFTER_HIT, t / AFTER_HIT) * close * 0.45;
  return {
    bearing: slot.bearing,
    radius: Math.max(0.1, slot.radius - travel),
  };
}

export default function MatchView({
  snapshot,
  fielderNames,
  batterNames,
  dayNight = true,
}: Props) {
  const seed = snapshot?.seed ?? 1;

  const latest = snapshot?.recent_balls?.[0] ?? null;
  const latestKey = latest
    ? JSON.stringify([latest.runs, latest.direction_deg, latest.commentary])
    : "";

  // Deterministic field positions, nudged per match so two sides do not look
  // identical, but stable within one match.
  const slots = useMemo<FieldSlot[]>(() => {
    let s = ((seed ^ 0x51ed) >>> 0) || 1;
    const rand = () => {
      s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
      return s / 4294967296;
    };
    return FIELD_SLOTS.map((slot) => ({
      bearing: slot.bearing + (rand() - 0.5) * 14,
      radius: slot.radius + (rand() - 0.5) * 0.05,
      pace: slot.pace,
    }));
  }, [seed]);

  const names = useMemo(() => Object.values(fielderNames), [fielderNames]);
  const [t, setT] = useState(TOTAL);
  const anim = useRef({ key: "", t: TOTAL });
  const raf = useRef(0);

  useEffect(() => {
    // A new delivery restarts the clock.
    if (anim.current.key !== latestKey) {
      anim.current = { key: latestKey, t: 0 };
      setT(0);
    }
    let last = performance.now();
    const step = (now: number) => {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      anim.current.t = Math.min(TOTAL, anim.current.t + dt);
      setT(anim.current.t);
      if (anim.current.t < TOTAL) raf.current = requestAnimationFrame(step);
    };
    if (typeof requestAnimationFrame === "function") {
      raf.current = requestAnimationFrame(step);
    }
    return () => {
      if (raf.current) cancelAnimationFrame(raf.current);
    };
  }, [latestKey]);

  const settled = t >= TOTAL;
  const ball = ballAt(t, latest);
  const ballPt = project(ball);
  const boundary = boundaryPath();

  const fielders = slots.map((slot, i) => {
    const at = fielderTo(slot, ball, t);
    const p = project(at);
    const name = names[i] ?? "";
    return { p, name, key: i };
  });

  const strikerPt = project({ bearing: 180, radius: 0.055 });
  const nonStrikerPt = project({ bearing: 180, radius: 0.42 });
  const bowlerPt = project(ballAt(Math.min(t, RUN_UP), null));
  const batterEntries = Object.values(batterNames);

  return (
    <svg
      className="ground"
      viewBox="0 0 1000 640"
      preserveAspectRatio="xMidYMid slice"
      role="img"
      aria-label="Cricket ground with fielders and the ball"
      data-testid="ground"
      data-delivery={settled ? "settled" : "bowling"}
    >
      <defs>
        {/*
          The crowd. The original's stands are a dense speckle of small
          coloured dots on blue, which is what sells it as a full house rather
          than a painted backdrop.
        */}
        <pattern id="crowd" width="6" height="5" patternUnits="userSpaceOnUse">
          <rect width="6" height="5" fill="#6d8ab4" />
          {[
            [1, 1, "#dfe3ea"], [4, 1, "#b8454a"], [2.6, 3, "#e9e4c8"],
            [5, 3.4, "#dfe3ea"], [0.4, 3.8, "#33456b"],
          ].map(([x, y, fill], i) => (
            <circle key={i} cx={x} cy={y} r="0.85" fill={fill as string} />
          ))}
        </pattern>
        {/* Rows of seating, which is what gives the stands their banding. */}
        <pattern id="seats" width="6" height="5" patternUnits="userSpaceOnUse">
          <rect width="6" height="5" fill="url(#crowd)" />
          <rect width="6" height="1.6" fill="#20304f" opacity="0.35" />
        </pattern>
        {/* Mown stripes across the outfield, as the original cuts them. */}
        <pattern id="stripes" width="52" height="52" patternUnits="userSpaceOnUse"
                 patternTransform="rotate(-18)">
          <rect width="52" height="52" fill="#4a7a45" />
          <rect width="26" height="52" fill="#548550" />
        </pattern>
        <radialGradient id="vignette" cx="0.5" cy="0.5" r="0.72">
          <stop offset="60%" stopColor="#000" stopOpacity="0" />
          <stop offset="100%" stopColor="#000" stopOpacity="0.45" />
        </radialGradient>
        <clipPath id="groundClip">
          <path d={boundary} />
        </clipPath>
      </defs>

      {/* Stands: a band across the top, behind the boundary. */}
      <rect x="0" y="0" width="1000" height="640" fill="url(#seats)" />
      {dayNight && (
        <g className="ground__floodlights" opacity="0.5">
          <circle cx="150" cy="60" r="2.5" fill="#fff" />
          <circle cx="850" cy="40" r="2.5" fill="#fff" />
        </g>
      )}

      {/*
        Boundary advertising, then the rope.

        The original rings the ground with sponsor boards between the rope and
        the stands. Drawn as a stroke on the boundary path rather than as a
        separate shape so it follows the perspective for free.
      */}
      <path
        d={boundary}
        fill="none"
        stroke="#1b2740"
        strokeWidth="54"
        opacity="0.97"
      />
      <path d={boundary} fill="none" stroke="#2c3c5e" strokeWidth="2" opacity="0.8" />
      {/*
        Sponsor names along the top of the advertising ring.

        Set horizontally rather than rotated along the boundary. Rotating them
        by the local tangent put one board at a steep angle half off the frame
        and pushed the rest outside it, which read as a rendering fault rather
        than as perspective. The original's boards do follow the curve, but at
        this projection they are not legible enough for that to be worth it.
      */}
      <g className="ground__ads" opacity="0.92">
        {ADS.map((ad, i) => {
          const p = project({ bearing: 258 + i * 15, radius: 1.06 });
          return (
            <text
              key={ad}
              x={p.x}
              y={p.y}
              fill={i % 3 === 0 ? "#e8fd00" : i % 3 === 1 ? "#e04a4a" : "#f0f0f0"}
              fontSize="15"
              fontWeight="700"
              textAnchor="middle"
            >
              {ad}
            </text>
          );
        })}
      </g>

      {/* The outfield. */}
      <path d={boundary} fill="url(#stripes)" />
      <g clipPath="url(#groundClip)">
        {/* The square, cut differently from the rest of the ground. */}
        <ellipse cx={CX} cy={CY - 10} rx={RX * 0.3} ry={RY * 0.5} fill="#5b8f52" opacity="0.75" />
        {/* Thirty-yard circle. */}
        <ellipse
          cx={CX} cy={CY} rx={RX * 0.34} ry={RY * 0.34}
          fill="none" stroke="#ffffff" strokeWidth="1.2" opacity="0.28"
        />
      </g>
      <path d={boundary} fill="none" stroke="#e8e8e8" strokeWidth="2.4" opacity="0.9" />

      {/* The pitch, drawn as a trapezoid because it recedes. */}
      <g className="ground__pitch">
        <path
          d={`M${strikerPt.x - 34} ${strikerPt.y}
              L${strikerPt.x + 34} ${strikerPt.y}
              L${bowlerPt.x + 24} ${bowlerPt.y}
              L${bowlerPt.x - 24} ${bowlerPt.y} Z`}
          fill="#c8b98a"
          stroke="#a8975f"
          strokeWidth="1"
        />
        {/* Creases and stumps. */}
        <line
          x1={strikerPt.x - 30} y1={strikerPt.y} x2={strikerPt.x + 30} y2={strikerPt.y}
          stroke="#f0f0f0" strokeWidth="1.4"
        />
        <line
          x1={bowlerPt.x - 22} y1={bowlerPt.y} x2={bowlerPt.x + 22} y2={bowlerPt.y}
          stroke="#f0f0f0" strokeWidth="1.4"
        />
        <circle cx={bowlerPt.x} cy={bowlerPt.y - 4} r="2" fill="#f5f5f5" />
      </g>

      {/* Fielders, each labelled with their surname as the original does. */}
      <g className="ground__fielders">
        {fielders.map((f) => (
          <g key={f.key} data-testid="fielder">
            <circle cx={f.p.x} cy={f.p.y} r="4.5" fill="#e8fd00" stroke="#20200f" strokeWidth="1" />
            {f.name && (
              <text
                x={f.p.x}
                y={f.p.y - 9}
                className="ground__fname"
                textAnchor="middle"
                fontSize="11"
              >
                {f.name}
              </text>
            )}
          </g>
        ))}
      </g>

      {/* The two batters and the bowler. */}
      <g className="ground__batters">
        <circle cx={strikerPt.x} cy={strikerPt.y} r="4" fill="#ff6b6b" stroke="#3a0d0d" strokeWidth="1" />
        <circle cx={nonStrikerPt.x} cy={nonStrikerPt.y} r="3.4" fill="#ff9d9d" stroke="#3a0d0d" strokeWidth="1" />
        <circle cx={bowlerPt.x} cy={bowlerPt.y} r="3.4" fill="#6bb5ff" stroke="#0d1f3a" strokeWidth="1" />
        {batterEntries[0] && (
          <text x={strikerPt.x} y={strikerPt.y - 9} className="ground__fname" textAnchor="middle" fontSize="11">
            {batterEntries[0]}
          </text>
        )}
      </g>

      {/* The ball. `lift` rises off the ground for a lofted shot. */}
      <circle
        className="ground__ball"
        data-testid="ball"
        cx={ballPt.x}
        cy={ballPt.y - ball.lift * 70}
        r="3.6"
        fill="#fff"
        stroke="#222"
        strokeWidth="0.8"
      />

      <rect x="0" y="0" width="1000" height="640" fill="url(#vignette)" pointerEvents="none" />
    </svg>
  );
}

/** The over number, for the score bar. Kept here so the view stays self-contained. */
export function overNumber(snapshot: MatchSnapshot | null): number {
  const innings = snapshot?.innings[snapshot.current_innings];
  if (!innings) return 1;
  return Math.floor(innings.balls / 6) + 1;
}