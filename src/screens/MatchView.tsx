//! Overhead 2D match view: stadium bowl, boundary boards, named fielders.
//! Mirrors the Cricket Coach 2014 2D view.

import { useEffect, useMemo, useRef } from "react";
import type { MatchSnapshot } from "../types";

interface Props {
  snapshot: MatchSnapshot | null;
  /** Fielder names keyed by player id (from the bowling side's XI). */
  fielderNames: Record<number, string>;
  /** Batting side's names, for the two batters at the crease. */
  batterNames: Record<number, string>;
  /** Show floodlights for day/night cricket. */
  dayNight?: boolean;
}

interface FielderDot {
  id: number;
  angle: number;
  orbit: number;
  speed: number;
}

const INNINGS_BALLS = 6;

export default function MatchView({
  snapshot,
  fielderNames,
  batterNames,
  dayNight = true,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const raf = useRef(0);
  const t = useRef(0);

  const innings = snapshot?.innings[snapshot.current_innings] ?? null;
  const seed = snapshot?.seed ?? 1;

  // Deterministic fielding positions: nine close/catching ring slots.
  const fielders = useMemo<FielderDot[]>(() => {
    let s = (seed ^ 0x51ed) >>> 0 || 1;
    const rand = () => {
      s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
      return s / 4294967296;
    };
    const slots = [
      { angle: -Math.PI * 0.62, orbit: 0.44 }, // slip
      { angle: -Math.PI * 0.5, orbit: 0.3 }, // keeper-ish
      { angle: -Math.PI * 0.25, orbit: 0.52 }, // point
      { angle: -Math.PI * 0.06, orbit: 0.34 }, // cover
      { angle: Math.PI * 0.02, orbit: 0.58 }, // mid-off
      { angle: Math.PI * 0.16, orbit: 0.62 }, // mid-on
      { angle: Math.PI * 0.36, orbit: 0.5 }, // midwicket
      { angle: Math.PI * 0.6, orbit: 0.4 }, // square leg
      { angle: Math.PI * 0.86, orbit: 0.55 }, // fine leg
    ];
    return slots.map((slot, i) => ({
      id: i,
      angle: slot.angle + (rand() - 0.5) * 0.12,
      orbit: slot.orbit,
      speed: 0.02 + rand() * 0.05,
    }));
  }, [seed]);

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
    // without it the whole match screen would unmount. Fall back to the window
    // resize event, which is enough to keep the canvas sized correctly.
    let observer: ResizeObserver | null = null;
    if (typeof ResizeObserver === "function") {
      observer = new ResizeObserver(resize);
      observer.observe(canvas);
    } else {
      window.addEventListener("resize", resize);
    }

    const draw = () => {
      t.current += 1 / 60;
      const time = t.current;
      const w = canvas.clientWidth;
      const h = canvas.clientHeight;
      const cx = w / 2;
      const cy = h / 2;
      const R = Math.min(w, h) * 0.46;

      ctx.clearRect(0, 0, w, h);

      // ---- night sky / stadium bowl ----
      const sky = ctx.createRadialGradient(cx, cy * 0.4, R * 0.2, cx, cy, R * 2.2);
      if (dayNight) {
        sky.addColorStop(0, "#1d4a2e");
        sky.addColorStop(0.55, "#0d2416");
        sky.addColorStop(1, "#050d07");
      } else {
        sky.addColorStop(0, "#2f7a42");
        sky.addColorStop(0.6, "#14401f");
        sky.addColorStop(1, "#08180d");
      }
      ctx.fillStyle = sky;
      ctx.fillRect(0, 0, w, h);

      // ---- stadium bowl: stand tiers with a roof shadow ----
      const stands = dayNight
        ? ["#1b2436", "#141c2b", "#0e1522"]
        : ["#26364c", "#1d2b3d", "#152233"];
      for (let ring = 0; ring < 3; ring++) {
        const rr = R * (1.03 + ring * 0.14);
        ctx.fillStyle = stands[ring];
        ctx.beginPath();
        ctx.ellipse(cx, cy, rr, rr * 0.87, 0, 0, Math.PI * 2);
        ctx.fill();
      }

      // roof lip: a bright rim where the stand overhangs the field
      ctx.strokeStyle = "rgba(255, 255, 255, 0.1)";
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      ctx.ellipse(cx, cy, R * 1.02, R * 0.885, 0, 0, Math.PI * 2);
      ctx.stroke();

      // crowd speckle — deterministic, denser toward the outer tiers
      let s = 20270101;
      const rnd = () => {
        s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
        return s / 4294967296;
      };
      for (let i = 0; i < 1500; i++) {
        const a = rnd() * Math.PI * 2;
        const rr = R * (1.02 + rnd() * 0.42);
        const x = cx + Math.cos(a) * rr;
        const y = cy + Math.sin(a) * rr * 0.87;
        if (x < -4 || x > w + 4 || y < -4 || y > h + 4) continue;
        const shade = 150 + rnd() * 90;
        ctx.fillStyle = `rgba(${shade}, ${shade - 8}, ${shade + 14}, ${0.35 + rnd() * 0.4})`;
        ctx.fillRect(x, y, 1.5, 1.5);
      }

      // ---- outfield ----
      const grad = ctx.createRadialGradient(cx, cy, R * 0.12, cx, cy, R);
      grad.addColorStop(0, "#2f8a44");
      grad.addColorStop(0.7, "#1f6b32");
      grad.addColorStop(1, "#155024");
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.ellipse(cx, cy, R, R * 0.86, 0, 0, Math.PI * 2);
      ctx.fill();

      // mown stripes
      ctx.save();
      ctx.beginPath();
      ctx.ellipse(cx, cy, R, R * 0.86, 0, 0, Math.PI * 2);
      ctx.clip();
      for (let i = -10; i <= 10; i++) {
        if (i % 2 !== 0) continue;
        ctx.fillStyle = "rgba(255, 255, 255, 0.035)";
        ctx.fillRect(cx - R + (i / 10) * R, cy - R, R / 20, R * 2);
      }
      ctx.restore();

      // ---- 30-yard circle ----
      ctx.strokeStyle = "rgba(255, 255, 255, 0.5)";
      ctx.lineWidth = 1.5;
      ctx.setLineDash([5, 5]);
      ctx.beginPath();
      ctx.ellipse(cx, cy, R * 0.62, R * 0.53, 0, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);

      // ---- boundary rope + advertising boards ----
      ctx.strokeStyle = "rgba(255, 255, 255, 0.85)";
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      ctx.ellipse(cx, cy, R * 0.97, R * 0.83, 0, 0, Math.PI * 2);
      ctx.stroke();

      // ad boards around the rope
      const ads = ["SPONSOR", "BOOK NOW", "INSURANCE", "AIRLINE", "BANK", "TYRES"];
      ctx.font = "600 9px 'Segoe UI', Arial, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      for (let i = 0; i < 24; i++) {
        const a = (i / 24) * Math.PI * 2;
        const x = cx + Math.cos(a) * R * 0.99;
        const y = cy + Math.sin(a) * R * 0.85;
        ctx.save();
        ctx.translate(x, y);
        ctx.rotate(a + Math.PI / 2);
        ctx.fillStyle = i % 2 === 0 ? "rgba(12, 30, 50, 0.9)" : "rgba(70, 20, 25, 0.9)";
        ctx.fillRect(-R * 0.075, -6, R * 0.15, 12);
        ctx.fillStyle = "rgba(255, 255, 255, 0.62)";
        ctx.fillText(ads[i % ads.length], 0, 1);
        ctx.restore();
      }

      // ---- pitch ----
      const pitchH = R * 0.42;
      const pitchW = R * 0.085;
      ctx.fillStyle = "#c2a471";
      ctx.beginPath();
      ctx.ellipse(cx, cy, pitchW, pitchH, 0, 0, Math.PI * 2);
      ctx.fill();
      // worn patch
      ctx.fillStyle = "rgba(150, 125, 85, 0.55)";
      ctx.beginPath();
      ctx.ellipse(cx, cy + pitchH * 0.18, pitchW * 0.82, pitchH * 0.34, 0, 0, Math.PI * 2);
      ctx.fill();

      // creases
      ctx.strokeStyle = "rgba(255, 255, 255, 0.9)";
      ctx.lineWidth = 1;
      for (const dy of [-pitchH * 0.3, pitchH * 0.3]) {
        ctx.beginPath();
        ctx.moveTo(cx - pitchW * 0.62, cy + dy);
        ctx.lineTo(cx + pitchW * 0.62, cy + dy);
        ctx.stroke();
      }

      // stumps at both ends
      for (const dy of [-pitchH * 0.31, pitchH * 0.31]) {
        ctx.strokeStyle = "#f2f5f8";
        ctx.lineWidth = 1.4;
        for (const dx of [-2.2, 0, 2.2]) {
          ctx.beginPath();
          ctx.moveTo(cx + dx, cy + dy);
          ctx.lineTo(cx + dx, cy + dy + (dy < 0 ? -7 : 7));
          ctx.stroke();
        }
        // bails
        ctx.beginPath();
        ctx.moveTo(cx - 2.2, cy + dy + (dy < 0 ? -8 : 8));
        ctx.lineTo(cx + 2.2, cy + dy + (dy < 0 ? -8 : 8));
        ctx.stroke();
      }

      // ---- fielders: head + shirt, with the surname above ----
      ctx.font = "600 10px 'Segoe UI', Arial, sans-serif";
      fielders.forEach((f, i) => {
        const drift = Math.sin(time * f.speed * 6 + i) * 0.012;
        const a = f.angle + drift;
        const rr = R * (f.orbit + drift * 0.5);
        const x = cx + Math.cos(a) * rr * 1.05;
        const y = cy + Math.sin(a) * rr * 0.9;

        // shirt
        ctx.fillStyle = "#e8f0f7";
        ctx.beginPath();
        ctx.moveTo(x, y - 1);
        ctx.lineTo(x - 3, y + 4);
        ctx.lineTo(x + 3, y + 4);
        ctx.closePath();
        ctx.fill();
        // head
        ctx.fillStyle = "#f6d9b8";
        ctx.beginPath();
        ctx.arc(x, y - 3.4, 2.1, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = "rgba(0, 0, 0, 0.55)";
        ctx.lineWidth = 0.5;
        ctx.stroke();

        const name = fielderNames[i] ?? "";
        if (name) {
          ctx.fillStyle = "#f2d67a";
          ctx.textAlign = "center";
          ctx.fillText(name, x, y - 8);
        }
      });

      // ---- batters ----
      const strikerId = snapshot?.striker_id ?? null;
      const nonStrikerId = snapshot?.non_striker_id ?? null;
      const batter = (id: number | null, dy: number, colour: string) => {
        if (!id) return;
        const x = cx + 9;
        const y = cy + dy * pitchH * 0.31 + Math.sin(time * 5) * 1.2;
        ctx.fillStyle = colour;
        ctx.beginPath();
        ctx.arc(x, y - 7, 2.8, 0, Math.PI * 2); // head
        ctx.fill();
        ctx.strokeStyle = colour;
        ctx.lineWidth = 2.2;
        ctx.beginPath();
        ctx.moveTo(x, y - 4.5);
        ctx.lineTo(x, y + 2.5);
        ctx.stroke();
        // bat
        ctx.strokeStyle = "#e8d5b0";
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(x - 4, y - 3);
        ctx.lineTo(x - 4, y + 6);
        ctx.stroke();
      };
      batter(strikerId, 1, "#f2c14e");
      batter(nonStrikerId, -1, "#8fd0ff");

      // ---- bowler + ball in flight ----
      const bowlX = cx - 10;
      const bowlY = cy - pitchH * 0.31;
      ctx.fillStyle = "#e05263";
      ctx.beginPath();
      ctx.arc(bowlX, bowlY - 6, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = "#e05263";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(bowlX, bowlY - 3);
      ctx.lineTo(bowlX, bowlY + 4);
      ctx.stroke();

      const phase = (time * 1.1) % 1;
      const bx = bowlX + (cx + 9 - bowlX) * phase;
      const by =
        bowlY + ((cy + pitchH * 0.31) - bowlY) * phase - Math.sin(phase * Math.PI) * 14;
      ctx.fillStyle = "#e04a56";
      ctx.beginPath();
      ctx.arc(bx, by, 2.8, 0, Math.PI * 2);
      ctx.fill();
      // seam
      ctx.strokeStyle = "rgba(255,255,255,0.75)";
      ctx.lineWidth = 0.7;
      ctx.beginPath();
      ctx.arc(bx, by, 2.8, 0.6, 2.4);
      ctx.stroke();

      // ---- keeper ----
      ctx.fillStyle = "#f4f7fa";
      ctx.beginPath();
      ctx.arc(cx + 16, cy - pitchH * 0.3, 2.6, 0, Math.PI * 2);
      ctx.fill();

      // ---- umpire ----
      ctx.fillStyle = "#dfe6ee";
      ctx.beginPath();
      ctx.arc(cx - 16, cy - pitchH * 0.3, 2.6, 0, Math.PI * 2);
      ctx.fill();

      // ---- floodlight pools for day/night ----
      if (dayNight) {
        for (const [lx, ly] of [
          [cx - R * 0.78, cy - R * 0.8],
          [cx + R * 0.78, cy - R * 0.8],
          [cx - R * 0.78, cy + R * 0.72],
          [cx + R * 0.78, cy + R * 0.72],
        ]) {
          // the mast
          ctx.strokeStyle = "rgba(220, 228, 240, 0.35)";
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.moveTo(lx, ly);
          ctx.lineTo(lx, ly - R * 0.1);
          ctx.stroke();
          // the lamp array
          const flare = ctx.createRadialGradient(lx, ly - R * 0.1, 0, lx, ly - R * 0.1, R * 0.42);
          flare.addColorStop(0, "rgba(255, 252, 226, 0.34)");
          flare.addColorStop(1, "rgba(255, 252, 226, 0)");
          ctx.fillStyle = flare;
          ctx.beginPath();
          ctx.arc(lx, ly - R * 0.1, R * 0.42, 0, Math.PI * 2);
          ctx.fill();
        }
      }

      raf.current = requestAnimationFrame(draw);
    };

    raf.current = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf.current);
      if (observer) {
        observer.disconnect();
      } else {
        window.removeEventListener("resize", resize);
      }
    };
  }, [fielders, fielderNames, batterNames, dayNight, innings, snapshot?.striker_id, snapshot?.non_striker_id]);

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

export { INNINGS_BALLS };
