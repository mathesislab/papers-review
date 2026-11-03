// The due date of a project, said plainly (the files panel, the tiles of the project bar), and dates written and
// read in the format of the settings.

import type { DateFormat } from "./types";

/** The days left to a due date (`2026-11-14`); negative once past. */
export const daysLeft = (date: string, now = Date.now()) => Math.round((new Date(date).getTime() - now) / 86_400_000);

/** The due date and the days left to it, said plainly. */
export function dueText(date: string, now = Date.now()): string {
  const days = daysLeft(date, now);
  const when = new Date(date).toLocaleDateString(undefined, { day: "numeric", month: "short" });
  if (days > 1) return `Due ${when} · in ${days} days`;
  if (days === 1) return `Due ${when} · tomorrow`;
  if (days === 0) return `Due ${when} · today`;
  return `Due ${when} · ${-days} days ago`;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** A date (`2026-11-14`) as the settings write it: 2026-11-14, 14.11.2026 or 11/14/2026. */
export function formatDate(date: string, format: DateFormat): string {
  const [y, m, d] = date.split("-");
  if (format === "dmy") return `${d}.${m}.${y}`;
  if (format === "mdy") return `${m}/${d}/${y}`;
  return `${y}-${m}-${d}`;
}

/** A date typed in the format of the settings (any of . / - or spaces between its parts), as `2026-11-14`; null
 *  if it is not a date. */
export function parseDate(text: string, format: DateFormat): string | null {
  const parts = text.trim().split(/[./\-\s]+/);
  if (parts.length !== 3 || !parts.every((p) => /^\d+$/.test(p))) return null;
  const order = { ymd: [0, 1, 2], dmy: [2, 1, 0], mdy: [2, 0, 1] }[format];
  const [y, m, d] = order.map((i) => parts[i]);
  const [year, month, day] = [Number(y), Number(m), Number(d)];
  const date = new Date(Date.UTC(year, month - 1, day));
  if (y.length !== 4 || date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) return null;
  return `${y}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

/** A date in a few letters, for a tile: "14 Nov". */
export function shortDate(date: string): string {
  const [, m, d] = date.split("-").map(Number);
  return `${d} ${MONTHS[m - 1]}`;
}

/** How near a due date is, for the colour of its dot: more than a week, a week or less, three days or less, due
 *  or past. */
export type Urgency = "far" | "near" | "close" | "due";

export function urgency(days: number): Urgency {
  if (days <= 0) return "due";
  if (days <= 3) return "close";
  if (days <= 7) return "near";
  return "far";
}
