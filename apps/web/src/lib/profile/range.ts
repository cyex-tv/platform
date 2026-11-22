// The profile Activity feed's date range (owner rule, 2026/11/22: "either all time or select a
// specific timeframe date range, not day or month"). Only the account's owner and staff get it;
// everyone else reads the newest 100 lines, all time (the API ignores their range anyway).
// A range is two days in the reader's own calendar, both included: `from` (null: from the start)
// and `to` (null: up to now). All time is neither. Presets (last 7, 30, 90 days) are shortcuts
// that fill both. Days are typed and shown in the cyex format (2026/11/19), as cyex logs does; a
// future day is refused. The address keeps it (?from=2026-11-01&to=2026-11-22). Pure, so it is
// tested (range.test.ts).
import { pad } from '$lib/format';

export type DayRange = { from: string | null; to: string | null };

export const ALL_TIME: DayRange = { from: null, to: null };

/** `YYYY-MM-DD` of `d` in the reader's time zone. */
export function dayOf(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Local midnight of a `YYYY-MM-DD`, or null for anything else. */
function midnight(day: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  return dayOf(d) === day ? d : null;
}

/** Why a day can't be used (not a date, in the future); null when it can. */
export function dayProblem(day: string, now = new Date()): string | null {
  if (!midnight(day)) return 'Pick a date.';
  if (day > dayOf(now)) return 'That date is in the future. Pick today or an earlier date.';
  return null;
}

/** `2026/11/19` from a `YYYY-MM-DD`. */
export const slashDay = (day: string) => day.replaceAll('-', '/');

/** What was typed in a date field (`2026/11/19`, `2026-11-19` too): a day, empty (an open end), or why not. */
export function readDay(text: string, now = new Date()): { day: string | null } | { problem: string } {
  if (!text.trim()) return { day: null };
  const m = /^\s*(\d{4})\s*[/.-]\s*(\d{1,2})\s*[/.-]\s*(\d{1,2})\s*$/.exec(text);
  if (!m) return { problem: 'Type a date like 2026/11/19.' };
  const day = `${m[1]}-${pad(Number(m[2]))}-${pad(Number(m[3]))}`;
  const problem = dayProblem(day, now);
  return problem ? { problem: problem === 'Pick a date.' ? 'That isn’t a date. Type one like 2026/11/19.' : problem } : { day };
}

/** Why a range can't be used; null when it can. */
export function rangeProblem(r: DayRange): string | null {
  if (r.from && r.to && r.from > r.to) return 'The start is after the end. Swap them or pick another day.';
  return null;
}

/** The API's `since` (inclusive) and `until` (exclusive), as RFC 3339: `to` counts the whole day. */
export function rangeOf(r: DayRange): { since: string | null; until: string | null } {
  const start = r.from ? midnight(r.from) : null;
  const last = r.to ? midnight(r.to) : null;
  const end = last ? new Date(last.getFullYear(), last.getMonth(), last.getDate() + 1) : null;
  return { since: start?.toISOString() ?? null, until: end?.toISOString() ?? null };
}

export const PRESETS = [
  { days: 7, label: 'Last 7 days' },
  { days: 30, label: 'Last 30 days' },
  { days: 90, label: 'Last 90 days' },
] as const;

/** The last `days` days, today included. */
export function lastDays(days: number, now = new Date()): DayRange {
  const from = new Date(now.getFullYear(), now.getMonth(), now.getDate() - (days - 1));
  return { from: dayOf(from), to: dayOf(now) };
}

/** Which preset a range is (its days), 0 for all time, null for any other range. */
export function presetOf(r: DayRange, now = new Date()): number | null {
  if (!r.from && !r.to) return 0;
  return PRESETS.find((p) => {
    const want = lastDays(p.days, now);
    return want.from === r.from && want.to === r.to;
  })?.days ?? null;
}

/** The range in cyex format: `2026/11/01 – 2026/11/22`, `From 2026/11/01`, `Until …`, `All time`. */
export function labelOf(r: DayRange): string {
  if (r.from && r.to) return r.from === r.to ? slashDay(r.from) : `${slashDay(r.from)} – ${slashDay(r.to)}`;
  if (r.from) return `From ${slashDay(r.from)}`;
  if (r.to) return `Until ${slashDay(r.to)}`;
  return 'All time';
}

/** The range in the address: `from=2026-11-01`, `to=2026-11-22`; nothing for all time. */
export function toQuery(r: DayRange): Record<string, string> {
  const out: Record<string, string> = {};
  if (r.from) out.from = r.from;
  if (r.to) out.to = r.to;
  return out;
}

/** Reads `toQuery` back; a broken or future day is dropped, a backwards range is all time. */
export function fromQuery(q: URLSearchParams, now = new Date()): DayRange {
  const read = (k: string) => {
    const v = q.get(k);
    return v && !dayProblem(v, now) ? v : null;
  };
  const r = { from: read('from'), to: read('to') };
  return rangeProblem(r) ? ALL_TIME : r;
}
