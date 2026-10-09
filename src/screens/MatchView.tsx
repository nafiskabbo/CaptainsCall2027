import { useEffect, useMemo, useRef } from "react";
import type { MatchSnapshot } from "../types";

interface Props {
  snapshot: MatchSnapshot | null;
}

interface Dot {
  id: string;
  angle: number;
  speed: number;
  dist: number;
}

const COLORS = {
  night: "#0b1016",
  day: "#1f7a3e",
  pitch: "#c4a46e",
  ring: "rgba(255,255,255,0.75)",
  batter: "#f0b429",
  bowler: "#e05263",
  fielder: "#8ab4f8",
};

export default function MatchView({ snapshot }: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const frame = useRef(0);

  // Weather / lighting: day or night from the ground.
  const night = useMemo(() => true, []);

  // Animated ball dots (decorative, deterministic from the seed).
  const dots = useMemo<Dot[]>(() => {
    const seed = snapshot?.seed ?? 1;
    const n = 22;
    const out: Dot[] = [];
    let s = seed >>> 0 || 1;
    const rand = () => {
      s = (s * 1664525 + 1013904223) >>> 0;
      return s / 4294967296;
    };
    for (let i = 0; i < n; i++) {
      out.push({
        id: `d${i}`,
        angle: rand() * Math.PI * 2,
        speed: 0.15 + rand() * 0.5,
        dist: 0.25 + rand() * 0.55,
      });
    }
    return out;
  }, [snapshot?.seed]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    canvas.width = canvas.clientWidth * 2;
    canvas.height = canvas.clientHeight * 2;
    ctx.scale(2, 2);

    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    const cx = w / 2;
    const cy = h / 2;
    const r = Math.min(w, h) / 2 - 8;

    const draw = () => {
      frame.current += 0.45;
      const t = frame.current / 60;

      // Ground.
      ctx.clearRect(0, 0, w, h);
      const grad = ctx.createRadialGradient(cx, cy, r * 0.1, cx, cy, r);
      if (night) {
        grad.addColorStop(0, "#0f1a12");
        grad.addColorStop(1, "#050a07");
      } else {
        grad.addColorStop(0, "#26873f");
        grad.addColorStop(1, "#14522a");
      }
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.ellipse(cx, cy, r, r * 0.82, 0, 0, Math.PI * 2);
      ctx.fill();

      // Boundary rope + 30-yard circle.
      ctx.strokeStyle = COLORS.ring;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.ellipse(cx, cy, r - 6, (r - 6) * 0.82, 0, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([4, 5]);
      ctx.beginPath();
      ctx.ellipse(cx, cy, r * 0.55, (r * 0.55) * 0.82, 0, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);

      // Pitch strip.
      ctx.fillStyle = COLORS.pitch;
      ctx.beginPath();
      ctx.ellipse(cx, cy, r * 0.075, r * 0.3, 0, 0, Math.PI * 2);
      ctx.fill();

      // Creases.
      ctx.strokeStyle = "rgba(255,255,255,0.9)";
      ctx.lineWidth = 1;
      for (const dy of [-0.2, 0.2]) {
        ctx.beginPath();
        ctx.moveTo(cx - r * 0.05, cy + dy * r);
        ctx.lineTo(cx + r * 0.05, cy + dy * r);
        ctx.stroke();
      }

      // Stumps.
      ctx.strokeStyle = "#f4f7fa";
      for (const dy of [-0.205, 0.205]) {
        ctx.beginPath();
        ctx.moveTo(cx - 1.5, cy + dy * r);
        ctx.lineTo(cx - 1.5, cy + dy * r - 5);
        ctx.moveTo(cx + 1.5, cy + dy * r);
        ctx.lineTo(cx + 1.5, cy + dy * r - 5);
        ctx.stroke();
      }

      // Fielders: deterministic orbit dots.
      dots.forEach((d) => {
        const a = d.angle + t * d.speed;
        const rr = r * 0.35 + d.dist * r * 0.5;
        const x = cx + Math.cos(a) * rr * 1.05;
        const y = cy + Math.sin(a) * rr * 0.78;
        ctx.fillStyle = COLORS.fielder;
        ctx.globalAlpha = 0.75;
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.globalAlpha = 1;
      });

      // Keeper close to the stumps.
      ctx.fillStyle = "#f0b429";
      ctx.beginPath();
      ctx.arc(cx + r * 0.09, cy - r * 0.2, 3.5, 0, Math.PI * 2);
      ctx.fill();

      // Bowler + batter at the ends.
      drawPerson(ctx, cx - r * 0.1, cy + r * 0.22, COLORS.bowler, t);
      drawPerson(ctx, cx + r * 0.1, cy - r * 0.22, COLORS.batter, t + 1.5);

      // Ball on a short flight path.
      const phase = (t * 1.6) % 1;
      const bx = cx - r * 0.1 + (cx + r * 0.1 - (cx - r * 0.1)) * phase;
      const by =
        cy +
        r * 0.22 +
        ((cy - r * 0.22) - (cy + r * 0.22)) * phase -
        Math.sin(phase * Math.PI) * 12;
      ctx.fillStyle = "#e05263";
      ctx.beginPath();
      ctx.arc(bx, by, 2.6, 0, Math.PI * 2);
      ctx.fill();

      requestAnimationFrame(draw);
    };

    draw();
    return () => cancelAnimationFrame(frame.current);
  }, [dots, night]);

  return (
    <canvas
      ref={canvasRef}
      className="matchview"
      aria-label="Overhead 2D match view"
    />
  );
}

function drawPerson(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  color: string,
  t: number,
) {
  const bob = Math.sin(t * 6) * 0.6;
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(x, y + bob - 5, 2.6, 0, Math.PI * 2); // head
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(x, y + bob - 2.6);
  ctx.lineTo(x, y + bob + 2);
  ctx.stroke();
}
