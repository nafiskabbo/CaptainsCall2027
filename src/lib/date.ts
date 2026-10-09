//! Season calendar helpers: the career tracks a day counter, the UI shows dates.

/** The fictional season starts on 1 April 2027 (domestic season kickoff). */
export const SEASON_START = { year: 2027, month: 4, day: 1 };

const MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

/**
 * Format a season day as `09 May 2027`.
 * `seasonDay` is 1-based.
 */
export function seasonDate(seasonDay: number): string {
  const base = Date.UTC(SEASON_START.year, SEASON_START.month - 1, SEASON_START.day);
  const date = new Date(base + (Math.max(1, seasonDay) - 1) * 86_400_000);
  const dd = String(date.getUTCDate()).padStart(2, "0");
  return `${dd} ${MONTHS[date.getUTCMonth()]} ${date.getUTCFullYear()}`;
}

/** A plausible clock for the given day, so the top bar reads like the original. */
export function seasonClock(seasonDay: number): string {
  const hour = 8 + (seasonDay % 12);
  const minute = (seasonDay * 17) % 60;
  return `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
}

/** Match-day label used in the match title bar. */
export function matchDayLabel(day: number, totalDays: number): string {
  if (totalDays <= 1) return "";
  const suffix = ordinal(day);
  return `${suffix} Day of ${totalDays}`;
}

function ordinal(n: number): string {
  const s = ["th", "st", "nd", "rd"];
  const v = n % 100;
  return n + (s[(v - 20) % 10] ?? s[v] ?? s[0]);
}
