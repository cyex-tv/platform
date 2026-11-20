// The activity feed's timeframe, the way cyex logs does it (logs-frontend, the "Date" stat and its
// jump popover): the feed starts on everything recent, newest first; a day or a month narrows it,
// picked in a small popover ("Jump to date" with a date field, "Jump to month" with a year and a
// month), and a future date is refused with a line saying so. The label is the cyex date format:
// 2026/11/19 for a day, 2026/11 for a month; All time has no date to show. Days and months are the reader's own calendar.
// Pure, so it is tested (timeframe.test.ts).
import { pad } from '$lib/format';

export type Timeframe =
  | { frame: 'all' }
  | { frame: 'day'; day: string }
  | { frame: 'month'; year: number; month: number };

export type Frame = Timeframe['frame'];



/** `YYYY-MM-DD` of `d` in the reader's time zone (what `<input type="date">` takes). */
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

/** Switching the frame: a day starts on today, a month on this month (logs' defaults). */
export function frameTo(frame: Frame, now = new Date()): Timeframe {
  if (frame === 'day') return { frame, day: dayOf(now) };
  if (frame === 'month') return { frame, year: now.getFullYear(), month: now.getMonth() + 1 };
  return { frame: 'all' };
}

/** The API's `since` (inclusive) and `until` (exclusive) for a timeframe, as RFC 3339. */
export function rangeOf(t: Timeframe): { since: string | null; until: string | null } {
  if (t.frame === 'day') {
    const start = midnight(t.day);
    if (!start) return { since: null, until: null };
    const end = new Date(start.getFullYear(), start.getMonth(), start.getDate() + 1);
    return { since: start.toISOString(), until: end.toISOString() };
  }
  if (t.frame === 'month') {
    const start = new Date(t.year, t.month - 1, 1);
    const end = new Date(t.year, t.month, 1);
    return { since: start.toISOString(), until: end.toISOString() };
  }
  return { since: null, until: null };
}

/** Why a picked day can't be used (empty, not a date, in the future); null when it can. */
export function dayProblem(day: string, now = new Date()): string | null {
  const d = midnight(day);
  if (!d) return 'Pick a date.';
  if (day > dayOf(now)) return 'That date is in the future. Pick today or an earlier date.';
  return null;
}

/** Why a picked month can't be used; null when it can. */
export function monthProblem(year: number, month: number, now = new Date()): string | null {
  if (!Number.isInteger(year) || year < 2000 || !Number.isInteger(month) || month < 1 || month > 12) return 'Pick a year and a month.';
  const y = now.getFullYear();
  if (year > y || (year === y && month > now.getMonth() + 1)) return 'That month is in the future. Pick this month or an earlier one.';
  return null;
}

/** `2026/11/19` from a `YYYY-MM-DD`. */
export const slashDay = (day: string) => day.replaceAll('-', '/');

/** The timeframe's label in cyex format: a day `2026/11/19`, a month `2026/11`, "All time". */
export function labelOf(t: Timeframe): string {
  if (t.frame === 'day') return slashDay(t.day);
  if (t.frame === 'month') return `${t.year}/${pad(t.month)}`;
  return 'All time';
}

/** What was typed in the date field (`2026/11/19`, `2026-11-19` too), as a day, or the line saying why not. */
export function readDay(text: string, now = new Date()): { day: string } | { problem: string } {
  const m = /^\s*(\d{4})\s*[/.-]\s*(\d{1,2})\s*[/.-]\s*(\d{1,2})\s*$/.exec(text);
  if (!m) return { problem: 'Type a date like 2026/11/19.' };
  const day = `${m[1]}-${pad(Number(m[2]))}-${pad(Number(m[3]))}`;
  const problem = dayProblem(day, now);
  return problem ? { problem: problem === 'Pick a date.' ? 'That isn’t a date. Type one like 2026/11/19.' : problem } : { day };
}

/** The timeframe in the address: `d=2026-11-19` or `m=2026-11`; nothing for all time. */
export function toQuery(t: Timeframe): Record<string, string> {
  if (t.frame === 'day') return { d: t.day };
  if (t.frame === 'month') return { m: `${t.year}-${pad(t.month)}` };
  return {};
}

/** Reads `toQuery` back; anything unknown, broken or in the future is all time. */
export function fromQuery(q: URLSearchParams, now = new Date()): Timeframe {
  const d = q.get('d');
  if (d && !dayProblem(d, now)) return { frame: 'day', day: d };
  const m = /^(\d{4})-(\d{2})$/.exec(q.get('m') ?? '');
  if (m && !monthProblem(Number(m[1]), Number(m[2]), now)) return { frame: 'month', year: Number(m[1]), month: Number(m[2]) };
  return { frame: 'all' };
}
