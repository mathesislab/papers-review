// The due date of a project, said plainly (the files panel, the tiles of the project bar).

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
